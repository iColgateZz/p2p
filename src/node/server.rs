use crate::http::server::{HttpHandler, HttpRequest, HttpResult};
use crate::ledger::{self, Block, Transaction};
use crate::node::protocol::*;
use crate::node::{client, route::Route};
use crate::peers;

pub struct RequestHandler;

impl HttpHandler for RequestHandler {
    fn handle(&self, req: HttpRequest) -> HttpResult {
        let HttpRequest { method, body, .. } = req;

        let Some(route) = Route::parse(&method) else {
            return HttpResult::not_impl();
        };

        match route {
            Route::GetPeers => get_peers(),
            Route::GetHashes => get_hashes(),
            Route::GetHashesAfter(hash) => get_hashes_after(&hash),
            Route::GetBlock(hash) => get_block(&hash),
            Route::PostBlock => post_block(&body),
            Route::PostTransaction => post_transaction(&body),
            Route::PostUsers => post_users(&body),
            Route::PostTransfers => post_transfers(&body),
        }
    }
}

fn get_peers() -> HttpResult {
    let peers = peers::select_random_peers();

    let peer_list: Vec<PeerDto> = peers
        .into_iter()
        .map(|p| PeerDto {
            ip: p.ip,
            port: p.port,
        })
        .collect();

    HttpResult::ok(&PeersDto { peers: peer_list })
}

fn get_hashes() -> HttpResult {
    let hashes = ledger::get_all_block_hashes();
    HttpResult::ok(&HashesDto { hashes })
}

fn get_hashes_after(start_hash: &str) -> HttpResult {
    let hashes = ledger::get_block_hashes_after(start_hash);
    HttpResult::ok(&HashesDto { hashes })
}

fn get_block(hash: &str) -> HttpResult {
    match ledger::get_block(hash) {
        Some(block) => HttpResult::ok(&BlockDto::from(&block)),
        None => HttpResult::not_found(),
    }
}

fn post_transaction(body: &str) -> HttpResult {
    let dto: TransactionDto = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => {
            return HttpResult::bad_req();
        }
    };

    if ledger::add_transaction(&Transaction::from(&dto)) {
        client::broadcast_transaction(dto);
        HttpResult::created(&Message {
            message: "Transaction accepted",
        })
    } else {
        HttpResult::ok(&Message {
            message: "Transaction already exists",
        })
    }
}

fn post_block(body: &str) -> HttpResult {
    let dto: BlockDto = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => {
            return HttpResult::bad_req();
        }
    };

    if ledger::add_block(&Block::from(&dto)) {
        client::broadcast_block(dto);
        HttpResult::created(&Message {
            message: "Block accepted",
        })
    } else {
        HttpResult::ok(&Message {
            message: "Block already exists or its hash does not match the hash of the last block in the chain",
        })
    }
}

fn post_users(body: &str) -> HttpResult {
    let dto: UserDto = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => {
            return HttpResult::bad_req();
        }
    };

    let data = format!("{}={}", dto.name, dto.balance);
    let tx = Transaction::from_data(data);
    ledger::add_transaction(&tx);
    client::broadcast_transaction(TransactionDto::from(&tx));

    HttpResult::created(&Message {
        message: "User added",
    })
}

fn post_transfers(body: &str) -> HttpResult {
    let dto: TransferDto = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => {
            return HttpResult::bad_req();
        }
    };

    let data = format!("{}->{}:{}", dto.from, dto.to, dto.sum);
    let tx = Transaction::from_data(data);
    ledger::add_transaction(&tx);
    client::broadcast_transaction(TransactionDto::from(&tx));

    HttpResult::created(&Message {
        message: "Transfer accepted",
    })
}
