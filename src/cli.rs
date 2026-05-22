use std::collections::HashMap;

use crate::commands::*;

type CliResult<T> = std::result::Result<T, String>;

pub fn parse_request_line(line: &str) -> CliResult<Option<Vec<u8>>> {
    let mut parts = line.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(None);
    };

    let command = normalize_name(command);
    if command == "help" {
        print_help();
        return Ok(None);
    }

    let mut args = Args::new(parts.collect::<Vec<_>>())?;
    let bytes = match command.as_str() {
        "startinput" => request_bytes(StartInput, &mut args),
        "stopinput" => request_bytes(StopInput, &mut args),
        "stopinputspecially" => request_bytes(
            StopInputSpecially {
                output: parse_stopped_output(&args.required("output")?)?,
            },
            &mut args,
        ),
        "muteunmutechangeoutput" => request_bytes(
            MuteUnmuteChangeOutput {
                output: parse_mute_output(&args.required("output")?)?,
            },
            &mut args,
        ),
        "saveuserparam" => request_bytes(
            SaveUserParam {
                video_position: parse_save_video_position(&args.required("video-position")?)?,
                optical_alignment_biphase: parse_save_optical_alignment_biphase(
                    &args.required("optical-alignment-biphase")?,
                )?,
                picture_quality: parse_save_picture_quality(&args.required("picture-quality")?)?,
            },
            &mut args,
        ),
        "initializeuserparam" => request_bytes(InitializeUserParam, &mut args),
        "shutdownreboot" | "shutdown" => request_bytes(
            ShutdownReboot {
                option: parse_shutdown_option(&args.required("option")?)?,
            },
            &mut args,
        ),
        "getvideooutputposition" => request_bytes(GetVideoOutputPosition, &mut args),
        "setvideooutputposition" => request_bytes(
            SetVideoOutputPosition {
                pan: Pan(parse_i8(&args.required("pan")?)?),
                tilt: Tilt(parse_i8(&args.required("tilt")?)?),
                flip: parse_flip(&args.required("flip")?)?,
            },
            &mut args,
        ),
        "getopticalalignment" => request_bytes(GetOpticalAlignment, &mut args),
        "setopticalalignment" => request_bytes(
            SetOpticalAlignment {
                alignment: OpticalAlignment {
                    r0_horizontal: I8Byte(parse_i8(&args.required("r0-horizontal")?)?),
                    r1_horizontal: I8Byte(parse_i8(&args.required("r1-horizontal")?)?),
                    g0_horizontal: I8Byte(parse_i8(&args.required("g0-horizontal")?)?),
                    g1_horizontal: I8Byte(parse_i8(&args.required("g1-horizontal")?)?),
                    b_horizontal: I8Byte(parse_i8(&args.required("b-horizontal")?)?),
                    r0_vertical: parse_u8(&args.required("r0-vertical")?)?,
                    r1_vertical: parse_u8(&args.required("r1-vertical")?)?,
                    g0_vertical: parse_u8(&args.required("g0-vertical")?)?,
                    g1_vertical: parse_u8(&args.required("g1-vertical")?)?,
                    b_vertical: parse_u8(&args.required("b-vertical")?)?,
                },
            },
            &mut args,
        ),
        "getbiphase" => request_bytes(GetBiphase, &mut args),
        "setbiphase" => request_bytes(
            SetBiphase {
                amount: parse_u32(&args.required("amount")?)?,
            },
            &mut args,
        ),
        "seteasyopticaladjustmentcontrol" => {
            request_bytes(SetEasyOpticalAdjustmentControl, &mut args)
        }
        "seteasyopticaladjustmentplus" => request_bytes(SetEasyOpticalAdjustmentPlus, &mut args),
        "seteasyopticaladjustmentminus" => request_bytes(SetEasyOpticalAdjustmentMinus, &mut args),
        "seteasyopticaladjustmentexit" => request_bytes(
            SetEasyOpticalAdjustmentExit {
                save: parse_save_adjustment(&args.required("save")?)?,
            },
            &mut args,
        ),
        "seteasybiphaseadjustmentcontrol" => {
            request_bytes(SetEasyBiphaseAdjustmentControl, &mut args)
        }
        "seteasybiphaseadjustmentplus" => request_bytes(SetEasyBiphaseAdjustmentPlus, &mut args),
        "seteasybiphaseadjustmentminus" => request_bytes(SetEasyBiphaseAdjustmentMinus, &mut args),
        "seteasybiphaseadjustmentexit" => request_bytes(
            SetEasyBiphaseAdjustmentExit {
                save: parse_save_adjustment(&args.required("save")?)?,
            },
            &mut args,
        ),
        "getallpicturequality" => request_bytes(GetAllPictureQuality, &mut args),
        "setallpicturequality" => request_bytes(
            SetAllPictureQuality {
                picture_quality: PictureQuality {
                    contrast: parse_i8(&args.required("contrast")?)?,
                    brightness: parse_i8(&args.required("brightness")?)?,
                    hue_u: parse_i8(&args.required("hue-u")?)?,
                    hue_v: parse_i8(&args.required("hue-v")?)?,
                    saturation_u: parse_i8(&args.required("saturation-u")?)?,
                    saturation_v: parse_i8(&args.required("saturation-v")?)?,
                    sharpness: parse_u8(&args.required("sharpness")?)?,
                },
            },
            &mut args,
        ),
        "getbrightness" => request_bytes(GetBrightness, &mut args),
        "setbrightness" => request_bytes(
            SetBrightness {
                brightness: parse_i8(&args.required("brightness")?)?,
            },
            &mut args,
        ),
        "getcontrast" => request_bytes(GetContrast, &mut args),
        "setcontrast" => request_bytes(
            SetContrast {
                contrast: parse_i8(&args.required("contrast")?)?,
            },
            &mut args,
        ),
        "gethue" => request_bytes(GetHue, &mut args),
        "sethue" => request_bytes(
            SetHue {
                hue_u: parse_i8(&args.required("hue-u")?)?,
                hue_v: parse_i8(&args.required("hue-v")?)?,
            },
            &mut args,
        ),
        "getsaturation" => request_bytes(GetSaturation, &mut args),
        "setsaturation" => request_bytes(
            SetSaturation {
                saturation_u: parse_i8(&args.required("saturation-u")?)?,
                saturation_v: parse_i8(&args.required("saturation-v")?)?,
            },
            &mut args,
        ),
        "getsharpness" => request_bytes(GetSharpness, &mut args),
        "setsharpness" => request_bytes(
            SetSharpness {
                sharpness: parse_u8(&args.required("sharpness")?)?,
            },
            &mut args,
        ),
        "updatefwimage" => request_bytes(
            UpdateFwImage {
                data_size: parse_u32(&args.required("data-size")?)?,
                checksum: parse_u32(&args.required("checksum")?)?,
                data: parse_bytes(&args.required("data")?)?,
            },
            &mut args,
        ),
        "updatepicturedata" => request_bytes(
            UpdatePictureData {
                data_size: parse_u32(&args.required("data-size")?)?,
                checksum: parse_u32(&args.required("checksum")?)?,
                data: parse_bytes(&args.required("data")?)?,
            },
            &mut args,
        ),
        "divisiontransmissionupdatefwimage" => request_bytes(
            DivisionTransmissionUpdateFwImage {
                data_size: parse_u32(&args.required("data-size")?)?,
                checksum: parse_u32(&args.required("checksum")?)?,
                format: parse_division_transmission_format(&args.required("format")?)?,
                max_block_number: parse_u16(&args.required("max-block-number")?)?,
            },
            &mut args,
        ),
        "divisiontransmissionupdatepicturedata" => request_bytes(
            DivisionTransmissionUpdatePictureData {
                data_size: parse_u32(&args.required("data-size")?)?,
                checksum: parse_u32(&args.required("checksum")?)?,
                format: parse_division_transmission_format(&args.required("format")?)?,
                max_block_number: parse_u16(&args.required("max-block-number")?)?,
            },
            &mut args,
        ),
        "divisiontransmissionupdatedata" => {
            let request = DivisionTransmissionUpdateData::new(
                parse_division_transmission_format(&args.required("format")?)?,
                parse_u16(&args.required("block-number")?)?,
                parse_bytes(&args.required("data")?)?,
            )
            .map_err(|error| format!("{error:?}"))?;
            request_bytes(request, &mut args)
        }
        "gettemperature" => request_bytes(GetTemperature, &mut args),
        "gettime" => request_bytes(GetTime, &mut args),
        "getversion" => request_bytes(GetVersion, &mut args),
        "outputtestpicture" => request_bytes(
            OutputTestPicture {
                pattern: parse_test_pattern(&args.required("pattern")?)?,
                setting: parse_u8(&args.required("setting")?)?,
                background_color: parse_rgb(&args.required("background-color")?)?,
                foreground_color: parse_rgb(&args.required("foreground-color")?)?,
            },
            &mut args,
        ),
        "getlotnumber" => request_bytes(GetLotNumber, &mut args),
        "getserialnumber" => request_bytes(GetSerialNumber, &mut args),
        "raw" => {
            let bytes = parse_bytes(&args.required("bytes")?)?;
            ensure_no_extra_args(&args)?;
            Ok(bytes)
        }
        _ => Err(format!("unknown command '{command}'")),
    }?;

    Ok(Some(bytes))
}

fn request_bytes(request: impl Request, args: &mut Args) -> CliResult<Vec<u8>> {
    ensure_no_extra_args(args)?;
    Ok(request.to_bytes())
}

fn ensure_no_extra_args(args: &Args) -> CliResult<()> {
    if args.flags.is_empty() {
        Ok(())
    } else {
        let mut keys = args.flags.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        Err(format!("unused flag(s): {}", keys.join(", ")))
    }
}

struct Args {
    flags: HashMap<String, String>,
}

impl Args {
    fn new(tokens: Vec<&str>) -> CliResult<Self> {
        let mut flags = HashMap::new();
        let mut index = 0;

        while index < tokens.len() {
            let flag = tokens[index];
            if !flag.starts_with("--") {
                return Err(format!("expected flag, got '{flag}'"));
            }

            let key = normalize_name(flag.trim_start_matches("--"));
            index += 1;

            let Some(value) = tokens.get(index) else {
                return Err(format!("missing value for --{key}"));
            };
            if value.starts_with("--") {
                return Err(format!("missing value for --{key}"));
            }

            flags.insert(key, (*value).to_owned());
            index += 1;
        }

        Ok(Self { flags })
    }

    fn required(&mut self, name: &str) -> CliResult<String> {
        let key = normalize_name(name);
        self.flags
            .remove(&key)
            .ok_or_else(|| format!("missing --{name}"))
    }
}

fn parse_rgb(value: &str) -> CliResult<Rgb> {
    let bytes = parse_bytes(value)?;
    if bytes.len() != 3 {
        return Err(format!("RGB value must have 3 bytes, got {}", bytes.len()));
    }
    Ok(Rgb {
        r: bytes[0],
        g: bytes[1],
        b: bytes[2],
    })
}

fn parse_bytes(value: &str) -> CliResult<Vec<u8>> {
    let parts = value
        .split([',', ':'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.len() > 1 {
        return parts.into_iter().map(parse_u8).collect();
    }

    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if hex.len() >= 2 && hex.len() % 2 == 0 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).map_err(|e| e.to_string()))
            .collect();
    }

    Ok(vec![parse_u8(value)?])
}

fn parse_i8(value: &str) -> CliResult<i8> {
    parse_signed(value, i8::MIN as i64, i8::MAX as i64).map(|value| value as i8)
}

fn parse_u8(value: &str) -> CliResult<u8> {
    parse_unsigned(value, u8::MAX as u64).map(|value| value as u8)
}

fn parse_u16(value: &str) -> CliResult<u16> {
    parse_unsigned(value, u16::MAX as u64).map(|value| value as u16)
}

fn parse_u32(value: &str) -> CliResult<u32> {
    parse_unsigned(value, u32::MAX as u64).map(|value| value as u32)
}

fn parse_signed(value: &str, min: i64, max: i64) -> CliResult<i64> {
    let parsed = if let Some(hex) = value.strip_prefix("-0x") {
        -i64::from_str_radix(hex, 16).map_err(|e| e.to_string())?
    } else if let Some(hex) = value.strip_prefix("-0X") {
        -i64::from_str_radix(hex, 16).map_err(|e| e.to_string())?
    } else if let Some(hex) = value.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).map_err(|e| e.to_string())?
    } else if let Some(hex) = value.strip_prefix("0X") {
        i64::from_str_radix(hex, 16).map_err(|e| e.to_string())?
    } else {
        value.parse::<i64>().map_err(|e| e.to_string())?
    };

    if parsed < min || parsed > max {
        return Err(format!("{parsed} is outside {min}..={max}"));
    }
    Ok(parsed)
}

fn parse_unsigned(value: &str, max: u64) -> CliResult<u64> {
    let parsed = if let Some(hex) = value.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|e| e.to_string())?
    } else if let Some(hex) = value.strip_prefix("0X") {
        u64::from_str_radix(hex, 16).map_err(|e| e.to_string())?
    } else {
        value.parse::<u64>().map_err(|e| e.to_string())?
    };

    if parsed > max {
        return Err(format!("{parsed} is outside 0..={max}"));
    }
    Ok(parsed)
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

macro_rules! enum_parser {
    ($fn_name:ident, $ty:ty, { $($name:literal => $variant:expr,)+ }) => {
        fn $fn_name(value: &str) -> CliResult<$ty> {
            match normalize_name(value).as_str() {
                $($name => Ok($variant),)+
                _ => {
                    if let Ok(byte) = parse_u8(value) {
                        <$ty>::try_from(byte).map_err(|error| format!("{error:?}"))
                    } else {
                        Err(format!("invalid {} value '{value}'", stringify!($ty)))
                    }
                }
            }
        }
    };
}

enum_parser!(parse_stopped_output, StoppedOutput, {
    "openingpicture" => StoppedOutput::OpeningPicture,
    "muteblackpicture" => StoppedOutput::MuteBlackPicture,
    "muteopeningpicture" => StoppedOutput::MuteOpeningPicture,
    "finalpicture" => StoppedOutput::FinalPicture,
});

enum_parser!(parse_mute_output, MuteOutput, {
    "unmute" => MuteOutput::Unmute,
    "muteblackpicture" => MuteOutput::MuteBlackPicture,
    "muteopeningpicture" => MuteOutput::MuteOpeningPicture,
    "finalpicture" => MuteOutput::FinalPicture,
});

enum_parser!(parse_save_video_position, SaveVideoPosition, {
    "dontsave" => SaveVideoPosition::DontSave,
    "all" => SaveVideoPosition::All,
    "fliponly" => SaveVideoPosition::FlipOnly,
});

enum_parser!(parse_save_optical_alignment_biphase, SaveOpticalAlignmentBiphase, {
    "dontsave" => SaveOpticalAlignmentBiphase::DontSave,
    "all" => SaveOpticalAlignmentBiphase::All,
    "opticalalignmentonly" => SaveOpticalAlignmentBiphase::OpticalAlignmentOnly,
    "biphaseonly" => SaveOpticalAlignmentBiphase::BiphaseOnly,
});

enum_parser!(parse_save_picture_quality, SavePictureQuality, {
    "dontsave" => SavePictureQuality::DontSave,
    "save" => SavePictureQuality::Save,
});

enum_parser!(parse_shutdown_option, ShutdownOption, {
    "stopsallfunctions" => ShutdownOption::StopsAllFunctions,
    "reboot" => ShutdownOption::Reboot,
});

enum_parser!(parse_flip, Flip, {
    "off" => Flip::Off,
    "rightleft" => Flip::RightLeft,
    "updown" => Flip::UpDown,
    "updownandrightleft" => Flip::UpDownAndRightLeft,
});

enum_parser!(parse_save_adjustment, SaveAdjustment, {
    "dontsave" => SaveAdjustment::DontSave,
    "save" => SaveAdjustment::Save,
});

enum_parser!(parse_division_transmission_format, DivisionTransmissionFormat, {
    "size64" => DivisionTransmissionFormat::Size64,
    "64" => DivisionTransmissionFormat::Size64,
    "size256" => DivisionTransmissionFormat::Size256,
    "256" => DivisionTransmissionFormat::Size256,
    "size1k" => DivisionTransmissionFormat::Size1K,
    "1k" => DivisionTransmissionFormat::Size1K,
    "size4k" => DivisionTransmissionFormat::Size4K,
    "4k" => DivisionTransmissionFormat::Size4K,
    "size16k" => DivisionTransmissionFormat::Size16K,
    "16k" => DivisionTransmissionFormat::Size16K,
});

enum_parser!(parse_test_pattern, TestPattern, {
    "stop" => TestPattern::Stop,
    "colorbar" => TestPattern::ColorBar,
    "crosshatching" => TestPattern::CrossHatching,
    "raster" => TestPattern::Raster,
    "ramp" => TestPattern::Ramp,
    "circle" => TestPattern::Circle,
    "cross" => TestPattern::Cross,
    "circleandcross" => TestPattern::CircleAndCross,
    "filledcircle" => TestPattern::FilledCircle,
    "filledsquare" => TestPattern::FilledSquare,
    "checkerboard" => TestPattern::Checkerboard,
    "resolutioncheckerverticallines" => TestPattern::ResolutionCheckerVerticalLines,
    "resolutioncheckerhorizontallines" => TestPattern::ResolutionCheckerHorizontalLines,
    "resolutioncheckersquare" => TestPattern::ResolutionCheckerSquare,
    "colorbarrampspecial" => TestPattern::ColorBarRampSpecial,
    "rectangularhatcheven" => TestPattern::RectangularHatchEven,
    "rectangularhatchequalcrosshatchinterval" => TestPattern::RectangularHatchEqualCrossHatchInterval,
});

fn print_help() {
    println!(
        "Enter commands as kebab-case names with --field values, for example:\n\
         start-input\n\
         set-brightness --brightness -5\n\
         get-version\n\
         output-test-picture --pattern color-bar --setting 0 --background-color 00,00,00 --foreground-color ff,ff,ff\n\
         raw --bytes 01,00"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_command() {
        assert_eq!(
            parse_request_line("start-input").unwrap(),
            Some(vec![0x01, 0x00])
        );
    }

    #[test]
    fn parses_i8_command() {
        assert_eq!(
            parse_request_line("set-brightness --brightness -5").unwrap(),
            Some(vec![0x43, 0x01, 0xfb])
        );
    }

    #[test]
    fn parses_enum_command() {
        assert_eq!(
            parse_request_line("shutdown-reboot --option reboot").unwrap(),
            Some(vec![0x0b, 0x01, 0x01])
        );
    }
}
