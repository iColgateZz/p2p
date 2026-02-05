use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;

struct HttpRequest;

impl HttpRequest {
    fn try_from(buf: &[u8]) -> Option<HttpRequest> {
        if buf.ends_with(b"\r\n\r\n") {
            Some(HttpRequest {})
        } else {
            None
        }
    }
}

struct HttpResponse;

impl HttpResponse {
    fn respond(stream: &mut TcpStream, _req: HttpRequest) {
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

        let _ = stream.write_all(response.as_bytes());
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];

    loop {
        match stream.read(&mut tmp) {
            Ok(0) => return, // client closed
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);

                if let Some(req) = HttpRequest::try_from(&buf) {
                    HttpResponse::respond(&mut stream, req);
                    return; // HTTP/1.0 → close after response
                }
            }
            Err(_) => return,
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Threaded server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    handle_client(stream);
                });
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }

    Ok(())
}