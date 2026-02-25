pub mod client;
pub mod protocol;
pub mod route;
pub mod server;

use crate::http;
use crate::ledger;
use crate::node;
use crate::peers;
use protocol::PeersDto;
use std::{fs, process};
use tokio::runtime::Runtime;

pub fn start(ip: &str, port: u16) {
    let addr = format!("{ip}:{port}");

    println!("========================================");
    println!("P2P Distributed Ledger Node");
    println!("Starting on: {}", addr);
    println!("========================================");
    println!();

    peers::set_self_peer(ip.into(), port);
    println!("[NODE] Added {}:{} as self", ip, port);

    load_peers();
    println!("[NODE] Peers loaded from config");

    ledger::init_genesis_block();
    println!();

    let _rt = start_async_background_jobs();
    println!("[NODE] Started background jobs");

    http::server::start(&addr, node::server::RequestHandler);
}

fn start_async_background_jobs() -> Runtime {
    let rt = Runtime::new().expect("[ERROR] Async runtime could not be started");

    rt.spawn(async {
        node::client::discovery_loop().await;
    });

    rt
}

fn load_peers() {
    let bootstrap_peers = load_peer_config();
    let peers = bootstrap_peers
        .peers
        .iter()
        .map(|p| (p.ip.clone(), p.port))
        .collect();
    peers::add_bootstrap_peers(peers);
}

fn load_peer_config() -> PeersDto {
    let config_file = "peers_config.json";

    let content = fs::read_to_string(config_file).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", config_file, e);
        process::exit(1);
    });

    serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Could not parse {}: {}", config_file, e);
        process::exit(1);
    })
}
