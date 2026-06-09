#!/usr/bin/env bash
# Smoke test: verify electrs is reachable and indexing via Electrum protocol.
# Usage: ./scripts/smoke-electrs.sh [host:port]
set -euo pipefail

ADDR="${1:-127.0.0.1:60401}"
HOST="${ADDR%:*}"
PORT="${ADDR#*:}"

echo "Querying electrs at $HOST:$PORT ..."
RESPONSE=$(python3 - <<EOF
import socket, sys
s = socket.socket()
s.settimeout(5)
try:
    s.connect(("$HOST", $PORT))
    s.sendall(b'{"id":1,"method":"blockchain.headers.subscribe","params":[]}\n')
    print(s.recv(4096).decode())
except Exception as e:
    print(f"ERROR: {e}", file=sys.stderr)
    sys.exit(1)
finally:
    s.close()
EOF
)

if echo "$RESPONSE" | grep -q '"result"'; then
    echo "OK: electrs is up and indexing."
else
    echo "FAIL: unexpected response from electrs at $ADDR" >&2
    echo "$RESPONSE" >&2
    exit 1
fi
