# ----- Build stage -----
FROM rust:1.86-slim AS builder

WORKDIR /app
COPY . .

RUN cargo build --release

# ----- Runtime stage -----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/p2p /app/p2p
COPY --from=builder /app/index.html /app/index.html

# peers_config.json is mounted per-container via Docker volume/bind or env-driven entrypoint
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

EXPOSE 5000

ENTRYPOINT ["/app/docker-entrypoint.sh"]