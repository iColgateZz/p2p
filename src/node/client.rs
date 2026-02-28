use crate::ledger::{self, Block};
use crate::node::RUNTIME;
use crate::node::protocol::{BlockDto, HashesDto, PeerDto, TransactionDto};
use crate::node::route::Route;
use crate::peers::{self, Peer, update_peer};
use reqwest::Client;
use serde::Serialize;
use std::sync::OnceLock;
use tokio::task::JoinSet;
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

async fn post_json_with_length<T: Serialize>(client: &Client, url: &str, value: &T) {
    let body = match serde_json::to_string(value) {
        Ok(b) => b,
        Err(_) => return,
    };

    let _ = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Content-Length", body.len())
        .body(body)
        .send()
        .await;
}

pub async fn discover_peers() {
    let peers = peers::select_random_peers();
    let mut set = JoinSet::new();
    let client = http_client();

    for peer in peers {
        let url = peer.to_url(&Route::GetPeers.to_path());

        set.spawn(async move {
            match client.get(&url).send().await {
                Ok(r) => {
                    if let Ok(resp) = r.json::<Vec<PeerDto>>().await {
                        for p in resp {
                            peers::add_peer(p.ip, p.port);
                        }
                    }
                }
                Err(_) => update_peer(peer),
            }
        });
    }

    while let Some(_) = set.join_next().await {}
}

pub async fn fetch_blocks_from_peers() {
    let peers = peers::select_random_peers();
    let my_last_hash = ledger::last_block_hash();
    let client = http_client();
    let route = Route::GetHashesAfter(my_last_hash);
    let mut set = JoinSet::new();

    for peer in peers {
        let url = peer.to_url(&route.to_path());

        set.spawn(async move {
            let Ok(resp) = client.get(&url).send().await else {
                return;
            };

            let Ok(data) = resp.json::<HashesDto>().await else {
                return;
            };

            //TODO: actually one peer should be enough to fetch all blocks,
            //      no need to ask other peers.
            //TODO: optimization - ask many peers simultaneously about
            //      different missing blocks and enqueue them
            for hash in data.hashes {
                fetch_block(&peer, &hash).await;
            }
        });
    }

    while set.join_next().await.is_some() {}
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

pub fn broadcast_transaction(tx: TransactionDto) {
    RUNTIME.spawn(async move {
        let peers = peers::select_random_peers();
        let client = http_client();
        let mut set = JoinSet::new();

        for peer in peers {
            let url = peer.to_url(&Route::PostTransaction.to_path());
            let tx = tx.clone();

            set.spawn(async move {
                post_json_with_length(client, &url, &tx).await;
            });
        }

        while set.join_next().await.is_some() {}
    });
}

pub fn broadcast_block(block: BlockDto) {
    RUNTIME.spawn(async move {
        let peers = peers::select_random_peers();
        let client = http_client();
        let mut set = JoinSet::new();

        for peer in peers {
            let url = peer.to_url(&Route::PostBlock.to_path());
            let block = block.clone();

            set.spawn(async move {
                post_json_with_length(client, &url, &block).await;
            });
        }

        while set.join_next().await.is_some() {}
    });
}
pub async fn broadcast_self() {
    RUNTIME.spawn(async move {
        let peers = peers::select_random_peers();
        let client = http_client();
        let mut set = JoinSet::new();
        let xself = peers::self_peer();

        for peer in peers {
            let url = peer.to_url(&Route::PostPeers.to_path());
            set.spawn(async move {
                post_json_with_length(client, &url, &PeerDto::from(xself)).await;
            });
        }

        while set.join_next().await.is_some() {}
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

pub async fn advertisement_loop() {
    loop {
        broadcast_self().await;
        sleep(Duration::from_secs(15)).await;
    }
}
