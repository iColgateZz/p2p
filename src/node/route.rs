use crate::http::server::HttpMethod;

#[derive(Debug, Clone)]
pub enum Route {
    GetPeers,
    GetHashes,
    GetHashesAfter(String),
    GetBlock(String),
    PostBlock,
    PostTransaction,
    GetUsers,
    PostUsers,
    PostTransfers,
}

impl Route {
    pub fn to_path(&self) -> String {
        match self {
            Route::GetPeers => "/peers".into(),
            Route::GetHashes => "/hashes".into(),
            Route::GetHashesAfter(hash) => format!("/hashes/{}", hash),
            Route::GetBlock(hash) => format!("/blocks/{}", hash),
            Route::PostBlock => "/blocks".into(),
            Route::PostTransaction => "/transactions".into(),
            Route::GetUsers => "/users".into(),
            Route::PostUsers => "/users".into(),
            Route::PostTransfers => "/transfers".into(),
        }
    }

    pub fn parse(method: &HttpMethod) -> Option<Self> {
        match method {
            HttpMethod::GET(path) if path == "/peers" => Some(Route::GetPeers),

            HttpMethod::GET(path) if path == "/hashes" => Some(Route::GetHashes),
            HttpMethod::GET(path) if path.starts_with("/hashes/") => path
                .split('/')
                .nth(2)
                .map(|h| Route::GetHashesAfter(h.to_string())),

            HttpMethod::GET(path) if path.starts_with("/blocks/") => path
                .split('/')
                .nth(2)
                .map(|h| Route::GetBlock(h.to_string())),
            HttpMethod::POST(path) if path == "/blocks" => Some(Route::PostBlock),

            HttpMethod::POST(path) if path == "/transactions" => Some(Route::PostTransaction),

            HttpMethod::GET(path) if path == "/users" => Some(Route::GetUsers),
            HttpMethod::POST(path) if path == "/users" => Some(Route::PostUsers),

            HttpMethod::POST(path) if path == "/transfers" => Some(Route::PostTransfers),

            _ => None,
        }
    }
}
