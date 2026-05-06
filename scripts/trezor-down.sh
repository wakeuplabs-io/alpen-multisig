#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/trezor-down.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/config.json"

PORT="21324"

if [[ -f "$CONFIG_FILE" ]]; then
	CONFIG_PORT="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('trezor_port',''))" "$CONFIG_FILE" 2>/dev/null || true)"
	if [[ -n "$CONFIG_PORT" ]]; then
		PORT="$CONFIG_PORT"
	fi
fi

echo "[1/2] Stopping trezord-go on port $PORT (if running)..."
pkill -f "trezord-go -e $PORT" || true

echo "[2/2] Stopping trezor emulator (if running)..."
pkill -f "core/emu.py" || true
pkill -f "/emu.py" || true

sleep 1

echo
echo "Trezor emulator stack stopped (if it was running)."
echo "Last known logs:"
echo "  /tmp/trezord-go.log"
echo "  /tmp/trezor-emu.log"
