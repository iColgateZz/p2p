use lazy_static::lazy_static;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Block {
    pub hash: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
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

pub fn init_genesis_block() {
    let genesis_content = "Alice=100";
    let genesis_hash = compute_hash(genesis_content);
    add_block(&genesis_hash, genesis_content);
    println!("[LEDGER] Genesis block created: {}", genesis_hash);
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

pub fn get_block_hashes_from(start_hash: &str) -> Vec<String> {
    let hashes = BLOCK_HASHES.lock().unwrap();

    if let Some(pos) = hashes.iter().position(|h| h == start_hash) {
        hashes[pos..].to_vec()
    } else {
        hashes.clone()
    }
}
