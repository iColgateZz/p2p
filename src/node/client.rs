use crate::ledger::{self, AddBlockResult, Block};
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
    let client = http_client();
    let mut set = JoinSet::new();

    for peer in peers {
        set.spawn(async move {
            let hashes_url = peer.to_url(&Route::GetHashes.to_path());
            let Ok(resp) = client.get(&hashes_url).send().await else {
                return;
            };

            let Ok(data) = resp.json::<HashesDto>().await else {
                return;
            };

            sync_with_peer_chain(&peer, data.hashes).await;
        });
    }

    while set.join_next().await.is_some() {}
}

async fn sync_with_peer_chain(peer: &Peer, peer_hashes: Vec<String>) {
    let local_hashes = ledger::get_all_block_hashes();

    if peer_hashes.len() <= local_hashes.len() {
        return;
    }

    let mut common_prefix_len = 0usize;
    while common_prefix_len < local_hashes.len()
        && common_prefix_len < peer_hashes.len()
        && local_hashes[common_prefix_len] == peer_hashes[common_prefix_len]
    {
        common_prefix_len += 1;
    }

    for hash in peer_hashes.into_iter().skip(common_prefix_len) {
        fetch_block(peer, &hash).await;
    }
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
        sleep(Duration::from_secs(30)).await;
    }
}

pub async fn sync_transactions_from_peers() {
    let peers = peers::select_random_peers();
    let client = http_client();
    let mut set = JoinSet::new();

    for peer in peers {
        set.spawn(async move {
            let url = peer.to_url(&Route::GetTransactions.to_path());
            let Ok(resp) = client.get(&url).send().await else {
                return;
            };

            let Ok(txs) = resp.json::<Vec<TransactionDto>>().await else {
                return;
            };

            for tx in &txs {
                let t = tx.into();
                ledger::add_transaction(&t);
            }
        });
    }

    while set.join_next().await.is_some() {}
}

pub async fn transactions_sync_loop() {
    loop {
        sync_transactions_from_peers().await;
        sleep(Duration::from_secs(15)).await;
    }
}

pub async fn block_creation_loop() {
    loop {
        sleep(Duration::from_secs(60)).await;

        let pending = ledger::get_transactions_for_mining(1000);
        if pending.is_empty() {
            continue;
        }

        let prev_hash = ledger::last_block_hash();
        let timestamp = ledger::now();
        let block = Block::new(prev_hash, pending, timestamp);

        if matches!(ledger::add_block(&block), AddBlockResult::Added) {
            broadcast_block(BlockDto::from(&block));
        }
    }
}

pub async fn advertisement_loop() {
    loop {
        broadcast_self().await;
        sleep(Duration::from_secs(15)).await;
    }
}
