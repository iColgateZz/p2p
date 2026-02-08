use p2p::http::{HttpParseError, HttpRequest, HttpResponse};
use p2p::threadpool::ThreadPool;
use std::io::Read;
use std::net::{TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];

    loop {
        match stream.read(&mut tmp) {
            Ok(0) => return, // client closed
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);

                match HttpRequest::try_from(&buf) {
                    Ok(req) => {
                        println!("{req:#?}");
                        HttpResponse::respond(&mut stream, req);
                        return;
                    },

                    Err(HttpParseError::Incomplete) => {
                        continue;
                    },

                    Err(e) => {
                        //TODO 404 Bad Request
                        eprintln!("{e:?}");
                        return;
                    }
                }
            }
            Err(_) => return,
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let pool = ThreadPool::new(20);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| {
                    handle_client(stream);
                });
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }
}
