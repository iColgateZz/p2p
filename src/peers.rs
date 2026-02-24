use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct PeerInfo {
    pub ip: String,
    pub port: u16,
}

impl PeerInfo {
    pub fn new(ip: String, port: u16) -> Self {
        PeerInfo { ip, port }
    }

    pub fn to_url(&self, path: &str) -> String {
        format!("http://{}:{}{}", self.ip, self.port, path)
    }

    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

lazy_static! {
    static ref KNOWN_PEERS: Mutex<HashSet<PeerInfo>> = Mutex::new(HashSet::new());
}

static SELF_PEER: OnceLock<PeerInfo> = OnceLock::new();

pub fn set_self_peer(ip: String, port: u16) {
    let peer = PeerInfo::new(ip, port);
    SELF_PEER
        .set(peer)
        .expect("[ERROR] SELF_PEER value was already set");
}

pub fn add_bootstrap_peers(peers: Vec<(String, u16)>) {
    let mut known = KNOWN_PEERS.lock().unwrap();
    for (ip, port) in peers {
        known.insert(PeerInfo::new(ip, port));
    }
    println!("[PEERS] Added {} bootstrap peers", known.len());
}

pub fn add_peer(ip: String, port: u16) -> bool {
    let peer = PeerInfo::new(ip, port);
    let mut known = KNOWN_PEERS.lock().unwrap();

    if known.insert(peer.clone()) {
        println!("[PEERS] Added new peer: {}:{}", peer.ip, peer.port);
        true
    } else {
        false
    }
}

pub fn get_known_peers() -> Vec<PeerInfo> {
    let known = KNOWN_PEERS.lock().unwrap();
    known.iter().cloned().collect()
}
