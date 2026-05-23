use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::cli::parse_request_line;
use crate::commands::CXN0102Notify;
use crate::cxn0102::CXN0102;

const MAX_BODY_SIZE: usize = 1024 * 1024;
const MAX_FILTERED_NOTIFY_READS: usize = 8;
const INDEX_HTML: &str = include_str!("../static/index.html");

type SharedDevice = Arc<Mutex<CXN0102>>;

pub fn run(cxn0102: CXN0102, listen_addr: &str) -> io::Result<()> {
    let listener = TcpListener::bind(listen_addr)?;
    let cxn0102 = Arc::new(Mutex::new(cxn0102));
    println!("backend listening on http://{listen_addr}");

    for stream in listener.incoming() {
        let stream = stream?;
        let cxn0102 = Arc::clone(&cxn0102);
        thread::spawn(move || {
            if let Err(error) = handle_connection(stream, cxn0102) {
                eprintln!("backend connection error: {error}");
            }
        });
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream, cxn0102: SharedDevice) -> io::Result<()> {
    let request = match HttpRequest::read(&mut stream) {
        Ok(request) => request,
        Err(error) if error.kind() == io::ErrorKind::InvalidData => {
            return write_response(&mut stream, 400, json_error(&error.to_string()));
        }
        Err(error) => return Err(error),
    };

    let response = route_request(request, cxn0102);
    write_response_with_type(
        &mut stream,
        response.status,
        response.content_type,
        response.body,
    )
}

fn route_request(request: HttpRequest, cxn0102: SharedDevice) -> HttpResponse {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => HttpResponse::html(200, INDEX_HTML.to_owned()),
        ("GET", "/health") => HttpResponse::json(200, r#"{"status":"ok"}"#.to_owned()),
        ("GET", "/api/temperature") => read_temperature(cxn0102),
        ("GET", "/api/version") => read_version(cxn0102),
        ("POST", "/command") | ("POST", "/api/command") => process_command(request, cxn0102),
        ("GET", "/command") | ("GET", "/api/command") => {
            HttpResponse::json(405, json_error("use POST for command requests"))
        }
        _ => HttpResponse::json(404, json_error("not found")),
    }
}

fn process_command(request: HttpRequest, cxn0102: SharedDevice) -> HttpResponse {
    let body = String::from_utf8(request.body)
        .map_err(|error| format!("request body must be UTF-8: {error}"))
        .and_then(|body| command_line_from_body(&body));

    let line = match body {
        Ok(line) => line,
        Err(error) => return HttpResponse::json(400, json_error(&error)),
    };

    let bytes = match parse_request_line(&line) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return HttpResponse::json(400, json_error("request did not produce a command"));
        }
        Err(error) => return HttpResponse::json(400, json_error(&error)),
    };

    let cxn0102 = match cxn0102.lock() {
        Ok(cxn0102) => cxn0102,
        Err(_) => return HttpResponse::json(500, json_error("device lock is poisoned")),
    };

    if let Err(error) = cxn0102.write(&bytes) {
        return HttpResponse::json(500, json_error(&error.to_string()));
    }

    let expected_command_id = bytes[0];
    let reply = match read_matching_notify(&cxn0102, |notify| {
        notify_command_id(notify) == expected_command_id
    }) {
        Ok(notify) => notify,
        Err(error) => return HttpResponse::json(500, json_error(&error.to_string())),
    };

    HttpResponse::json(
        200,
        format!(
            r#"{{"ok":true,"command":"{}","request_bytes":"{}","reply":{}}}"#,
            json_escape(&line),
            format_hex(&bytes),
            notify_json(&reply)
        ),
    )
}

fn read_temperature(cxn0102: SharedDevice) -> HttpResponse {
    let cxn0102 = match cxn0102.lock() {
        Ok(cxn0102) => cxn0102,
        Err(_) => return HttpResponse::json(500, json_error("device lock is poisoned")),
    };

    if let Err(error) = cxn0102.write(&[0xA0, 0x00]) {
        return HttpResponse::json(500, json_error(&error.to_string()));
    }

    match read_matching_notify(&cxn0102, |notify| {
        matches!(notify, CXN0102Notify::GetTemperature(_))
    }) {
        Ok(CXN0102Notify::GetTemperature(notify)) => HttpResponse::json(
            200,
            format!(
                r#"{{"ok":true,"result":"{:?}","module_temperature":{},"mute_threshold_temperature":{},"system_stop_threshold_temperature":{}}}"#,
                notify.result,
                notify.module_temperature,
                notify.mute_threshold_temperature,
                notify.system_stop_threshold_temperature
            ),
        ),
        Ok(notify) => HttpResponse::json(500, json_error(&format!("unexpected notify {notify:?}"))),
        Err(error) => HttpResponse::json(500, json_error(&error.to_string())),
    }
}

fn read_version(cxn0102: SharedDevice) -> HttpResponse {
    let cxn0102 = match cxn0102.lock() {
        Ok(cxn0102) => cxn0102,
        Err(_) => return HttpResponse::json(500, json_error("device lock is poisoned")),
    };

    if let Err(error) = cxn0102.write(&[0xA2, 0x00]) {
        return HttpResponse::json(500, json_error(&error.to_string()));
    }

    match read_matching_notify(&cxn0102, |notify| {
        matches!(notify, CXN0102Notify::GetVersion(_))
    }) {
        Ok(CXN0102Notify::GetVersion(notify)) => HttpResponse::json(
            200,
            format!(
                r#"{{"ok":true,"result":"{:?}","firmware":"{}","parameter":"{}","data":"{}","firmware_bytes":[{}],"parameter_bytes":[{}],"data_bytes":[{}]}}"#,
                notify.result,
                format_version(notify.firmware),
                format_version(notify.parameter),
                format_version(notify.data),
                format_byte_array(notify.firmware),
                format_byte_array(notify.parameter),
                format_byte_array(notify.data)
            ),
        ),
        Ok(notify) => HttpResponse::json(500, json_error(&format!("unexpected notify {notify:?}"))),
        Err(error) => HttpResponse::json(500, json_error(&error.to_string())),
    }
}

fn read_matching_notify(
    cxn0102: &CXN0102,
    matches: impl Fn(&CXN0102Notify) -> bool,
) -> io::Result<CXN0102Notify> {
    for _ in 0..MAX_FILTERED_NOTIFY_READS {
        let notify = cxn0102.read_notify()?;
        if matches(&notify) {
            return Ok(notify);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "expected notify was not received",
    ))
}

fn notify_command_id(notify: &CXN0102Notify) -> u8 {
    match notify {
        CXN0102Notify::BootCompleted(_) => 0x00,
        CXN0102Notify::StartInput(_) => 0x01,
        CXN0102Notify::StopInput(_) => 0x02,
        CXN0102Notify::MuteUnmuteChangeOutput(_) => 0x03,
        CXN0102Notify::SaveUserParam(_) => 0x07,
        CXN0102Notify::InitializeUserParam(_) => 0x08,
        CXN0102Notify::ShutdownReboot(_) => 0x0B,
        CXN0102Notify::StopInputSpecially(_) => 0x0C,
        CXN0102Notify::Emergency(_) => 0x10,
        CXN0102Notify::TemperatureEmergencyAndRecovery(_) => 0x11,
        CXN0102Notify::CommandEmergency(_) => 0x12,
        CXN0102Notify::GetVideoOutputPosition(_) => 0x25,
        CXN0102Notify::SetVideoOutputPosition(_) => 0x26,
        CXN0102Notify::GetOpticalAlignment(_) => 0x27,
        CXN0102Notify::SetOpticalAlignment(_) => 0x28,
        CXN0102Notify::GetBiphase(_) => 0x29,
        CXN0102Notify::SetBiphase(_) => 0x2A,
        CXN0102Notify::EasyOpticalAdjustmentControl(_) => 0x32,
        CXN0102Notify::EasyOpticalAdjustmentPlus(_) => 0x33,
        CXN0102Notify::EasyOpticalAdjustmentMinus(_) => 0x34,
        CXN0102Notify::EasyOpticalAdjustmentExit(_) => 0x35,
        CXN0102Notify::EasyBiphaseAdjustmentControl(_) => 0x36,
        CXN0102Notify::EasyBiphaseAdjustmentPlus(_) => 0x37,
        CXN0102Notify::EasyBiphaseAdjustmentMinus(_) => 0x38,
        CXN0102Notify::EasyBiphaseAdjustmentExit(_) => 0x39,
        CXN0102Notify::GetAllPictureQuality(_) => 0x40,
        CXN0102Notify::SetAllPictureQuality(_) => 0x41,
        CXN0102Notify::GetBrightness(_) => 0x42,
        CXN0102Notify::SetBrightness(_) => 0x43,
        CXN0102Notify::GetContrast(_) => 0x44,
        CXN0102Notify::SetContrast(_) => 0x45,
        CXN0102Notify::GetHue(_) => 0x46,
        CXN0102Notify::SetHue(_) => 0x47,
        CXN0102Notify::GetSaturation(_) => 0x48,
        CXN0102Notify::SetSaturation(_) => 0x49,
        CXN0102Notify::GetSharpness(_) => 0x4E,
        CXN0102Notify::SetSharpness(_) => 0x4F,
        CXN0102Notify::UpdateFwImage(_) => 0x82,
        CXN0102Notify::UpdatePictureData(_) => 0x84,
        CXN0102Notify::DivisionTransmissionUpdateFwImage(_) => 0x92,
        CXN0102Notify::DivisionTransmissionUpdatePictureData(_) => 0x94,
        CXN0102Notify::GetTemperature(_) => 0xA0,
        CXN0102Notify::GetTime(_) => 0xA1,
        CXN0102Notify::GetVersion(_) => 0xA2,
        CXN0102Notify::OutputTestPicture(_) => 0xA3,
        CXN0102Notify::GetLotNumber(_) => 0xB2,
        CXN0102Notify::GetSerialNumber(_) => 0xB4,
    }
}

fn notify_json(notify: &CXN0102Notify) -> String {
    match notify {
        CXN0102Notify::BootCompleted(notify) => object_json(
            "boot-completed",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::StartInput(notify) => command_result_json("start-input", notify.result),
        CXN0102Notify::StopInput(notify) => command_result_json("stop-input", notify.result),
        CXN0102Notify::StopInputSpecially(notify) => {
            command_result_json("stop-input-specially", notify.result)
        }
        CXN0102Notify::MuteUnmuteChangeOutput(notify) => {
            command_result_json("mute-unmute-change-output", notify.result)
        }
        CXN0102Notify::SaveUserParam(notify) => {
            command_result_json("save-user-param", notify.result)
        }
        CXN0102Notify::InitializeUserParam(notify) => {
            command_result_json("initialize-user-param", notify.result)
        }
        CXN0102Notify::ShutdownReboot(notify) => {
            command_result_json("shutdown-reboot", notify.result)
        }
        CXN0102Notify::GetVideoOutputPosition(notify) => object_json(
            "get-video-output-position",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("pan", notify.pan.0.to_string()),
                field_json("tilt", notify.tilt.0.to_string()),
                field_json("flip", result_json(notify.flip)),
            ],
        ),
        CXN0102Notify::SetVideoOutputPosition(notify) => {
            error_result_json("set-video-output-position", notify.result)
        }
        CXN0102Notify::GetOpticalAlignment(notify) => object_json(
            "get-optical-alignment",
            vec![
                field_json("result", result_json(notify.result)),
                field_json(
                    "alignment",
                    format!(
                        r#"{{"r0_horizontal":{},"r1_horizontal":{},"g0_horizontal":{},"g1_horizontal":{},"b_horizontal":{},"r0_vertical":{},"r1_vertical":{},"g0_vertical":{},"g1_vertical":{},"b_vertical":{}}}"#,
                        notify.alignment.r0_horizontal.0,
                        notify.alignment.r1_horizontal.0,
                        notify.alignment.g0_horizontal.0,
                        notify.alignment.g1_horizontal.0,
                        notify.alignment.b_horizontal.0,
                        notify.alignment.r0_vertical,
                        notify.alignment.r1_vertical,
                        notify.alignment.g0_vertical,
                        notify.alignment.g1_vertical,
                        notify.alignment.b_vertical
                    ),
                ),
            ],
        ),
        CXN0102Notify::SetOpticalAlignment(notify) => {
            error_result_json("set-optical-alignment", notify.result)
        }
        CXN0102Notify::GetBiphase(notify) => object_json(
            "get-biphase",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("amount", notify.amount.to_string()),
            ],
        ),
        CXN0102Notify::SetBiphase(notify) => error_result_json("set-biphase", notify.result),
        CXN0102Notify::EasyOpticalAdjustmentControl(notify) => object_json(
            "set-easy-optical-adjustment-control",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::EasyOpticalAdjustmentPlus(notify) => object_json(
            "set-easy-optical-adjustment-plus",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::EasyOpticalAdjustmentMinus(notify) => object_json(
            "set-easy-optical-adjustment-minus",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::EasyOpticalAdjustmentExit(notify) => {
            command_result_json("set-easy-optical-adjustment-exit", notify.result)
        }
        CXN0102Notify::EasyBiphaseAdjustmentControl(notify) => object_json(
            "set-easy-biphase-adjustment-control",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::EasyBiphaseAdjustmentPlus(notify) => object_json(
            "set-easy-biphase-adjustment-plus",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::EasyBiphaseAdjustmentMinus(notify) => object_json(
            "set-easy-biphase-adjustment-minus",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::EasyBiphaseAdjustmentExit(notify) => {
            command_result_json("set-easy-biphase-adjustment-exit", notify.result)
        }
        CXN0102Notify::GetAllPictureQuality(notify) => object_json(
            "get-all-picture-quality",
            vec![
                field_json("result", result_json(notify.result)),
                field_json(
                    "picture_quality",
                    format!(
                        r#"{{"contrast":{},"brightness":{},"hue_u":{},"hue_v":{},"saturation_u":{},"saturation_v":{},"sharpness":{}}}"#,
                        notify.picture_quality.contrast,
                        notify.picture_quality.brightness,
                        notify.picture_quality.hue_u,
                        notify.picture_quality.hue_v,
                        notify.picture_quality.saturation_u,
                        notify.picture_quality.saturation_v,
                        notify.picture_quality.sharpness
                    ),
                ),
            ],
        ),
        CXN0102Notify::SetAllPictureQuality(notify) => {
            error_result_json("set-all-picture-quality", notify.result)
        }
        CXN0102Notify::GetBrightness(notify) => scalar_json(
            "get-brightness",
            result_json(notify.result),
            "brightness",
            notify.brightness.to_string(),
        ),
        CXN0102Notify::SetBrightness(notify) => error_result_json("set-brightness", notify.result),
        CXN0102Notify::GetContrast(notify) => scalar_json(
            "get-contrast",
            result_json(notify.result),
            "contrast",
            notify.contrast.to_string(),
        ),
        CXN0102Notify::SetContrast(notify) => error_result_json("set-contrast", notify.result),
        CXN0102Notify::GetHue(notify) => object_json(
            "get-hue",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("hue_u", notify.hue_u.to_string()),
                field_json("hue_v", notify.hue_v.to_string()),
            ],
        ),
        CXN0102Notify::SetHue(notify) => error_result_json("set-hue", notify.result),
        CXN0102Notify::GetSaturation(notify) => object_json(
            "get-saturation",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("saturation_u", notify.saturation_u.to_string()),
                field_json("saturation_v", notify.saturation_v.to_string()),
            ],
        ),
        CXN0102Notify::SetSaturation(notify) => error_result_json("set-saturation", notify.result),
        CXN0102Notify::GetSharpness(notify) => scalar_json(
            "get-sharpness",
            result_json(notify.result),
            "sharpness",
            notify.sharpness.to_string(),
        ),
        CXN0102Notify::SetSharpness(notify) => error_result_json("set-sharpness", notify.result),
        CXN0102Notify::UpdateFwImage(notify) => {
            update_result_json("update-fw-image", notify.result)
        }
        CXN0102Notify::UpdatePictureData(notify) => {
            update_result_json("update-picture-data", notify.result)
        }
        CXN0102Notify::DivisionTransmissionUpdateFwImage(notify) => {
            update_result_json("division-transmission-update-fw-image", notify.result)
        }
        CXN0102Notify::DivisionTransmissionUpdatePictureData(notify) => {
            update_result_json("division-transmission-update-picture-data", notify.result)
        }
        CXN0102Notify::GetTemperature(notify) => object_json(
            "get-temperature",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("module_temperature", notify.module_temperature.to_string()),
                field_json(
                    "mute_threshold_temperature",
                    notify.mute_threshold_temperature.to_string(),
                ),
                field_json(
                    "system_stop_threshold_temperature",
                    notify.system_stop_threshold_temperature.to_string(),
                ),
            ],
        ),
        CXN0102Notify::GetTime(notify) => object_json(
            "get-time",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("seconds", notify.seconds.to_string()),
            ],
        ),
        CXN0102Notify::GetVersion(notify) => object_json(
            "get-version",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("firmware", string_json(&format_version(notify.firmware))),
                field_json("parameter", string_json(&format_version(notify.parameter))),
                field_json("data", string_json(&format_version(notify.data))),
                field_json("firmware_bytes", u8_array_json(&notify.firmware)),
                field_json("parameter_bytes", u8_array_json(&notify.parameter)),
                field_json("data_bytes", u8_array_json(&notify.data)),
            ],
        ),
        CXN0102Notify::OutputTestPicture(notify) => {
            error_result_json("output-test-picture", notify.result)
        }
        CXN0102Notify::GetLotNumber(notify) => object_json(
            "get-lot-number",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("lot_numbers", u32_array_json(&notify.lot_numbers)),
            ],
        ),
        CXN0102Notify::GetSerialNumber(notify) => object_json(
            "get-serial-number",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("serial_numbers", u32_array_json(&notify.serial_numbers)),
            ],
        ),
        CXN0102Notify::Emergency(notify) => object_json(
            "emergency",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::TemperatureEmergencyAndRecovery(notify) => object_json(
            "temperature-emergency-and-recovery",
            vec![field_json("result", result_json(notify.result))],
        ),
        CXN0102Notify::CommandEmergency(notify) => object_json(
            "command-emergency",
            vec![
                field_json("result", result_json(notify.result)),
                field_json("reference", notify.reference.to_string()),
            ],
        ),
    }
}

fn command_result_json(command: &str, result: impl std::fmt::Debug) -> String {
    object_json(command, vec![field_json("result", result_json(result))])
}

fn error_result_json(command: &str, result: u8) -> String {
    object_json(command, vec![field_json("result", result.to_string())])
}

fn update_result_json(command: &str, result: impl std::fmt::Debug) -> String {
    object_json(command, vec![field_json("result", result_json(result))])
}

fn scalar_json(command: &str, result: String, field: &str, value: String) -> String {
    object_json(
        command,
        vec![field_json("result", result), field_json(field, value)],
    )
}

fn object_json(command: &str, mut fields: Vec<String>) -> String {
    let mut all_fields = vec![
        field_json(
            "command_id",
            notify_command_id_from_name(command).to_string(),
        ),
        field_json("type", string_json(command)),
    ];
    all_fields.append(&mut fields);
    format!("{{{}}}", all_fields.join(","))
}

fn notify_command_id_from_name(command: &str) -> u8 {
    match command {
        "boot-completed" => 0x00,
        "start-input" => 0x01,
        "stop-input" => 0x02,
        "mute-unmute-change-output" => 0x03,
        "save-user-param" => 0x07,
        "initialize-user-param" => 0x08,
        "shutdown-reboot" => 0x0B,
        "stop-input-specially" => 0x0C,
        "emergency" => 0x10,
        "temperature-emergency-and-recovery" => 0x11,
        "command-emergency" => 0x12,
        "get-video-output-position" => 0x25,
        "set-video-output-position" => 0x26,
        "get-optical-alignment" => 0x27,
        "set-optical-alignment" => 0x28,
        "get-biphase" => 0x29,
        "set-biphase" => 0x2A,
        "set-easy-optical-adjustment-control" => 0x32,
        "set-easy-optical-adjustment-plus" => 0x33,
        "set-easy-optical-adjustment-minus" => 0x34,
        "set-easy-optical-adjustment-exit" => 0x35,
        "set-easy-biphase-adjustment-control" => 0x36,
        "set-easy-biphase-adjustment-plus" => 0x37,
        "set-easy-biphase-adjustment-minus" => 0x38,
        "set-easy-biphase-adjustment-exit" => 0x39,
        "get-all-picture-quality" => 0x40,
        "set-all-picture-quality" => 0x41,
        "get-brightness" => 0x42,
        "set-brightness" => 0x43,
        "get-contrast" => 0x44,
        "set-contrast" => 0x45,
        "get-hue" => 0x46,
        "set-hue" => 0x47,
        "get-saturation" => 0x48,
        "set-saturation" => 0x49,
        "get-sharpness" => 0x4E,
        "set-sharpness" => 0x4F,
        "update-fw-image" => 0x82,
        "update-picture-data" => 0x84,
        "division-transmission-update-fw-image" => 0x92,
        "division-transmission-update-picture-data" => 0x94,
        "get-temperature" => 0xA0,
        "get-time" => 0xA1,
        "get-version" => 0xA2,
        "output-test-picture" => 0xA3,
        "get-lot-number" => 0xB2,
        "get-serial-number" => 0xB4,
        _ => 0,
    }
}

fn field_json(name: &str, value: String) -> String {
    format!(r#""{}":{}"#, json_escape(name), value)
}

fn result_json(result: impl std::fmt::Debug) -> String {
    string_json(&format!("{result:?}"))
}

fn string_json(value: &str) -> String {
    format!(r#""{}""#, json_escape(value))
}

fn u8_array_json<const N: usize>(bytes: &[u8; N]) -> String {
    format!(
        "[{}]",
        bytes
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn u32_array_json<const N: usize>(values: &[u32; N]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn command_line_from_body(body: &str) -> Result<String, String> {
    let body = body.trim();
    if body.is_empty() {
        return Err("request body is empty".to_owned());
    }

    if body.starts_with('{') {
        return command_line_from_json(body);
    }

    Ok(body.to_owned())
}

fn command_line_from_json(body: &str) -> Result<String, String> {
    let mut parser = JsonParser::new(body);
    let JsonValue::Object(mut object) = parser.parse()? else {
        return Err("JSON request body must be an object".to_owned());
    };
    parser.finish()?;

    if let Some(JsonValue::String(line)) = object.remove("line") {
        return Ok(line);
    }

    let command = match object.remove("command") {
        Some(JsonValue::String(command)) if !command.trim().is_empty() => command,
        Some(_) => return Err("JSON field 'command' must be a non-empty string".to_owned()),
        None => return Err("JSON request needs either 'line' or 'command'".to_owned()),
    };

    let mut line = command;
    match object.remove("args") {
        Some(JsonValue::Object(args)) => {
            for (key, value) in args {
                line.push_str(" --");
                line.push_str(&key);
                line.push(' ');
                line.push_str(&json_arg_value(value)?);
            }
        }
        Some(_) => return Err("JSON field 'args' must be an object".to_owned()),
        None => {}
    }

    Ok(line)
}

fn json_arg_value(value: JsonValue) -> Result<String, String> {
    match value {
        JsonValue::String(value) => Ok(value),
        JsonValue::Number(value) => Ok(value),
        JsonValue::Bool(value) => Ok(value.to_string()),
        JsonValue::Null => Err("argument values cannot be null".to_owned()),
        JsonValue::Object(_) => Err("nested argument objects are not supported".to_owned()),
    }
}

struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

impl HttpRequest {
    fn read(stream: &mut TcpStream) -> io::Result<Self> {
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "empty HTTP request",
            ));
        }

        let mut parts = request_line.split_whitespace();
        let method = parts
            .next()
            .ok_or_else(|| invalid_request("missing HTTP method"))?
            .to_owned();
        let target = parts
            .next()
            .ok_or_else(|| invalid_request("missing HTTP target"))?;
        let path = target.split('?').next().unwrap_or(target).to_owned();

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                return Err(invalid_request("HTTP headers ended unexpectedly"));
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }

            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| invalid_request("invalid Content-Length"))?;
                    if content_length > MAX_BODY_SIZE {
                        return Err(invalid_request("request body is too large"));
                    }
                }
            }
        }

        let mut body = vec![0; content_length];
        reader.read_exact(&mut body)?;

        Ok(Self { method, path, body })
    }
}

fn invalid_request(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

struct HttpResponse {
    status: u16,
    body: String,
    content_type: &'static str,
}

impl HttpResponse {
    fn json(status: u16, body: String) -> Self {
        Self {
            status,
            body,
            content_type: "application/json",
        }
    }

    fn html(status: u16, body: String) -> Self {
        Self {
            status,
            body,
            content_type: "text/html; charset=utf-8",
        }
    }
}

fn write_response(stream: &mut TcpStream, status: u16, body: String) -> io::Result<()> {
    write_response_with_type(stream, status, "application/json", body)
}

fn write_response_with_type(
    stream: &mut TcpStream,
    status: u16,
    content_type: &'static str,
    body: String,
) -> io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "OK",
    };

    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    )
}

fn format_version(bytes: [u8; 4]) -> String {
    bytes
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

fn format_byte_array(bytes: [u8; 4]) -> String {
    bytes
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn json_error(message: &str) -> String {
    format!(r#"{{"ok":false,"error":"{}"}}"#, json_escape(message))
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for c in value.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonValue {
    Object(BTreeMap<String, JsonValue>),
    String(String),
    Number(String),
    Bool(bool),
    Null,
}

struct JsonParser<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, index: 0 }
    }

    fn parse(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('t') => self.parse_literal("true", JsonValue::Bool(true)),
            Some('f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some('n') => self.parse_literal("null", JsonValue::Null),
            Some('-' | '0'..='9') => self.parse_number().map(JsonValue::Number),
            Some(c) => Err(format!("unexpected JSON character '{c}'")),
            None => Err("unexpected end of JSON".to_owned()),
        }
    }

    fn finish(&mut self) -> Result<(), String> {
        self.skip_whitespace();
        if self.index == self.input.len() {
            Ok(())
        } else {
            Err("unexpected data after JSON object".to_owned())
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.expect('{')?;
        let mut object = BTreeMap::new();
        self.skip_whitespace();
        if self.consume('}') {
            return Ok(JsonValue::Object(object));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.parse()?;
            object.insert(key, value);
            self.skip_whitespace();

            if self.consume('}') {
                break;
            }
            self.expect(',')?;
        }

        Ok(JsonValue::Object(object))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut value = String::new();

        while let Some(c) = self.next() {
            match c {
                '"' => return Ok(value),
                '\\' => value.push(self.parse_escape()?),
                c if c.is_control() => return Err("control character in JSON string".to_owned()),
                c => value.push(c),
            }
        }

        Err("unterminated JSON string".to_owned())
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        match self.next() {
            Some('"') => Ok('"'),
            Some('\\') => Ok('\\'),
            Some('/') => Ok('/'),
            Some('b') => Ok('\u{0008}'),
            Some('f') => Ok('\u{000c}'),
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('u') => {
                let mut code = 0u32;
                for _ in 0..4 {
                    let Some(c) = self.next() else {
                        return Err("incomplete JSON unicode escape".to_owned());
                    };
                    code = code
                        .checked_mul(16)
                        .and_then(|code| c.to_digit(16).map(|digit| code + digit))
                        .ok_or_else(|| "invalid JSON unicode escape".to_owned())?;
                }
                char::from_u32(code).ok_or_else(|| "invalid JSON unicode scalar".to_owned())
            }
            Some(c) => Err(format!("invalid JSON escape '\\{c}'")),
            None => Err("incomplete JSON escape".to_owned()),
        }
    }

    fn parse_number(&mut self) -> Result<String, String> {
        let start = self.index;
        self.consume('-');
        self.consume_digits();
        if self.consume('.') {
            self.consume_digits();
        }
        if self.consume('e') || self.consume('E') {
            let _ = self.consume('+') || self.consume('-');
            self.consume_digits();
        }

        if self.index == start {
            Err("invalid JSON number".to_owned())
        } else {
            Ok(self.input[start..self.index].to_owned())
        }
    }

    fn parse_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue, String> {
        if self.input[self.index..].starts_with(literal) {
            self.index += literal.len();
            Ok(value)
        } else {
            Err(format!("expected JSON literal '{literal}'"))
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\r' | '\t')) {
            self.next();
        }
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek(), Some('0'..='9')) {
            self.next();
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.next() {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(format!("expected '{expected}', got '{actual}'")),
            None => Err(format!("expected '{expected}', got end of JSON")),
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.next();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.index..].chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.index += c.len_utf8();
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_cli_line() {
        assert_eq!(
            command_line_from_body("set-brightness --brightness -5").unwrap(),
            "set-brightness --brightness -5"
        );
    }

    #[test]
    fn accepts_json_line() {
        assert_eq!(
            command_line_from_body(r#"{"line":"get-version"}"#).unwrap(),
            "get-version"
        );
    }

    #[test]
    fn builds_cli_line_from_json_command_and_args() {
        assert_eq!(
            command_line_from_body(r#"{"command":"set-brightness","args":{"brightness":-5}}"#)
                .unwrap(),
            "set-brightness --brightness -5"
        );
    }

    #[test]
    fn builds_cli_line_from_string_args() {
        assert_eq!(
            command_line_from_body(
                r#"{"command":"output-test-picture","args":{"background-color":"00,00,00","foreground-color":"ff,ff,ff","pattern":"color-bar","setting":0}}"#
            )
            .unwrap(),
            "output-test-picture --background-color 00,00,00 --foreground-color ff,ff,ff --pattern color-bar --setting 0"
        );
    }
}
