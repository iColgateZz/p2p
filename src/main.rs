use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write};
use std::time::Duration;
use std::thread;

struct TcpConnection {
    stream: TcpStream,
    in_buf: Vec<u8>,
    closed: bool
}

impl TcpConnection {
    fn new(stream: TcpStream) -> TcpConnection {
        stream.set_nonblocking(true).expect("err2");

        TcpConnection { 
            stream,
            in_buf: Vec::new(),
            closed: false,
        }
    }
}

struct HttpRequest;

impl HttpRequest {
    fn try_from(buf: &[u8]) -> Option<HttpRequest> {
        if !buf.ends_with(b"\r\n\r\n") {
            return None;
        }

        return Some(HttpRequest {});
    }
}

struct HttpResponse;

impl HttpResponse {
    fn respond(conn: &mut TcpConnection, _req: HttpRequest) {
        let body = "Hello, world!\n";

        // HTTP/1.0 closes connections after response
        let response = format!(
            "HTTP/1.0 200 OK\r\n\
            Content-Length: {}\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            {}",
            body.len(),
            body
        );

        conn.stream.write_all(response.as_bytes()).expect("err3");
    }
}

fn main() {
    let listener = TcpListener::bind("localhost:8080").unwrap();
    listener.set_nonblocking(true).expect("err1");

    let mut connections = Vec::new();
    let mut buffer = [0u8; 1024];

    loop {
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    connections.push(TcpConnection::new(s));
                },
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                },
                Err(e) => panic!("accept error: {e}")
            }
        }
    
        for conn in connections.iter_mut() {
            match conn.stream.read(&mut buffer) {
                Ok(0) => {
                    conn.closed = true;
                    println!("Closed connection");
                },
                Ok(n) => {
                    conn.in_buf.extend_from_slice(&buffer[..n]);

                    if let Some(req) = HttpRequest::try_from(&conn.in_buf) {
                        HttpResponse::respond(conn, req);
                    }
                },
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {},
                Err(e) => panic!("read error: {e}"),
            }
        }

        connections.retain(|c| !c.closed);
        thread::sleep(Duration::from_millis(1));
    }
}