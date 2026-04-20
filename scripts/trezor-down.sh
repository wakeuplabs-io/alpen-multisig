#!/usr/bin/env bash
set -euo pipefail

PORT="${1:-21324}"

echo "Stopping trezord-go (port $PORT) and emulator..."
pkill -f "trezord-go -e $PORT" || true
pkill -f "trezor-firmware/core/emu.py" || true
echo "Done."
