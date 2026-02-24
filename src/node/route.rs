use crate::http::server::HttpMethod;

#[derive(Debug, Clone)]
pub enum Route {
    GetPeers,
    GetHashes,
    GetHashesFrom(String),
    GetBlock(String),
    PostBlock,
    PostTransaction,
}

impl Route {
    pub fn to_path(&self) -> String {
        match self {
            Route::GetPeers => "/peers".into(),
            Route::GetHashes => "/hashes".into(),
            Route::GetHashesFrom(hash) => format!("/hashes/{}", hash),
            Route::GetBlock(hash) => format!("/block/{}", hash),
            Route::PostBlock => "/block".into(),
            Route::PostTransaction => "/transaction".into(),
        }
    }

    pub fn parse(method: &HttpMethod) -> Option<Self> {
        match method {
            HttpMethod::GET(path) if path == "/peers" => Some(Route::GetPeers),

            HttpMethod::GET(path) if path == "/hashes" => Some(Route::GetHashes),

            HttpMethod::GET(path) if path.starts_with("/hashes/") => path
                .split('/')
                .nth(2)
                .map(|h| Route::GetHashesFrom(h.to_string())),

            HttpMethod::GET(path) if path.starts_with("/block/") => path
                .split('/')
                .nth(2)
                .map(|h| Route::GetBlock(h.to_string())),

            HttpMethod::POST(path) if path == "/block" => Some(Route::PostBlock),

            HttpMethod::POST(path) if path == "/transaction" => Some(Route::PostTransaction),

            _ => None,
        }
    }
}
