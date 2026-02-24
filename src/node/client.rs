use crate::ledger;
use crate::node::protocol::{
    PeersDto, BlockRequest, GetBlocksResponse, GetDataResponse, InvRequest,
};
use crate::peers::{self, Peer};
use futures::future::join_all;
use reqwest::Client;
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
        let url = peer.to_url("/peers");

        async move {
            match client.get(&url).send().await {
                Ok(r) => {
                    if let Ok(resp) = r.json::<PeersDto>().await {
                        for p in resp.peers {
                            peers::add_peer(p.ip, p.port);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[DISCOVERY] Failed to query {}: {}", url, e);
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
                    if let Ok(data) = resp.json::<GetBlocksResponse>().await {
                        for hash in data.blocks {
                            fetch_block(&peer, &hash).await;
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

async fn fetch_block(peer: &Peer, hash: &str) {
    let client = http_client();
    let url = peer.to_url(&format!("/getdata/{}", hash));

    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(data) = resp.json::<GetDataResponse>().await {
            if data.found {
                ledger::add_block(&data.hash, &data.content);
            }
        }
    }
}

pub fn broadcast_transaction(hash: &str, data: &str) {
    let req = InvRequest {
        hash: hash.to_string(),
        data: data.to_string(),
    };

    tokio::spawn(async move {
        let peers = peers::get_known_peers();

        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let url = peer.to_url("/inv");
            let req = req.clone();

            async move {
                let _ = client.post(&url).json(&req).send().await;
            }
        });

        join_all(futures).await;
    });
}

pub fn broadcast_block(hash: &str, content: &str) {
    let req = BlockRequest {
        hash: hash.to_string(),
        content: content.to_string(),
    };

    tokio::spawn(async move {
        let peers = peers::get_known_peers();

        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let url = peer.to_url("/block");
            let req = req.clone();

            async move {
                let _ = client.post(&url).json(&req).send().await;
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
