use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::cli::parse_request_line;
use crate::commands::CXN0102Notify;
use crate::cxn0102::CXN0102;

const MAX_BODY_SIZE: usize = 1024 * 1024;
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

    HttpResponse::json(
        200,
        format!(
            r#"{{"ok":true,"command":"{}","bytes":"{}"}}"#,
            json_escape(&line),
            format_hex(&bytes)
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

    match cxn0102.read_notify() {
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
        Ok(notify) => HttpResponse::json(
            500,
            json_error(&format!("expected temperature notify, received {notify:?}")),
        ),
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

    match cxn0102.read_notify() {
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
        Ok(notify) => HttpResponse::json(
            500,
            json_error(&format!("expected version notify, received {notify:?}")),
        ),
        Err(error) => HttpResponse::json(500, json_error(&error.to_string())),
    }
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
