use crate::http_server::{HttpHandler, HttpMethod, HttpRequest, HttpResult};
use crate::ledger;
use crate::peers;
use serde_json::{Value, json};

pub struct NodeHandler;

impl HttpHandler for NodeHandler {
    fn handle(&self, req: HttpRequest) -> HttpResult {
        let HttpRequest { method, body, .. } = req;

        let result = match method {
            HttpMethod::GET(path) if path.starts_with("/addr") => get_addr(),
            HttpMethod::GET(path) if path.starts_with("/getblocks") => get_getblocks(path),
            HttpMethod::GET(path) if path.starts_with("/getdata") => get_getdata(path),

            HttpMethod::POST(path) if path.starts_with("/inv") => post_inv(body),
            HttpMethod::POST(path) if path.starts_with("/block") => post_block(body),

            _ => HttpResult {
                status: 501,
                body: json!({"error": "not implemented"}).to_string(),
                content_type: "application/json",
            },
        };

        result
    }
}

fn get_addr() -> HttpResult {
    HttpResult {
        status: 200,
        body: get_known_peers_json(),
        content_type: "application/json",
    }
}

fn get_known_peers_json() -> String {
    let peers = peers::get_known_peers();
    let peer_list: Vec<Value> = peers
        .iter()
        .map(|p| json!({"ip": p.ip, "port": p.port}))
        .collect();
    json!({"peers": peer_list, "count": peer_list.len()}).to_string()
}

fn get_getblocks(path: String) -> HttpResult {
    let (status, body) = handle_getblocks_request(&path);
    HttpResult {
        status,
        body,
        content_type: "application/json",
    }
}

fn handle_getblocks_request(path: &str) -> (u16, String) {
    if path == "/getblocks" {
        let hashes = ledger::get_all_block_hashes();
        let body = json!({
            "blocks": hashes,
            "count": hashes.len()
        })
        .to_string();

        (200, body)
    } else {
        // Parse hash from path like /getblocks/abc123
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 3 {
            let start_hash = parts[2];
            let hashes = ledger::get_blocks_from(start_hash);
            let body = json!({
                "blocks": hashes,
                "count": hashes.len()
            })
            .to_string();

            (200, body)
        } else {
            let body = json!({
                "error": "Invalid request",
                "blocks": [],
                "count": 0
            })
            .to_string();

            (404, body)
        }
    }
}

fn get_getdata(path: String) -> HttpResult {
    let (status, body) = handle_getdata_request(&path);
    HttpResult {
        status,
        body,
        content_type: "application/json",
    }
}

fn handle_getdata_request(path: &str) -> (u16, String) {
    // Parse hash from path like /getdata/abc123
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() < 3 {
        return (404, json!({"error": "Invalid request"}).to_string());
    }

    let hash = parts[2];

    match ledger::get_block(hash) {
        Some(block) => {
            let body = json!({
                "hash": block.hash,
                "content": block.content,
                "timestamp": block.timestamp,
                "found": true
            })
            .to_string();

            (200, body)
        }
        None => {
            let body = json!({
                "error": "Block not found",
                "hash": hash,
                "found": false
            })
            .to_string();

            (404, body)
        }
    }
}

fn post_inv(body: String) -> HttpResult {
    let (status, body) = handle_inv_request(&body);
    HttpResult {
        status,
        body,
        content_type: "application/json",
    }
}

fn handle_inv_request(body: &str) -> (u16, String) {
    match serde_json::from_str::<Value>(body) {
        Ok(data) => {
            if let (Some(hash), Some(tx_data)) = (
                data.get("hash").and_then(|v| v.as_str()),
                data.get("data").and_then(|v| v.as_str()),
            ) {
                if ledger::add_transaction(hash, tx_data) {
                    // Broadcast to peers
                    crate::peers::broadcast_transaction(hash, tx_data);
                    (200, json!({"message": "Transaction accepted"}).to_string())
                } else {
                    (
                        200,
                        json!({"errcode": 1, "errmsg": "Transaction already exists"}).to_string(),
                    )
                }
            } else {
                (
                    404,
                    json!({"errcode": 2, "errmsg": "Invalid transaction format"}).to_string(),
                )
            }
        }
        Err(e) => (
            500,
            json!({"errcode": 3, "errmsg": format!("JSON parse error: {}", e)}).to_string(),
        ),
    }
}

fn post_block(body: String) -> HttpResult {
    let (status, body) = handle_block_request(&body);
    HttpResult {
        status,
        body,
        content_type: "application/json",
    }
}

fn handle_block_request(body: &str) -> (u16, String) {
    match serde_json::from_str::<Value>(body) {
        Ok(data) => {
            if let (Some(hash), Some(content)) = (
                data.get("hash").and_then(|v| v.as_str()),
                data.get("content").and_then(|v| v.as_str()),
            ) {
                if ledger::add_block(hash, content) {
                    // Broadcast to peers
                    crate::peers::broadcast_block(hash, content);
                    (200, json!({"message": "Block accepted"}).to_string())
                } else {
                    (
                        200,
                        json!({"errcode": 1, "errmsg": "Block already exists"}).to_string(),
                    )
                }
            } else {
                (
                    404,
                    json!({"errcode": 2, "errmsg": "Invalid block format"}).to_string(),
                )
            }
        }
        Err(e) => (
            500,
            json!({"errcode": 3, "errmsg": format!("JSON parse error: {}", e)}).to_string(),
        ),
    }
}
