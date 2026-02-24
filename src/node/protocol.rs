use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PeerDto {
    pub ip: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize)]
pub struct PeersDto {
    pub peers: Vec<PeerDto>,
    pub count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct HashesDto {
    pub blocks: Vec<String>,
    pub count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct BlockDto {
    pub hash: String,
    pub content: String,
    pub timestamp: u64,
    pub found: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InvRequest {
    pub hash: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BlockRequest {
    pub hash: String,
    pub content: String,
}
