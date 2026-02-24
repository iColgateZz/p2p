use crate::http::server::{HttpHandler, HttpMethod, HttpRequest, HttpResult};
use crate::ledger;
use crate::node::{
    client,
    protocol::{BlockDto, PeerDto, TransactionDto},
};
use crate::peers;
use serde_json::json;

pub struct RequestHandler;

impl HttpHandler for RequestHandler {
    fn handle(&self, req: HttpRequest) -> HttpResult {
        let HttpRequest { method, body, .. } = req;

        let result = match method {
            HttpMethod::GET(path) if path.starts_with("/peers") => get_peers(),
            HttpMethod::GET(path) if path.starts_with("/hashes") => get_hashes(&path),
            HttpMethod::GET(path) if path.starts_with("/block") => get_block(&path),

            HttpMethod::POST(path) if path.starts_with("/transaction") => post_transaction(&body),
            HttpMethod::POST(path) if path.starts_with("/block") => post_block(&body),

            _ => HttpResult::err(501, "not implemented"),
        };

        result
    }
}

fn get_peers() -> HttpResult {
    let peers = peers::get_known_peers();

    let peer_list: Vec<PeerDto> = peers
        .into_iter()
        .map(|p| PeerDto { ip: p.ip, port: p.port, })
        .collect();

    HttpResult::ok_json(json!({
        "peers": peer_list,
    }))
}

fn get_hashes(path: &str) -> HttpResult {
    if path == "/hashes" {
        let hashes = ledger::get_all_block_hashes();
        return HttpResult::ok_json(json!({
            "blocks": hashes,
            "count": hashes.len()
        }));
    }

    match path.split('/').nth(2) {
        Some(start_hash) => {
            let hashes = ledger::get_block_hashes_from(start_hash);
            HttpResult::ok_json(json!({
                "blocks": hashes,
                "count": hashes.len()
            }))
        }
        None => HttpResult::err(400, "Invalid request"),
    }
}

fn get_block(path: &str) -> HttpResult {
    let hash = match path.split('/').nth(2) {
        Some(h) => h,
        None => return HttpResult::err(400, "Invalid request"),
    };

    match ledger::get_block(hash) {
        Some(block) => HttpResult::ok_json(json!({
            "hash": block.hash,
            "content": block.content,
            "timestamp": block.timestamp,
        })),

        None => HttpResult::json(
            404,
            json!({
                "error": "Block not found",
                "hash": hash,
            }),
        ),
    }
}

fn post_transaction(body: &str) -> HttpResult {
    let transaction: TransactionDto = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return HttpResult::err(400, &format!("JSON parse error: {}", e));
        }
    };

    if ledger::add_transaction(&transaction.hash, &transaction.data) {
        client::broadcast_transaction(&transaction.hash, &transaction.data);
        HttpResult::ok_json(json!({"message": "Transaction accepted"}))
    } else {
        HttpResult::ok_json(json!({"message": "Transaction already exists"}))
    }
}

fn post_block(body: &str) -> HttpResult {
    let block: BlockDto = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return HttpResult::err(400, &format!("JSON parse error: {}", e));
        }
    };

    if ledger::add_block(&block.hash, &block.content) {
        client::broadcast_block(&block.hash, &block.content);
        HttpResult::ok_json(json!({"message": "Block accepted"}))
    } else {
        HttpResult::ok_json(json!({"message": "Block already exists"}))
    }
}
