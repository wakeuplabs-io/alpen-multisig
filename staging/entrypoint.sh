#!/usr/bin/env bash
# Staging entrypoint: first-time regtest init + supervisord launch.
#
# On the first container start (fresh persistent disk):
#   1. Starts bitcoind temporarily
#   2. Mines 101 blocks to reach the ASM anchor height
#   3. Gets the block 101 hash and injects it into asm-params.json
#   4. Stops the temporary bitcoind
#
# On subsequent starts the .initialized flag is present and this is skipped.
# supervisord then takes over and manages all three processes.

set -euo pipefail

DATADIR=/data/bitcoin
ASMDB=/data/asm-db
INITIALIZED_FLAG="$DATADIR/.initialized"

BTC_CLI="bitcoin-cli -regtest -datadir=$DATADIR -rpcconnect=127.0.0.1 -rpcport=18443 -rpcuser=user -rpcpassword=password"

mkdir -p "$DATADIR" "$ASMDB"

# ── First-time init ───────────────────────────────────────────────────────────
if [ ! -f "$INITIALIZED_FLAG" ]; then
    echo "[entrypoint] First run — initializing regtest chain..."

    # Start bitcoind in the background for the init phase
    bitcoind \
        -regtest \
        -datadir="$DATADIR" \
        -server=1 \
        -txindex=1 \
        -rpcbind=127.0.0.1 \
        -rpcallowip=127.0.0.1 \
        -rpcuser=user \
        -rpcpassword=password \
        -rpcport=18443 \
        -zmqpubrawblock=tcp://127.0.0.1:28332 \
        -fallbackfee=0.0002 \
        -daemon

    # Wait for RPC to be ready
    echo "[entrypoint] Waiting for bitcoind RPC..."
    for i in $(seq 1 30); do
        if $BTC_CLI getblockchaininfo > /dev/null 2>&1; then
            echo "[entrypoint] bitcoind RPC ready."
            break
        fi
        sleep 1
        if [ "$i" -eq 30 ]; then
            echo "[entrypoint] ERROR: bitcoind did not start in time." >&2
            exit 1
        fi
    done

    # Create wallet and mine 101 blocks
    $BTC_CLI createwallet "staging" > /dev/null 2>&1 || true
    ADDR=$($BTC_CLI -rpcwallet=staging getnewaddress)
    echo "[entrypoint] Mining 101 blocks to $ADDR..."
    $BTC_CLI -rpcwallet=staging generatetoaddress 101 "$ADDR" > /dev/null

    # Get block 101 hash for asm-params.json
    BLKID=$($BTC_CLI getblockhash 101)
    echo "[entrypoint] Anchor block hash (height 101): $BLKID"

    # Inject into asm-params.json — stored on the persistent volume so it
    # survives container restarts (the template lives in the image at /app/).
    sed "s/ANCHOR_BLKID_PLACEHOLDER/$BLKID/" /app/asm-params.template.json > /data/asm-params.json
    echo "[entrypoint] asm-params.json generated."

    # Stop the temporary bitcoind gracefully
    $BTC_CLI stop || true
    sleep 3

    touch "$INITIALIZED_FLAG"
    echo "[entrypoint] Init complete."
else
    echo "[entrypoint] Already initialized — skipping chain setup."
    # asm-params.json persists on disk from the first run
    if [ ! -f /data/asm-params.json ]; then
        echo "[entrypoint] ERROR: /data/asm-params.json missing but initialized flag present." >&2
        exit 1
    fi
fi

# ── Hand off to supervisord ───────────────────────────────────────────────────
echo "[entrypoint] Starting supervisord..."
exec /usr/bin/supervisord -c /etc/supervisor/conf.d/staging.conf
