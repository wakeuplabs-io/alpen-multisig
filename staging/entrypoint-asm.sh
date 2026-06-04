#!/usr/bin/env bash
# strata-asm-runner entrypoint.
#
# Waits for bitcoind (separate Render service) to be ready, then:
#   First run:  fetches block-101 hash, generates asm-params.json.
#   Subsequent: skips init (flag file on persistent disk).
#
# Required env var: BITCOIN_RPC_URL (set in docker-compose.yml to the bitcoin service URL)

set -euo pipefail

ASMDB=/data/asm-db
INITIALIZED_FLAG="$ASMDB/.initialized"

# Default to Render private-network hostname if not set
BITCOIN_RPC_URL="${BITCOIN_RPC_URL:-http://alpen-multisig-bitcoin:18443}"
BTC_RPC_USER="${BITCOIN_RPC_USER:-user}"
BTC_RPC_PASS="${BITCOIN_RPC_PASS:-password}"

mkdir -p "$ASMDB"

btc_rpc() {
    curl -sf \
        --user "${BTC_RPC_USER}:${BTC_RPC_PASS}" \
        --data-binary "{\"jsonrpc\":\"1.0\",\"id\":\"asm-init\",\"method\":\"$1\",\"params\":[$2]}" \
        -H 'content-type: text/plain;' \
        "$BITCOIN_RPC_URL"
}

echo "[asm] Waiting for Bitcoin RPC at $BITCOIN_RPC_URL..."
for i in $(seq 1 60); do
    btc_rpc getblockchaininfo "" > /dev/null 2>&1 && break
    sleep 3
    [ "$i" -eq 60 ] && echo "[asm] ERROR: Bitcoin RPC not ready after 3 min" >&2 && exit 1
done
echo "[asm] Bitcoin RPC ready."

if [ ! -f "$INITIALIZED_FLAG" ]; then
    echo "[asm] First run — generating asm-params.json..."

    BLKID=$(btc_rpc getblockhash "101" | jq -r '.result')
    echo "[asm] Anchor block hash (height 101): $BLKID"

    sed "s/ANCHOR_BLKID_PLACEHOLDER/$BLKID/" /app/asm-params.template.json > /data/asm-params.json
    echo "[asm] asm-params.json generated."

    touch "$INITIALIZED_FLAG"
else
    echo "[asm] Already initialized — skipping."
    if [ ! -f /data/asm-params.json ]; then
        echo "[asm] ERROR: /data/asm-params.json missing but flag present." >&2
        exit 1
    fi
fi

echo "[asm] Starting strata-asm-runner..."
exec /usr/local/bin/strata-asm-runner \
    --config /app/asm-config.toml \
    --params /data/asm-params.json
