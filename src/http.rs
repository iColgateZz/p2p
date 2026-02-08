use std::io::Write;
use std::net::TcpStream;
pub struct HttpRequest;

impl HttpRequest {
    pub fn try_from(buf: &[u8]) -> Option<HttpRequest> {
        if buf.ends_with(b"\r\n\r\n") {
            Some(HttpRequest {})
        } else {
            None
        }
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
