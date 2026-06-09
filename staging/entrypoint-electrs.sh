#!/usr/bin/env bash
# electrs regtest entrypoint.
# Creates an RPC auth cookie from the stack credentials and starts electrs.

set -euo pipefail

DATADIR=/data
mkdir -p "$DATADIR/db"

# electrs uses the Bitcoin Core cookie-file format: user:password (no trailing newline)
printf "user:password" > "$DATADIR/.cookie"

exec electrs \
    --network regtest \
    --daemon-rpc-addr bitcoin:18443 \
    --daemon-p2p-addr bitcoin:18444 \
    --cookie-file "$DATADIR/.cookie" \
    --db-dir "$DATADIR/db" \
    --electrum-rpc-addr 0.0.0.0:60401 \
    --log-filters INFO
