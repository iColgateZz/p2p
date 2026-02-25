use p2p::node;

fn main() {
    let ip = "127.0.0.1";
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(5000); // if port is not specified then default is 5000

    node::start(ip, port);
}
