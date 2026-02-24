use crate::http::server::{HttpHandler, HttpMethod, HttpRequest, HttpResult};
use crate::ledger;
use crate::node::client;
use crate::peers;
use serde_json::{Value, json};

pub struct RequestHandler;

impl HttpHandler for RequestHandler {
    fn handle(&self, req: HttpRequest) -> HttpResult {
        let HttpRequest { method, body, .. } = req;

        let result = match method {
            HttpMethod::GET(path) if path.starts_with("/addr") => get_addr(),
            HttpMethod::GET(path) if path.starts_with("/getblocks") => get_getblocks(&path),
            HttpMethod::GET(path) if path.starts_with("/getdata") => get_getdata(&path),

            HttpMethod::POST(path) if path.starts_with("/inv") => post_inv(&body),
            HttpMethod::POST(path) if path.starts_with("/block") => post_block(&body),

            _ => HttpResult::err(501, "not implemented"),
        };

        result
    }
}

fn get_addr() -> HttpResult {
    let peers = peers::get_known_peers();

    let peer_list: Vec<Value> = peers
        .iter()
        .map(|p| json!({ "ip": p.ip, "port": p.port }))
        .collect();

    HttpResult::ok_json(json!({
        "peers": peer_list,
        "count": peer_list.len()
    }))
}

fn get_getblocks(path: &str) -> HttpResult {
    if path == "/getblocks" {
        let hashes = ledger::get_all_block_hashes();
        return HttpResult::ok_json(json!({
            "blocks": hashes,
            "count": hashes.len()
        }));
    }

    match path.split('/').nth(2) {
        Some(start_hash) => {
            let hashes = ledger::get_blocks_from(start_hash);
            HttpResult::ok_json(json!({
                "blocks": hashes,
                "count": hashes.len()
            }))
        }
        None => HttpResult::err(400, "Invalid request"),
    }
}

fn get_getdata(path: &str) -> HttpResult {
    let hash = match path.split('/').nth(2) {
        Some(h) => h,
        None => return HttpResult::err(400, "Invalid request"),
    };

    match ledger::get_block(hash) {
        Some(block) => HttpResult::ok_json(json!({
            "hash": block.hash,
            "content": block.content,
            "timestamp": block.timestamp,
            "found": true
        })),

        None => HttpResult::json(
            404,
            json!({
                "error": "Block not found",
                "hash": hash,
                "found": false
            }),
        ),
    }
}

fn post_inv(body: &str) -> HttpResult {
    let data: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return HttpResult::json(
                400,
                json!({
                    "error": format!("JSON parse error: {}", e)
                }),
            );
        }
    };

    let hash = data.get("hash").and_then(|v| v.as_str());
    let tx_data = data.get("data").and_then(|v| v.as_str());

    match (hash, tx_data) {
        (Some(hash), Some(tx_data)) => {
            if ledger::add_transaction(hash, tx_data) {
                client::broadcast_transaction(hash, tx_data);
                HttpResult::ok_json(json!({ "message": "Transaction accepted" }))
            } else {
                HttpResult::ok_json(json!({
                    "message": "Transaction already exists"
                }))
            }
        }
        _ => HttpResult::json(
            400,
            json!({
                "error": "Invalid transaction format"
            }),
        ),
    }
}

fn post_block(body: &str) -> HttpResult {
    let data: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return HttpResult::json(
                400,
                json!({
                    "error": format!("JSON parse error: {}", e)
                }),
            );
        }
    };

    let hash = data.get("hash").and_then(|v| v.as_str());
    let content = data.get("content").and_then(|v| v.as_str());

    match (hash, content) {
        (Some(hash), Some(content)) => {
            if ledger::add_block(hash, content) {
                client::broadcast_block(hash, content);

                HttpResult::ok_json(json!({
                    "message": "Block accepted"
                }))
            } else {
                HttpResult::ok_json(json!({
                    "message": "Block already exists"
                }))
            }
        }

        _ => HttpResult::json(
            400,
            json!({
                "error": "Invalid block format"
            }),
        ),
    }
}
