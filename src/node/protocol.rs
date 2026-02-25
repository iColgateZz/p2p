use serde::{Deserialize, Serialize};

use crate::ledger::{Block, Transaction};

#[derive(Serialize, Deserialize)]
pub struct PeerDto {
    pub ip: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize)]
pub struct PeersDto {
    pub peers: Vec<PeerDto>,
}

#[derive(Serialize, Deserialize)]
pub struct HashesDto {
    pub hashes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BlockDto {
    pub hash: String,
    pub prev_hash: String,
    pub transactions: Vec<TransactionDto>,
    pub timestamp: u64,
}

impl From<&Block> for BlockDto {
    fn from(b: &Block) -> Self {
        BlockDto {
            hash: b.hash.clone(),
            prev_hash: b.prev_hash.clone(),
            transactions: b.transactions.iter().map(|t| t.into()).collect(),
            timestamp: b.timestamp,
        }
    }
}

impl From<&BlockDto> for Block {
    fn from(dto: &BlockDto) -> Self {
        Block {
            hash: dto.hash.clone(),
            prev_hash: dto.prev_hash.clone(),
            transactions: dto.transactions.iter().map(Into::into).collect(),
            timestamp: dto.timestamp,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TransactionDto {
    pub hash: String,
    pub data: String,
    pub timestamp: u64,
}

impl From<&Transaction> for TransactionDto {
    fn from(tx: &Transaction) -> Self {
        TransactionDto {
            hash: tx.hash.clone(),
            data: tx.data.clone(),
            timestamp: tx.timestamp,
        }
    }
}

impl From<&TransactionDto> for Transaction {
    fn from(dto: &TransactionDto) -> Self {
        Transaction {
            hash: dto.hash.clone(),
            data: dto.data.clone(),
            timestamp: dto.timestamp,
        }
    }
}

#[derive(Serialize)]
pub struct Message<'a> {
    pub message: &'a str,
}
