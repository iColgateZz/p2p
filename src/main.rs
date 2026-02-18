use p2p::http::{HttpParseError, HttpRequest, HttpResponse};
use p2p::ledger;
use p2p::peers;
use p2p::threadpool::ThreadPool;
use std::fs;
use std::io::Read;
use std::net::{TcpListener, TcpStream};

fn load_config(port: u16) -> Vec<(String, u16)> {
    let config_file = format!("peers_config_{}.json", port);

    if let Ok(content) = fs::read_to_string(&config_file) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(peers_arr) = data.get("peers").and_then(|v| v.as_array()) {
                let mut peers_list = Vec::new();
                for peer in peers_arr {
                    if let (Some(ip), Some(port)) = (
                        peer.get("ip").and_then(|v| v.as_str()),
                        peer.get("port").and_then(|v| v.as_u64()),
                    ) {
                        peers_list.push((ip.to_string(), port as u16));
                    }
                }
                return peers_list;
            }
        }
    }

    // Default bootstrap peers
    Vec::new()
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
                        eprintln!("[SERVER] Parse error: {:?}", e);
                        return;
                    }
                }
            }
            Err(_) => return,
        }
    }
}

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(5000); // if port is not specified then default is 5000

    let addr = format!("0.0.0.0:{}", port);

    println!("========================================");
    println!("P2P Distributed Ledger Node");
    println!("Listening on: {}", addr);
    println!("========================================\n");

    peers::set_self_peer("127.0.0.1".to_string(), port);

    // Load bootstrap peers from config
    let bootstrap_peers = load_config(port);
    if !bootstrap_peers.is_empty() {
        peers::add_bootstrap_peers(bootstrap_peers);
        println!("[INIT] Bootstrap peers loaded\n");
    }

    ledger::init_genesis_block();
    println!();

    let rt =  tokio::runtime::Runtime::new()
        .expect("[SERVER] Async runtime could not be started");
    rt.spawn(async {
        peers::discovery_loop().await;
    });

    let listener = match TcpListener::bind(&addr) {
        Ok(l) => {
            println!("[SERVER] TCP listener bound successfully\n");
            l
        }
        Err(e) => {
            eprintln!("[SERVER] Failed to bind: {}", e);
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
            Err(e) => eprintln!("[SERVER] Accept error: {}", e),
        }
    }
}
