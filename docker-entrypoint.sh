#!/bin/sh
# docker-entrypoint.sh
#
# Environment variables:
#   NODE_PORT          - port this node listens on (default: 5000)
#   BOOTSTRAP_IP       - IP of the bootstrap / seed node (default: 172.20.0.2)
#   BOOTSTRAP_PORT     - port of the bootstrap node (default: 5000)

NODE_PORT="${NODE_PORT:-5000}"
BOOTSTRAP_IP="${BOOTSTRAP_IP:-172.20.0.2}"
BOOTSTRAP_PORT="${BOOTSTRAP_PORT:-5000}"

# Write peers_config.json
cat > /app/peers_config.json <<EOF
[
  {
    "ip": "${BOOTSTRAP_IP}",
    "port": ${BOOTSTRAP_PORT}
  }
]
EOF

echo "[ENTRYPOINT] Node port      : ${NODE_PORT}"
echo "[ENTRYPOINT] Bootstrap peer : ${BOOTSTRAP_IP}:${BOOTSTRAP_PORT}"
echo "[ENTRYPOINT] peers_config.json written."

cd /app
exec /app/p2p "${NODE_PORT}"