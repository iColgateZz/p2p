use crate::ledger::{self, Block};
use crate::node::protocol::{BlockDto, HashesDto, PeersDto, TransactionDto};
use crate::node::route::Route;
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
    let peers = peers::select_random_peers();

    let futures = peers.into_iter().map(|peer| {
        let client = http_client();
        let url = peer.to_url(&Route::GetPeers.to_path());

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
    let peers = peers::select_random_peers();

    let futures = peers.into_iter().map(|peer| {
        let client = http_client();
        let url = peer.to_url(&Route::GetHashes.to_path());

        async move {
            println!("[SYNC] Fetching blocks from: {}", url);

            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<HashesDto>().await {
                        for hash in data.hashes {
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
    let url = peer.to_url(&Route::GetBlock(hash.into()).to_path());

    let Ok(resp) = client.get(&url).send().await else {
        return;
    };

    if !resp.status().is_success() {
        return;
    }

    if let Ok(block) = resp.json::<BlockDto>().await {
        ledger::add_block(&Block::from(&block));
    }
}

pub fn broadcast_transaction(req: TransactionDto) {
    tokio::spawn(async move {
        let peers = peers::select_random_peers();

        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let url = peer.to_url(&Route::PostTransaction.to_path());
            let req = req.clone();

            async move {
                let _ = client.post(&url).json(&req).send().await;
            }
        });

        join_all(futures).await;
    });
}

pub fn broadcast_block(req: BlockDto) {
    tokio::spawn(async move {
        let peers = peers::select_random_peers();

        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let url = peer.to_url(&Route::PostBlock.to_path());
            let req = req.clone();

            async move {
                let _ = client.post(&url).json(&req).send().await;
            }
        });

        join_all(futures).await;
    });
}

pub async fn peer_discovery_loop() {
    loop {
        discover_peers().await;
        sleep(Duration::from_secs(30)).await;
    }
}

pub async fn block_sync_loop() {
    loop {
        fetch_blocks_from_peers().await;
        sleep(Duration::from_secs(15)).await;
    }
}

pub async fn block_creation_loop() {
    loop {
        sleep(Duration::from_secs(60)).await;

        let pending = ledger::take_pending_transactions();
        if pending.is_empty() {
            continue;
        }

        let prev_hash = ledger::last_block_hash();
        let timestamp = ledger::now();
        let block = Block::new(prev_hash, pending, timestamp);

        ledger::add_block(&block);
        broadcast_block(BlockDto::from(&block));
    }
}
