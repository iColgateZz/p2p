use crate::http::threadpool::ThreadPool;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

pub trait HttpHandler: Send + Sync + 'static {
    fn handle(&self, request: HttpRequest) -> HttpResult;
}

pub struct HttpResult {
    pub status: u16,
    pub body: String,
    pub content_type: &'static str,
}

impl HttpResult {
    pub fn json<T: Serialize>(status: u16, value: &T) -> Self {
        let body = serde_json::to_string(value)
            .unwrap_or_else(|_| json!({"error": "serialization failed"}).to_string());

        Self {
            status,
            body,
            content_type: "application/json",
        }
    }

    pub fn ok<T: Serialize>(value: &T) -> Self {
        Self::json(200, value)
    }

    pub fn created<T: Serialize>(value: &T) -> Self {
        Self::json(201, value)
    }

    pub fn err(status: u16, msg: &str) -> Self {
        #[derive(Serialize)]
        struct ErrorResponse<'a> {
            error: &'a str,
        }

        Self::json(status, &ErrorResponse { error: msg })
    }

    pub fn not_found() -> Self {
        Self::err(404, "Not found")
    }

    pub fn bad_req() -> Self {
        Self::err(400, "Bad request")
    }

    pub fn not_impl() -> Self {
        Self::err(501, "Not implemented")
    }
}

pub fn start<H: HttpHandler>(addr: &str, handler: H) {
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => {
            println!("[SERVER] TCP listener bound successfully\n");
            l
        }
        Err(e) => {
            eprintln!("[ERROR] Failed to bind: {}", e);
            return;
        }
    };

    let pool = ThreadPool::new(32);
    let handler = Arc::new(handler);

    println!("[SERVER] Waiting for connections...\n");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let handler = Arc::clone(&handler);
                pool.execute(|| {
                    handle_client(stream, handler);
                });
            }
            Err(e) => eprintln!("[ERROR] Accept error: {}", e),
        }
    }
}

fn handle_client<H: HttpHandler>(mut stream: TcpStream, handler: Arc<H>) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];

    loop {
        match stream.read(&mut tmp) {
            Ok(0) => return, // client closed
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);

                match HttpRequest::try_from(&buf) {
                    Ok(req) => {
                        println!("[SERVER] Received request: {:?}", req.method);
                        HttpResponse::respond(&mut stream, req, handler);
                        return;
                    }

                    Err(HttpParseError::Incomplete) => {
                        continue;
                    }

                    Err(e) => {
                        eprintln!("[ERROR] Parse error: {:?}", e);
                        return;
                    }
                }
            }
            Err(_) => return,
        }
    }
}

pub struct HttpResponse;

impl HttpResponse {
    pub fn respond<H: HttpHandler>(stream: &mut TcpStream, req: HttpRequest, handler: Arc<H>) {
        let result = handler.handle(req);

        let response = format!(
            "HTTP/1.0 {} OK\r\n\
            Content-Type: {}\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST\r\n\
            \r\n\
            {}",
            result.status,
            result.content_type,
            result.body.len(),
            result.body,
        );

        let _ = stream.write_all(response.as_bytes());
    }
}

#[derive(Debug, Clone)]
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
    pub remote_addr: Option<String>,
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

            headers.insert(
                key.trim().to_string().to_lowercase(),
                value.trim().to_string(),
            );
        }

        let expected_len = headers
            .get("content-length")
            .map(|s| s.parse::<usize>().map_err(|_| HttpParseError::InvalidContentLength))
            .transpose()?
            .unwrap_or(0);

        let actual_len = body.as_bytes().len();

        if actual_len < expected_len {
            return Err(HttpParseError::Incomplete);
        }

        if actual_len > expected_len {
            return Err(HttpParseError::ContentLengthMismatch {
                expected: expected_len,
                actual: actual_len,
            });
        }

        Ok(HttpRequest {
            method,
            headers,
            body: body.to_string(),
            remote_addr: None,
        })
    }
}
