use serde_json::json;
use std::collections::HashMap;
use std::io::{Write, Read};
use std::net::{TcpStream, TcpListener};
use crate::threadpool::ThreadPool;

pub fn start_http_server(addr: &str) {
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

    let pool = ThreadPool::new(20);

    println!("[SERVER] Waiting for connections...\n");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| {
                    handle_client(stream);
                });
            }
            Err(e) => eprintln!("[ERROR] Accept error: {}", e),
        }
    }
}

fn handle_client(mut stream: TcpStream) {
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
                        HttpResponse::respond(&mut stream, req);
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

#[derive(Debug, Clone)]
pub enum HttpMethod {
    GET(String),
    POST(String),
    PUT(String),
    DELETE(String),
}

impl HttpMethod {
    pub fn path(&self) -> &str {
        match self {
            HttpMethod::GET(p)
            | HttpMethod::POST(p)
            | HttpMethod::PUT(p)
            | HttpMethod::DELETE(p) => p,
        }
    }
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
            remote_addr: None,
        })
    }
}

pub struct HttpResponse;

impl HttpResponse {
    pub fn respond(stream: &mut TcpStream, req: HttpRequest) {
        let path = req.method.path();

        let body = if path.starts_with("/addr") {
            crate::peers::get_known_peers_json()
        } else if path.starts_with("/getblocks") {
            crate::ledger::handle_getblocks_request(path)
        } else if path.starts_with("/getdata") {
            crate::ledger::handle_getdata_request(path)
        } else if path.starts_with("/inv") && matches!(req.method, HttpMethod::POST(_)) {
            crate::ledger::handle_inv_request(&req.body)
        } else if path.starts_with("/block") && matches!(req.method, HttpMethod::POST(_)) {
            crate::ledger::handle_block_request(&req.body)
        } else {
            json!({"status": "ok", "message": "P2P node active"}).to_string()
        };

        let response = format!(
            "HTTP/1.0 200 OK\r\n\
            Content-Length: {}\r\n\
            Content-Type: application/json\r\n\
            \r\n\
            {}",
            body.len(),
            body
        );

        let _ = stream.write_all(response.as_bytes());
    }
}
