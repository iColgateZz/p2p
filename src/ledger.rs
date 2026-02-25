use lazy_static::lazy_static;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Block {
    pub hash: String,
    pub prev_hash: String,
    pub transactions: Vec<Transaction>,
    pub timestamp: u64,
}

impl Block {
    pub fn new(prev_hash: String, transactions: Vec<Transaction>, timestamp: u64) -> Self {
        let tx_hashes: String = transactions
            .iter()
            .map(|t| t.hash.clone())
            .collect();

        let input = format!("{}{}{}", prev_hash, tx_hashes, timestamp);
        let hash = compute_hash(&input);

        Self {
            hash,
            prev_hash,
            transactions,
            timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub hash: String,
    pub data: String,
    pub timestamp: u64,
}

impl Transaction {
    pub fn new(data: String, timestamp: u64) -> Self {
        let input = format!("{}{}", data, timestamp);
        let hash = compute_hash(&input);

        Self {
            hash,
            data,
            timestamp,
        }
    }
}

lazy_static! {
    static ref BLOCKS: Mutex<HashMap<String, Block>> = Mutex::new(HashMap::new());
    static ref TRANSACTIONS: Mutex<HashMap<String, Transaction>> = Mutex::new(HashMap::new());
    static ref BLOCK_HASHES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref SEEN_BLOCKS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    static ref SEEN_TRANSACTIONS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

pub fn init_genesis_block() {
    let timestamp = 0;

    let tx = Transaction::new("Alice=100".to_string(), timestamp);

    let block = Block::new(
        String::new(),
        vec![tx],
        timestamp,
    );

    add_block(&block);
    println!("[LEDGER] Genesis block created: {}", block.hash);
}

pub fn compute_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn add_block(block: &Block) -> bool {
    let mut seen = SEEN_BLOCKS.lock().unwrap();

    if seen.contains(&block.hash) {
        return false;
    }

    seen.insert(block.hash.to_string());

    let mut blocks = BLOCKS.lock().unwrap();
    let mut hashes = BLOCK_HASHES.lock().unwrap();

    blocks.insert(block.hash.to_string(), block.clone());
    hashes.push(block.hash.to_string());

    println!("[LEDGER] Added block: {}", block.hash);
    true
}

pub fn add_transaction(tx: &Transaction) -> bool {
    let mut seen = SEEN_TRANSACTIONS.lock().unwrap();

    if seen.contains(&tx.hash) {
        return false;
    }

    seen.insert(tx.hash.to_string());

    let mut transactions = TRANSACTIONS.lock().unwrap();
    transactions.insert(tx.hash.to_string(), tx.clone());

    println!("[LEDGER] Added transaction: {}", tx.hash);
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
