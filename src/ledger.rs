use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub hash: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub data: String,
    pub timestamp: u64,
}

lazy_static! {
    static ref BLOCKS: Mutex<HashMap<String, Block>> = Mutex::new(HashMap::new());
    static ref TRANSACTIONS: Mutex<HashMap<String, Transaction>> = Mutex::new(HashMap::new());
    static ref BLOCK_HASHES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref SEEN_BLOCKS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    static ref SEEN_TRANSACTIONS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

pub fn compute_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn add_block(hash: &str, content: &str) -> bool {
    let mut seen = SEEN_BLOCKS.lock().unwrap();

    if seen.contains(hash) {
        return false;
    }

    seen.insert(hash.to_string());

    let block = Block {
        hash: hash.to_string(),
        content: content.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let mut blocks = BLOCKS.lock().unwrap();
    let mut hashes = BLOCK_HASHES.lock().unwrap();

    blocks.insert(hash.to_string(), block);
    hashes.push(hash.to_string());

    println!("[LEDGER] Added block: {}", hash);
    true
}

pub fn add_transaction(hash: &str, data: &str) -> bool {
    let mut seen = SEEN_TRANSACTIONS.lock().unwrap();

    if seen.contains(hash) {
        return false;
    }

    seen.insert(hash.to_string());

    let transaction = Transaction {
        hash: hash.to_string(),
        data: data.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let mut transactions = TRANSACTIONS.lock().unwrap();
    transactions.insert(hash.to_string(), transaction);

    println!("[LEDGER] Added transaction: {}", hash);
    true
}

pub fn get_block(hash: &str) -> Option<Block> {
    let blocks = BLOCKS.lock().unwrap();
    blocks.get(hash).cloned()
}

pub fn get_all_block_hashes() -> Vec<String> {
    let hashes = BLOCK_HASHES.lock().unwrap();
    hashes.clone()
}

pub fn get_blocks_from(start_hash: &str) -> Vec<String> {
    let hashes = BLOCK_HASHES.lock().unwrap();

    if let Some(pos) = hashes.iter().position(|h| h == start_hash) {
        hashes[pos..].to_vec()
    } else {
        hashes.clone()
    }
}

pub fn handle_getblocks_request(path: &str) -> String {
    if path == "/getblocks" {
        let hashes = get_all_block_hashes();
        json!({
            "blocks": hashes,
            "count": hashes.len()
        })
        .to_string()
    } else {
        // Parse hash from path like /getblocks/abc123
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 3 {
            let start_hash = parts[2];
            let hashes = get_blocks_from(start_hash);
            json!({
                "blocks": hashes,
                "count": hashes.len()
            })
            .to_string()
        } else {
            json!({
                "error": "Invalid request",
                "blocks": [],
                "count": 0
            })
            .to_string()
        }
    }
}

pub fn handle_getdata_request(path: &str) -> String {
    // Parse hash from path like /getdata/abc123
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() < 3 {
        return json!({"error": "Invalid request"}).to_string();
    }

    let hash = parts[2];

    match get_block(hash) {
        Some(block) => json!({
            "hash": block.hash,
            "content": block.content,
            "timestamp": block.timestamp,
            "found": true
        })
        .to_string(),
        None => json!({
            "error": "Block not found",
            "hash": hash,
            "found": false
        })
        .to_string(),
    }
}

pub fn handle_inv_request(body: &str) -> String {
    match serde_json::from_str::<Value>(body) {
        Ok(data) => {
            if let (Some(hash), Some(tx_data)) = (
                data.get("hash").and_then(|v| v.as_str()),
                data.get("data").and_then(|v| v.as_str()),
            ) {
                if add_transaction(hash, tx_data) {
                    // Broadcast to peers
                    crate::peers::broadcast_transaction(hash, tx_data);
                    json!({"status": 1, "message": "Transaction accepted"}).to_string()
                } else {
                    json!({"status": 0, "errcode": 1, "errmsg": "Transaction already exists"})
                        .to_string()
                }
            } else {
                json!({"status": 0, "errcode": 2, "errmsg": "Invalid transaction format"})
                    .to_string()
            }
        }
        Err(e) => json!({"status": 0, "errcode": 3, "errmsg": format!("JSON parse error: {}", e)})
            .to_string(),
    }
}

pub fn handle_block_request(body: &str) -> String {
    match serde_json::from_str::<Value>(body) {
        Ok(data) => {
            if let (Some(hash), Some(content)) = (
                data.get("hash").and_then(|v| v.as_str()),
                data.get("content").and_then(|v| v.as_str()),
            ) {
                if add_block(hash, content) {
                    // Broadcast to peers
                    crate::peers::broadcast_block(hash, content);
                    json!({"status": 1, "message": "Block accepted"}).to_string()
                } else {
                    json!({"status": 0, "errcode": 1, "errmsg": "Block already exists"}).to_string()
                }
            } else {
                json!({"status": 0, "errcode": 2, "errmsg": "Invalid block format"}).to_string()
            }
        }
        Err(e) => json!({"status": 0, "errcode": 3, "errmsg": format!("JSON parse error: {}", e)})
            .to_string(),
    }
}

pub fn init_genesis_block() {
    let genesis_content = "Genesis Block";
    let genesis_hash = compute_hash(genesis_content);
    add_block(&genesis_hash, genesis_content);
    println!("[LEDGER] Genesis block created: {}", genesis_hash);
}

pub fn received_block_from_network(hash: &str, content: &str) -> bool {
    add_block(hash, content)
}

pub fn received_transaction_from_network(hash: &str, data: &str) -> bool {
    add_transaction(hash, data)
}
