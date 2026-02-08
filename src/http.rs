use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;

#[derive(Debug)]
pub enum HttpMethod {
    GET(String),
    POST(String),
    PUT(String),
    DELETE(String),
}

#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Debug)]
pub enum HttpParseError {
    InvalidUtf8,
    MissingRequestLine,
    InvalidRequestLine,
    UnsupportedMethod(String),
    InvalidHeaderLine,
    MissingContentLength,
    InvalidContentLength,
    ContentLengthMismatch { expected: usize, actual: usize },
    Incomplete,
}

impl HttpRequest {
    pub fn try_from(buf: &[u8]) -> Result<HttpRequest, HttpParseError> {
        let text = std::str::from_utf8(buf).map_err(|_| HttpParseError::InvalidUtf8)?;

        let (head, body) = text
            .split_once("\r\n\r\n")
            .ok_or(HttpParseError::Incomplete)?;

        let mut lines = head.lines();
        let request_line = lines.next().ok_or(HttpParseError::MissingRequestLine)?;

        let mut parts = request_line.split_whitespace();
        let method_str = parts.next().ok_or(HttpParseError::InvalidRequestLine)?;
        let path = parts
            .next()
            .ok_or(HttpParseError::InvalidRequestLine)?
            .to_string();
        let _version = parts.next().ok_or(HttpParseError::InvalidRequestLine)?;

        let method = match method_str {
            "GET" => HttpMethod::GET(path),
            "POST" => HttpMethod::POST(path),
            "PUT" => HttpMethod::PUT(path),
            "DELETE" => HttpMethod::DELETE(path),
            other => return Err(HttpParseError::UnsupportedMethod(other.to_string())),
        };

        let mut headers = HashMap::new();
        for line in lines {
            let (key, value) = line
                .split_once(':')
                .ok_or(HttpParseError::InvalidHeaderLine)?;

            headers.insert(key.trim().to_string(), value.trim().to_string());
        }

        if !body.is_empty() {
            let len_str = headers
                .get("Content-Length")
                .ok_or(HttpParseError::MissingContentLength)?;

            let expected_len = len_str
                .parse::<usize>()
                .map_err(|_| HttpParseError::InvalidContentLength)?;

            let actual_len = body.as_bytes().len();

            if actual_len != expected_len {
                return Err(HttpParseError::ContentLengthMismatch {
                    expected: expected_len,
                    actual: actual_len,
                });
            }
        }

        Ok(HttpRequest {
            method,
            headers,
            body: body.to_string(),
        })
    }
}

pub struct HttpResponse;

impl HttpResponse {
    pub fn respond(stream: &mut TcpStream, _req: HttpRequest) {
        let body = "Hello, world!\n";

        let response = format!(
            "HTTP/1.0 200 OK\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            {}",
            body.len(),
            body
        );

        stream.write_all(response.as_bytes()).unwrap();
    }
}
