#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/ledger-up.sh <path/to/bitcoin.elf> [--model <MODEL>]
#
# Supported models:
#   nanos    — Nano S
#   nanosp   — Nano S+ (default)
#   nanox    — Nano X
#
# Example:
#   ./scripts/ledger-up.sh ~/ledger-apps/bitcoin_testnet_nanosp.elf
#   ./scripts/ledger-up.sh ~/ledger-apps/bitcoin_testnet_nanos.elf --model nanos
#
# Requires Docker. Starts Speculos on http://localhost:5000.
# Set LEDGER_SPECULOS_URL=http://localhost:5000 in desktop-app/.env to use it.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

APP_ELF=""
MODEL="nanosp"

while [[ $# -gt 0 ]]; do
	case "$1" in
		--model)
			MODEL="$2"
			shift 2
			;;
		*)
			APP_ELF="$1"
			shift
			;;
	esac
done

if [[ -z "$APP_ELF" ]]; then
	echo "Usage: $0 <path/to/bitcoin.elf> [--model <MODEL>]"
	echo ""
	echo "Get the app ELF from:"
	echo "  https://github.com/LedgerHQ/app-bitcoin-new/releases"
	echo ""
	echo "File naming convention:"
	echo "  bitcoin_testnet_nanosp.elf  (Nano S+)"
	echo "  bitcoin_testnet_nanos.elf   (Nano S)"
	echo "  bitcoin_testnet_nanox.elf   (Nano X)"
	exit 1
fi

VALID_MODELS=("nanos" "nanosp" "nanox")
VALID=false
for m in "${VALID_MODELS[@]}"; do
	[[ "$MODEL" == "$m" ]] && VALID=true && break
done
if [[ "$VALID" == false ]]; then
	echo "Unknown model: $MODEL"
	echo "Supported models: nanos, nanosp, nanox"
	exit 1
fi

if [[ ! -f "$APP_ELF" ]]; then
	echo "App ELF not found: $APP_ELF"
	exit 1
fi

APP_DIR="$(dirname "$(realpath "$APP_ELF")")"
APP_FILENAME="$(basename "$APP_ELF")"

# Kill any existing Speculos container
if docker ps -q --filter "name=alpen-speculos" | grep -q .; then
	echo "Stopping existing Speculos container..."
	docker stop alpen-speculos >/dev/null
fi

echo "Starting Speculos on http://localhost:5001 (model: $MODEL, app: $APP_FILENAME)..."
docker run --rm --name alpen-speculos \
	-p 5001:5000 \
	-v "$APP_DIR:/apps" \
	ghcr.io/ledgerhq/speculos:latest \
	--model "$MODEL" \
	--display headless \
	--api-port 5000 \
	"/apps/$APP_FILENAME"
