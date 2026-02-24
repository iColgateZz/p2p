use serde::{Deserialize, Serialize};

//
// --- Peer / Discovery ---
//

#[derive(Serialize, Deserialize)]
pub struct PeerDto {
    pub ip: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize)]
pub struct AddrResponse {
    pub peers: Vec<PeerDto>,
    pub count: usize,
}

//
// --- Blocks ---
//

#[derive(Serialize, Deserialize)]
pub struct GetBlocksResponse {
    pub blocks: Vec<String>,
    pub count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct GetDataResponse {
    pub hash: String,
    pub content: String,
    pub timestamp: u64,
    pub found: bool,
}

//
// --- Transactions ---
//

#[derive(Serialize, Deserialize)]
pub struct InvRequest {
    pub hash: String,
    pub data: String,
}

#[derive(Serialize, Deserialize)]
pub struct BlockRequest {
    pub hash: String,
    pub content: String,
}
