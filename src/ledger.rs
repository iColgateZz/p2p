use lazy_static::lazy_static;
use sha2::{Digest, Sha256};
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
        let tx_hashes: String = transactions.iter().map(|t| t.hash.clone()).collect();

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

    pub fn from_data(data: String) -> Self {
        let timestamp = now();
        Self::new(data, timestamp)
    }
}

lazy_static! {
    static ref BLOCKS: Mutex<Vec<Block>> = Mutex::new(Vec::new());
    static ref PENDING_TRANSACTIONS: Mutex<Vec<Transaction>> = Mutex::new(Vec::new());
}

pub fn init_genesis_block() {
    let timestamp = 0;

    let tx = Transaction::new("Alice=100".to_string(), timestamp);
    let block = Block::new(String::new(), vec![tx], timestamp);

    let mut blocks = BLOCKS.lock().unwrap();
    blocks.push(block);
}

pub fn compute_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn add_block(block: &Block) -> bool {
    let mut blocks = BLOCKS.lock().unwrap();
    if blocks.iter().any(|b| b.hash == block.hash) {
        return false;
    }

    // we always have at least the genesis block
    let last_block = blocks.last().unwrap();
    if last_block.hash != block.prev_hash {
        return false;
    }

    blocks.push(block.clone());
    drop(blocks); // release lock early

    // Remove confirmed transactions from pending
    let mut pending = PENDING_TRANSACTIONS.lock().unwrap();
    pending.retain(|pending_tx| {
        !block
            .transactions
            .iter()
            .any(|tx| tx.hash == pending_tx.hash)
    });

    println!("[LEDGER] Added block: {}", block.hash);
    true
}

pub fn add_transaction(transaction: &Transaction) -> bool {
    let mut transactions = PENDING_TRANSACTIONS.lock().unwrap();
    for tx in transactions.iter() {
        if tx.hash == transaction.hash {
            return false;
        }
    }

    transactions.push(transaction.clone());
    println!("[LEDGER] Added transaction: {}", transaction.hash);
    true
}

pub fn take_pending_transactions() -> Vec<Transaction> {
    let mut pending = PENDING_TRANSACTIONS.lock().unwrap();
    let txs = pending.clone();
    pending.clear();
    txs
}

pub fn pending_txs_len() -> usize {
    let pending = PENDING_TRANSACTIONS.lock().unwrap();
    pending.len()
}

pub fn last_block_hash() -> String {
    let blocks = BLOCKS.lock().unwrap();
    blocks.last().unwrap().hash.clone()
}

pub fn get_block(hash: &str) -> Option<Block> {
    let blocks = BLOCKS.lock().unwrap();
    for b in blocks.iter() {
        if b.hash == hash {
            return Some(b.clone());
        }
    }

    None
}

pub fn get_blocks_copy() -> Vec<Block> {
    let blocks = BLOCKS.lock().unwrap();
    blocks.to_vec()
}

pub fn chain_len() -> usize {
    let blocks = BLOCKS.lock().unwrap();
    blocks.len()
}

pub fn get_all_block_hashes() -> Vec<String> {
    let mut v = Vec::new();

    for b in BLOCKS.lock().unwrap().iter() {
        v.push(b.hash.clone());
    }

    v
}

pub fn get_block_hashes_after(start_hash: &str) -> Vec<String> {
    let blocks = BLOCKS.lock().unwrap();

    if let Some(pos) = blocks.iter().position(|b| b.hash == start_hash) {
        blocks
            .iter()
            .skip(pos + 1) // skip the matching hash
            .map(|b| b.hash.clone())
            .collect()
    } else {
        Vec::new()
    }
}
