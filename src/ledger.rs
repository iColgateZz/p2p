use lazy_static::lazy_static;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Block {
    pub hash: String,
    pub prev_hash: String,
    pub transactions: Vec<Transaction>,
    pub timestamp: u64,
    pub nonce: u64,
}

pub const MINING_COMPLEXITY: usize = 5;

impl Block {
    pub fn new(prev_hash: String, transactions: Vec<Transaction>, timestamp: u64) -> Self {
        let (nonce, hash) = Self::mine(&prev_hash, &transactions, timestamp);

        Self {
            hash,
            prev_hash,
            transactions,
            timestamp,
            nonce,
        }
    }

    pub fn mine(prev_hash: &str, transactions: &[Transaction], timestamp: u64) -> (u64, String) {
        let mut nonce = 0;
        let tx_hashes: String = transactions.iter().map(|t| t.hash.as_str()).collect();

        loop {
            let hash = compute_hash(&format!("{}{}{}{}", prev_hash, tx_hashes, timestamp, nonce));

            if Self::has_valid_prefix(&hash) {
                return (nonce, hash)
            }

            nonce += 1;
        }
    }

    pub fn has_valid_prefix(hash: &str) -> bool {
        hash.starts_with(&"0".repeat(MINING_COMPLEXITY))
    }

    /// Check that the hash inside the block actually matches the content
    pub fn is_valid(&self) -> bool {
        let tx_hashes: String = self.transactions.iter().map(|t| t.hash.as_str()).collect();
        let expected = compute_hash(&format!("{}{}{}{}", self.prev_hash, tx_hashes, self.timestamp, self.nonce));

        self.hash == expected && Self::has_valid_prefix(&self.hash)
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

#[derive(Debug, Clone)]
struct StoredBlock {
    block: Block,
    height: usize,
}

#[derive(Debug, Default)]
struct LedgerState {
    blocks_by_hash: HashMap<String, StoredBlock>,
    main_chain: Vec<Block>,
    best_tip: String,
}

#[derive(Debug, Default)]
struct TxPool {
    known_by_hash: HashMap<String, Transaction>,
}

#[derive(Debug)]
pub enum AddBlockResult {
    Added,
    StoredAsOrphan,
    Duplicate,
    Invalid,
}

lazy_static! {
    static ref LEDGER: Mutex<LedgerState> = Mutex::new(LedgerState::default());
    static ref TX_POOL: Mutex<TxPool> = Mutex::new(TxPool::default());
    static ref ORPHAN_BLOCKS: Mutex<HashMap<String, Vec<Block>>> = Mutex::new(HashMap::new());
}

pub fn init_genesis_block() {
    let timestamp = 0;

    let tx = Transaction::new("Alice=100".to_string(), timestamp);
    let block = Block::new(String::new(), vec![tx], timestamp);

    let mut ledger = LEDGER.lock().unwrap();
    let height = 1;
    ledger.blocks_by_hash.insert(
        block.hash.clone(),
        StoredBlock {
            block: block.clone(),
            height,
        },
    );
    ledger.best_tip = block.hash.clone();
    ledger.main_chain = vec![block];
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

pub fn add_block(block: &Block) -> AddBlockResult {
    if !block.is_valid() {
        return AddBlockResult::Invalid;
    }

    remember_block_transactions(block);

    {
        let ledger = LEDGER.lock().unwrap();
        if ledger.blocks_by_hash.contains_key(&block.hash) {
            return AddBlockResult::Duplicate;
        }
    }

    if !block.prev_hash.is_empty() {
        let parent_known = {
            let ledger = LEDGER.lock().unwrap();
            ledger.blocks_by_hash.contains_key(&block.prev_hash)
        };

        if !parent_known {
            let mut orphans = ORPHAN_BLOCKS.lock().unwrap();
            let entry = orphans.entry(block.prev_hash.clone()).or_default();
            if entry.iter().any(|b| b.hash == block.hash) {
                return AddBlockResult::Duplicate;
            }
            entry.push(block.clone());
            println!(
                "[LEDGER] Stored orphan block {} waiting for {}",
                block.hash, block.prev_hash
            );
            return AddBlockResult::StoredAsOrphan;
        }
    }

    insert_block_and_update_best_chain(block.clone());
    process_orphans(block.hash.clone());

    println!("[LEDGER] Added block: {}", block.hash);
    AddBlockResult::Added
}

fn insert_block_and_update_best_chain(block: Block) {
    let mut ledger = LEDGER.lock().unwrap();

    if ledger.blocks_by_hash.contains_key(&block.hash) {
        return;
    }

    let height = if block.prev_hash.is_empty() {
        1
    } else {
        match ledger.blocks_by_hash.get(&block.prev_hash) {
            Some(parent) => parent.height + 1,
            None => return,
        }
    };

    ledger.blocks_by_hash.insert(
        block.hash.clone(),
        StoredBlock {
            block: block.clone(),
            height,
        },
    );

    let current_best_height = ledger
        .blocks_by_hash
        .get(&ledger.best_tip)
        .map(|b| b.height)
        .unwrap_or(0);

    if height > current_best_height {
        ledger.best_tip = block.hash.clone();
        rebuild_main_chain(&mut ledger);
    }
}

fn rebuild_main_chain(ledger: &mut LedgerState) {
    let mut chain = Vec::new();
    let mut cursor = ledger.best_tip.clone();

    while !cursor.is_empty() {
        let Some(stored) = ledger.blocks_by_hash.get(&cursor) else {
            break;
        };

        chain.push(stored.block.clone());
        cursor = stored.block.prev_hash.clone();
    }

    chain.reverse();
    ledger.main_chain = chain;
}

fn process_orphans(starting_parent_hash: String) {
    let mut queue = vec![starting_parent_hash];
    let mut seen_parents = HashSet::new();

    while let Some(parent_hash) = queue.pop() {
        if !seen_parents.insert(parent_hash.clone()) {
            continue;
        }

        let children = {
            let mut orphans = ORPHAN_BLOCKS.lock().unwrap();
            orphans.remove(&parent_hash).unwrap_or_default()
        };

        for child in children {
            let child_hash = child.hash.clone();
            insert_block_and_update_best_chain(child);
            queue.push(child_hash);
        }
    }
}

fn confirmed_tx_hashes(ledger: &LedgerState) -> HashSet<String> {
    ledger
        .main_chain
        .iter()
        .flat_map(|block| block.transactions.iter().map(|tx| tx.hash.clone()))
        .collect()
}

fn remember_block_transactions(block: &Block) {
    let mut pool = TX_POOL.lock().unwrap();
    for tx in &block.transactions {
        pool.known_by_hash
            .entry(tx.hash.clone())
            .or_insert_with(|| tx.clone());
    }
}

pub fn add_transaction(transaction: &Transaction) -> bool {
    let mut pool = TX_POOL.lock().unwrap();
    if pool.known_by_hash.contains_key(&transaction.hash) {
        return false;
    }

    pool.known_by_hash
        .insert(transaction.hash.clone(), transaction.clone());

    println!("[LEDGER] Added transaction: {}", transaction.hash);
    true
}

pub fn get_pending_transactions() -> Vec<Transaction> {
    let ledger = LEDGER.lock().unwrap();
    let confirmed = confirmed_tx_hashes(&ledger);
    drop(ledger);

    let pool = TX_POOL.lock().unwrap();
    let mut pending: Vec<Transaction> = pool
        .known_by_hash
        .values()
        .filter(|tx| !confirmed.contains(&tx.hash))
        .cloned()
        .collect();

    pending.sort_by_key(|tx| tx.timestamp);
    pending
}

pub fn get_transactions_for_mining(limit: usize) -> Vec<Transaction> {
    let mut pending = get_pending_transactions();
    pending.truncate(limit);
    pending
}

pub fn pending_txs_len() -> usize {
    get_pending_transactions().len()
}

pub fn last_block_hash() -> String {
    let ledger = LEDGER.lock().unwrap();
    ledger.best_tip.clone()
}

pub fn get_block(hash: &str) -> Option<Block> {
    let ledger = LEDGER.lock().unwrap();
    ledger.blocks_by_hash.get(hash).map(|b| b.block.clone())
}

pub fn with_blocks<R>(f: impl FnOnce(&[Block]) -> R) -> R {
    let ledger = LEDGER.lock().unwrap();
    f(&ledger.main_chain)
}

pub fn chain_len() -> usize {
    let ledger = LEDGER.lock().unwrap();
    ledger.main_chain.len()
}

pub fn get_all_block_hashes() -> Vec<String> {
    let ledger = LEDGER.lock().unwrap();
    ledger.main_chain.iter().map(|b| b.hash.clone()).collect()
}

pub fn get_block_hashes_after(start_hash: &str) -> Vec<String> {
    let ledger = LEDGER.lock().unwrap();

    if let Some(pos) = ledger.main_chain.iter().position(|b| b.hash == start_hash) {
        ledger
            .main_chain
            .iter()
            .skip(pos + 1)
            .map(|b| b.hash.clone())
            .collect()
    } else {
        Vec::new()
    }
}
