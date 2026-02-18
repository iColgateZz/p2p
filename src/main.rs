use p2p::http_server;
use p2p::ledger;
use p2p::peers;
use std::fs;

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

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(5000); // if port is not specified then default is 5000

    let addr = format!("0.0.0.0:{}", port);

    println!("========================================");
    println!("P2P Distributed Ledger Node");
    println!("Starting on: {}", addr);
    println!("========================================\n");

    let ip = "127.0.0.1";
    peers::set_self_peer(ip.to_string(), port);
    println!("[PEERS] Self peer set to: {}:{}", ip, port);

    // Load bootstrap peers from config
    let bootstrap_peers = load_config(port);
    if !bootstrap_peers.is_empty() {
        peers::add_bootstrap_peers(bootstrap_peers);
        println!("[PEERS] Bootstrap peers loaded\n");
    }

    ledger::init_genesis_block();
    println!();

    let rt =  tokio::runtime::Runtime::new()
        .expect("[ERROR] Async runtime could not be started");
    rt.spawn(async {
        peers::discovery_loop().await;
    });

    http_server::start_http_server(&addr);
}
