#!/usr/bin/env bash
# Bitcoin regtest entrypoint.
#
# First run: creates wallet "staging" and mines 101 blocks.
# Subsequent runs: skips init (flag file present on persistent disk).

set -euo pipefail

DATADIR=/data/bitcoin
INITIALIZED_FLAG="$DATADIR/.initialized"
BTC_CLI="bitcoin-cli -regtest -datadir=$DATADIR -rpcconnect=127.0.0.1 -rpcport=18443 -rpcuser=user -rpcpassword=password"

mkdir -p "$DATADIR"

if [ ! -f "$INITIALIZED_FLAG" ]; then
    echo "[bitcoin] First run — initializing regtest chain..."

    # Start temporarily in background for init
    bitcoind \
        -regtest \
        -datadir="$DATADIR" \
        -server=1 -txindex=1 \
        -rpcbind=127.0.0.1 -rpcallowip=127.0.0.1 \
        -rpcuser=user -rpcpassword=password -rpcport=18443 \
        -zmqpubrawblock=tcp://127.0.0.1:28332 \
        -fallbackfee=0.0002 \
        -daemon

    echo "[bitcoin] Waiting for RPC..."
    for i in $(seq 1 30); do
        $BTC_CLI getblockchaininfo > /dev/null 2>&1 && break
        sleep 1
        [ "$i" -eq 30 ] && echo "[bitcoin] ERROR: RPC timeout" >&2 && exit 1
    done

    $BTC_CLI createwallet "staging" > /dev/null 2>&1 || true
    ADDR=$($BTC_CLI -rpcwallet=staging getnewaddress)
    echo "[bitcoin] Mining 101 blocks to $ADDR..."
    $BTC_CLI -rpcwallet=staging generatetoaddress 101 "$ADDR" > /dev/null

    $BTC_CLI stop || true
    sleep 3
    touch "$INITIALIZED_FLAG"
    echo "[bitcoin] Init complete."
else
    echo "[bitcoin] Already initialized — skipping."
fi

echo "[bitcoin] Starting bitcoind..."
exec bitcoind \
    -regtest \
    -datadir="$DATADIR" \
    -server=1 -txindex=1 \
    -rpcbind=0.0.0.0 -rpcallowip=0.0.0.0/0 \
    -rpcuser=user -rpcpassword=password -rpcport=18443 \
    -zmqpubrawblock=tcp://127.0.0.1:28332 \
    -fallbackfee=0.0002 \
    -wallet=staging \
    -daemon=0
