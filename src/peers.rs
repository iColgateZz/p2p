use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
}

impl Peer {
    pub fn new(ip: String, port: u16) -> Self {
        Peer { ip, port }
    }

    pub fn to_url(&self, path: &str) -> String {
        format!("http://{}:{}{}", self.ip, self.port, path)
    }

    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

lazy_static! {
    static ref KNOWN_PEERS: Mutex<HashMap<Peer, u8>> = Mutex::new(HashMap::new());
}

static SELF_PEER: OnceLock<Peer> = OnceLock::new();

pub fn set_self_peer(ip: String, port: u16) {
    add_peer(ip.clone(), port);

    let peer = Peer::new(ip, port);
    SELF_PEER
        .set(peer)
        .expect("[ERROR] SELF_PEER value was already set");
}

pub fn add_bootstrap_peers(peers: Vec<(String, u16)>) {
    let mut known = KNOWN_PEERS.lock().unwrap();
    for (ip, port) in peers {
        known.insert(Peer::new(ip, port), std::u8::MAX);
    }
    println!("[PEERS] Added {} bootstrap peers", known.len());
}

pub fn add_peer(ip: String, port: u16) -> bool {
    let peer = Peer::new(ip, port);
    let mut known = KNOWN_PEERS.lock().unwrap();

    if known.insert(peer.clone(), std::u8::MAX).is_none() {
        println!("[PEERS] Added new peer: {}:{}", peer.ip, peer.port);
        true
    } else {
        false
    }
}

pub fn update_peer(p: Peer) {
    let mut known = KNOWN_PEERS.lock().unwrap();

    if let Some(score) = known.get_mut(&p) {
        *score = score.saturating_sub(1);

        if *score == 0 {
            known.remove(&p);
        }
    }
}

pub fn get_known_peers() -> Vec<Peer> {
    let known = KNOWN_PEERS.lock().unwrap();
    known.iter().map(|(k, _)| k).cloned().collect()
}

pub fn select_random_peers() -> Vec<Peer> {
    let peers = KNOWN_PEERS.lock().unwrap();
    let mut rng = thread_rng();

    let mut peers: Vec<Peer> = peers.iter().map(|(p, _)| p.clone()).collect();
    peers.shuffle(&mut rng);
    peers.into_iter().take(100).collect()
}
