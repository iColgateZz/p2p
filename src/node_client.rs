use crate::peers::{self, PeerInfo};
use futures::future::join_all;
use reqwest::Client;
use serde_json::{Value, json};
use std::sync::OnceLock;
use tokio::time::{Duration, sleep};

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_max_idle_per_host(256)
            .build()
            .unwrap()
    })
}

pub async fn discover_peers() {
    let peers = peers::get_known_peers();

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
                                        peers::add_peer(ip.to_string(), port as u16);
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
    let peers = peers::get_known_peers();

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
        let peers = peers::get_known_peers();

        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let body = json!({ "hash": hash_owned, "data": data_owned }).to_string();
            let hash_clone = hash_owned.clone();
            let url = peer.to_url("/inv");

            async move {
                if let Err(e) = client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await
                {
                    eprintln!("[BROADCAST] Failed to send transaction to {}: {}", url, e);
                } else {
                    println!(
                        "[BROADCAST] Transaction {} sent to {}:{}",
                        hash_clone, peer.ip, peer.port
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
        let peers = peers::get_known_peers();

        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let body = json!({ "hash": hash_owned, "content": content_owned }).to_string();
            let hash_clone = hash_owned.clone();
            let url = peer.to_url("/block");

            async move {
                if let Err(e) = client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await
                {
                    eprintln!("[BROADCAST] Failed to send block to {}: {}", url, e);
                } else {
                    println!(
                        "[BROADCAST] Block {} sent to {}:{}",
                        hash_clone, peer.ip, peer.port
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
