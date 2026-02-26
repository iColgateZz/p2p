use crate::ledger::{self, Block};
use crate::node::protocol::{BlockDto, HashesDto, PeerDto, TransactionDto};
use crate::node::route::Route;
use crate::peers::{self, Peer, update_peer};
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
                    if let Ok(resp) = r.json::<Vec<PeerDto>>().await {
                        for p in resp {
                            peers::add_peer(p.ip, p.port);
                        }
                    }
                }
                Err(_) => update_peer(peer),
            }
        }
    });

    join_all(futures).await;
}

pub async fn fetch_blocks_from_peers() {
    let peers = peers::select_random_peers();
    let my_last_hash = ledger::last_block_hash();

    let futures = peers.into_iter().map(|peer| {
        let client = http_client();
        let route = Route::GetHashesAfter(my_last_hash.clone());
        let url = peer.to_url(&route.to_path());

        async move {
            // println!("[SYNC] Querying hashes from: {}", url);

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

pub fn broadcast_transaction(tx: TransactionDto) {
    tokio::spawn(async move {
        let peers = peers::select_random_peers();

        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let url = peer.to_url(&Route::PostTransaction.to_path());
            let tx = tx.clone();

            async move {
                let _ = client.post(&url).json(&tx).send().await;
            }
        });

        join_all(futures).await;
    });
}

pub fn broadcast_block(block: BlockDto) {
    tokio::spawn(async move {
        let peers = peers::select_random_peers();

        let futures = peers.into_iter().map(|peer| {
            let client = http_client();
            let url = peer.to_url(&Route::PostBlock.to_path());
            let block = block.clone();

            async move {
                let _ = client.post(&url).json(&block).send().await;
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
