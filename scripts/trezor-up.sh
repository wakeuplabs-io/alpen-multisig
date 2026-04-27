#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/trezor-up.sh [/ABS/PATH/trezor-firmware] [--build]
#
# Example:
#   ./scripts/trezor-up.sh /Users/juandahl/Documents/wakeup/repositories/trezor-firmware --build
#   ./scripts/trezor-up.sh --build
#   ./scripts/trezor-up.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/config.json"

TREZOR_REPO=""
BUILD_FLAG=""
MNEMONIC=""

for arg in "$@"; do
	case "$arg" in
		--build)
			BUILD_FLAG="--build"
			;;
		*)
			TREZOR_REPO="$arg"
			;;
	esac
done

if [[ -z "$TREZOR_REPO" && -f "$CONFIG_FILE" ]]; then
	TREZOR_REPO="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('trezor_repo',''))" "$CONFIG_FILE" 2>/dev/null || true)"
fi

if [[ -z "$TREZOR_REPO" ]]; then
	echo "Missing trezor repo path."
	echo "Set scripts/config.json with key 'trezor_repo' or pass it as first argument."
	echo "Usage: $0 [/ABS/PATH/trezor-firmware] [--build]"
	exit 1
fi

if [[ -f "$CONFIG_FILE" ]]; then
	MNEMONIC="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('mnemonic',''))" "$CONFIG_FILE" 2>/dev/null || true)"
fi

if [[ -z "$MNEMONIC" ]]; then
	echo "Missing mnemonic."
	echo "Set scripts/config.json with key 'mnemonic'."
	exit 1
fi

CORE_DIR="$TREZOR_REPO/core"
PYTHON_DIR="$TREZOR_REPO/python"
PORT="21324"
MODEL="T2B1"
PIN="1234"

if [[ ! -d "$CORE_DIR" ]]; then
	echo "Directory does not exist: $CORE_DIR"
	exit 1
fi

echo "[1/5] Stopping previous emulator processes (if any)..."
pkill -f "trezord-go -e $PORT" || true
pkill -f "$CORE_DIR/emu.py" || true
sleep 1

echo "[2/5] Starting trezord-go on port $PORT..."
trezord-go -e "$PORT" >/tmp/trezord-go.log 2>&1 &
sleep 1

if [[ "$BUILD_FLAG" == "--build" ]]; then
	echo "[3/5] Building firmware for model $MODEL..."
	(
		cd "$CORE_DIR"
		make build_unix TREZOR_MODEL="$MODEL"
	)
else
	echo "[3/5] Skipping build (pass --build to compile first)."
fi

echo "[4/5] Starting emulator..."
(
	cd "$CORE_DIR"
	TREZOR_MODEL="$MODEL" uv run ./emu.py >/tmp/trezor-emu.log 2>&1 &
)
sleep 2

echo "[5/5] Loading device seed and PIN..."
TREZORCTL_DIR="$TREZOR_REPO"
if [[ -d "$PYTHON_DIR" ]]; then
	TREZORCTL_DIR="$PYTHON_DIR"
fi

MAX_ATTEMPTS=20
ATTEMPT=1

while (( ATTEMPT <= MAX_ATTEMPTS )); do
	if (
		cd "$TREZORCTL_DIR"
		uv run trezorctl -p "udp:127.0.0.1:$PORT" load-device \
			--mnemonic "$MNEMONIC" \
			--pin "$PIN"
	); then
		break
	fi

	if (( ATTEMPT == MAX_ATTEMPTS )); then
		echo "Could not connect to emulator at udp:127.0.0.1:$PORT after $MAX_ATTEMPTS attempts."
		echo "Check logs:"
		echo "  /tmp/trezord-go.log"
		echo "  /tmp/trezor-emu.log"
		exit 1
	fi

	echo "Device not ready yet (attempt $ATTEMPT/$MAX_ATTEMPTS). Retrying in 1s..."
	sleep 1
	ATTEMPT=$((ATTEMPT + 1))
done

echo
echo "Trezor emulator is ready."
echo "Logs:"
echo "  /tmp/trezord-go.log"
echo "  /tmp/trezor-emu.log"
