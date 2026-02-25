pub mod client;
pub mod protocol;
pub mod route;
pub mod server;

use std::fs;
use crate::node;
use crate::peers;
use crate::http;
use tokio::runtime::Runtime;

pub fn start(ip: &str, port: u16) {
    let addr = format!("{ip}:{port}");

    println!("========================================");
    println!("P2P Distributed Ledger Node");
    println!("Starting on: {}", addr);
    println!("========================================\n");

    peers::set_self_peer(ip.to_string(), port);
    println!("Added {}:{} as self", ip, port);
    
    load_peers();
    println!("Peers loaded from config");

    // ledger::init_genesis_block();
    println!();

    let rt = Runtime::new().expect("[ERROR] Async runtime could not be started");
    rt.spawn(async {
        node::client::discovery_loop().await;
    });

    http::server::start(&addr, node::server::RequestHandler);
}

fn load_peers() {
    // Load bootstrap peers from config
    let bootstrap_peers = load_config(5000);
    if !bootstrap_peers.is_empty() {
        peers::add_bootstrap_peers(bootstrap_peers);
        println!("[PEERS] Bootstrap peers loaded\n");
    }
}

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
