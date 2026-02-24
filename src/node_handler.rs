use crate::http_server::{HttpHandler, HttpMethod, HttpRequest, HttpResult};
use crate::peers;
use serde_json::{Value, json};

pub struct NodeHandler;

impl HttpHandler for NodeHandler {
    fn handle(&self, req: HttpRequest) -> HttpResult {
        let path = req.method.path();

        let body = if path.starts_with("/addr") {
            get_known_peers_json()
        } else if path.starts_with("/getblocks") {
            crate::ledger::handle_getblocks_request(path)
        } else if path.starts_with("/getdata") {
            crate::ledger::handle_getdata_request(path)
        } else if path.starts_with("/inv") && matches!(req.method, HttpMethod::POST(_)) {
            crate::ledger::handle_inv_request(&req.body)
        } else if path.starts_with("/block") && matches!(req.method, HttpMethod::POST(_)) {
            crate::ledger::handle_block_request(&req.body)
        } else {
            json!({"status": "ok", "message": "P2P node active"}).to_string()
        };

        HttpResult {
            status: 200,
            body,
            content_type: "application/json",
        }
    }
}

pub fn get_known_peers_json() -> String {
    let peers = peers::get_known_peers();
    let peer_list: Vec<Value> = peers
        .iter()
        .map(|p| json!({"ip": p.ip, "port": p.port}))
        .collect();
    json!({"peers": peer_list, "count": peer_list.len()}).to_string()
}
