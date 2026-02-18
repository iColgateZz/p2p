use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::Mutex;
use reqwest::Client;
use std::sync::OnceLock;
use futures::future::join_all;
use tokio::time::{sleep, Duration};

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
    static ref SELF_PEER: Mutex<Option<PeerInfo>> = Mutex::new(None);
    static ref HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
}

pub fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_max_idle_per_host(256)
            .build()
            .unwrap()
    })
}

pub fn set_self_peer(ip: String, port: u16) {
    let ip_clone = ip.clone();
    let peer = PeerInfo::new(ip, port);
    let mut self_peer = SELF_PEER.lock().unwrap();
    *self_peer = Some(peer);
    println!("[PEERS] Self peer set to: {}:{}", ip_clone, port);
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

pub fn get_known_peers_json() -> String {
    let peers = get_known_peers();
    let peer_list: Vec<Value> = peers
        .iter()
        .map(|p| json!({"ip": p.ip, "port": p.port}))
        .collect();
    json!({"peers": peer_list, "count": peer_list.len()}).to_string()
}

pub async fn discover_peers() {
    let peers = get_known_peers();
    
    let futures = peers.into_iter().map(|peer| {
        let client = http_client();
        let url = peer.to_url("/addr");

        async move {
            println!("[DISCOVERY] Querying peer: {}", url);

            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(text) = resp.text().await {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            if let Some(arr) = data.get("peers").and_then(|v| v.as_array()) {
                                for peer_obj in arr {
                                    if let (Some(ip), Some(port)) = (
                                        peer_obj.get("ip").and_then(|v| v.as_str()),
                                        peer_obj.get("port").and_then(|v| v.as_u64()),
                                    ) {
                                        add_peer(ip.to_string(), port as u16);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[DISCOVERY] Failed to query peer {}: {}", url, e);
                }
            }
        }
    });

    join_all(futures).await;
}

pub async fn fetch_blocks_from_peers() {
    let peers = get_known_peers();
    
    let futures = peers.into_iter().map(|peer| {
        let client = http_client();
        let url = peer.to_url("/getblocks");

        async move {
            println!("[SYNC] Fetching blocks from: {}", url);
            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(text) = resp.text().await {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            if let Some(blocks) = data.get("blocks").and_then(|v| v.as_array()) {
                                for block_hash in blocks {
                                    if let Some(hash) = block_hash.as_str() {
                                        fetch_block(&peer, hash).await;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[SYNC] Failed to fetch block list: {}", e);
                }
            }
        }
    });

    join_all(futures).await;
}

async fn fetch_block(peer: &PeerInfo, hash: &str) {
    let client = http_client();
    let url = peer.to_url(&format!("/getdata/{}", hash));
    let hash_owned = hash.to_string();

    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(text) = resp.text().await {
            if let Ok(data) = serde_json::from_str::<Value>(&text) {
                if let Some(true) = data.get("found").and_then(|v| v.as_bool()) {
                    if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                        if let Some(hash) = data.get("hash").and_then(|v| v.as_str()) {
                            crate::ledger::received_block_from_network(hash, content);
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("[SYNC] Failed to fetch block {}: {}", hash_owned, url);
    }
}

pub fn broadcast_transaction(hash: &str, data: &str) {
    let hash_owned = hash.to_string();
    let data_owned = data.to_string();

    tokio::spawn(async move {
        let peers = get_known_peers();
        let client = http_client();
        let body = json!({ "hash": hash_owned, "data": data_owned }).to_string();

        let futures = peers.into_iter().map(|peer| {
            let client = client.clone();
            let peer_clone = peer.clone();
            let body_clone = body.clone();
            let hash_clone = hash_owned.clone();
            let url = peer.to_url("/inv");

            async move {
                if let Err(e) = client.post(&url)
                    .header("Content-Type", "application/json")
                    .body(body_clone)
                    .send()
                    .await
                {
                    eprintln!("[BROADCAST] Failed to send transaction to {}: {}", url, e);
                } else {
                    println!(
                        "[BROADCAST] Transaction {} sent to {}:{}",
                        hash_clone, peer_clone.ip, peer_clone.port
                    );
                }
            }
        });

        join_all(futures).await;
    });
}


pub fn broadcast_block(hash: &str, content: &str) {
    let hash_owned = hash.to_string();
    let content_owned = content.to_string();

    tokio::spawn(async move {
        let peers = get_known_peers();
        let body = json!({ "hash": hash_owned, "content": content_owned }).to_string();
        
        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let peer_clone = peer.clone();
            let body_clone = body.clone();
            let hash_clone = hash_owned.clone();
            let url = peer.to_url("/block");

            async move {
                if let Err(e) = client.post(&url)
                    .header("Content-Type", "application/json")
                    .body(body_clone)
                    .send()
                    .await
                {
                    eprintln!("[BROADCAST] Failed to send block to {}: {}", url, e);
                } else {
                    println!(
                        "[BROADCAST] Block {} sent to {}:{}",
                        hash_clone, peer_clone.ip, peer_clone.port
                    );
                }
            }
        });

        join_all(futures).await;
    });
}


pub async fn discovery_loop() {
    loop {
        println!("[DISCOVERY] Running peer discovery...");
        discover_peers().await;
        fetch_blocks_from_peers().await;
        sleep(Duration::from_secs(15)).await;
    }
}
