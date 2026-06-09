#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/ledger-up.sh <path/to/bitcoin.elf> [--model <MODEL>] [--seed "<MNEMONIC>"]
#
# Supported models:
#   nanos    — Nano S
#   nanosp   — Nano S+ (default)
#   nanox    — Nano X
#
# Example:
#   ./scripts/ledger-up.sh ~/ledger-apps/bitcoin_testnet_nanosp.elf
#   ./scripts/ledger-up.sh ~/ledger-apps/bitcoin_testnet_nanos.elf --model nanos
#   ./scripts/ledger-up.sh ~/ledger-apps/bitcoin_testnet_nanosp.elf \
#     --seed "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
#
# Requires Docker. Starts Speculos on http://localhost:5001.
# Set LEDGER_SPECULOS_URL=http://localhost:5001 in desktop-app/.env to use it.
# --seed passes a BIP-39 mnemonic (or hex:...) to Speculos; omit to use Speculos default seed.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

APP_ELF=""
MODEL="nanosp"
SEED=""

while [[ $# -gt 0 ]]; do
	case "$1" in
		--model)
			MODEL="$2"
			shift 2
			;;
		--seed)
			if [[ $# -lt 2 ]]; then
				echo "Missing value for --seed"
				exit 1
			fi
			SEED="$2"
			shift 2
			;;
		*)
			APP_ELF="$1"
			shift
			;;
	esac
done

if [[ -z "$APP_ELF" ]]; then
	echo "Usage: $0 <path/to/bitcoin.elf> [--model <MODEL>] [--seed \"<MNEMONIC>\"]"
	echo ""
	echo "Get the app ELF from:"
	echo "  https://github.com/LedgerHQ/app-bitcoin-new/releases"
	echo ""
	echo "File naming convention:"
	echo "  bitcoin_testnet_nanosp.elf  (Nano S+)"
	echo "  bitcoin_testnet_nanos.elf   (Nano S)"
	echo "  bitcoin_testnet_nanox.elf   (Nano X)"
	echo ""
	echo "Optional --seed: BIP-39 mnemonic (quoted) or hex:... for Speculos device seed."
	exit 1
fi

if [[ -n "$SEED" ]]; then
	SEED_ARGS=(--seed "$SEED")
else
	SEED_ARGS=()
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

if [[ -n "$SEED" ]]; then
	echo "Starting Speculos on http://localhost:5001 (model: $MODEL, app: $APP_FILENAME, custom seed)..."
else
	echo "Starting Speculos on http://localhost:5001 (model: $MODEL, app: $APP_FILENAME)..."
fi
docker run --rm --name alpen-speculos \
	-p 5001:5000 \
	-p 9999:9999 \
	-v "$APP_DIR:/apps" \
	ghcr.io/ledgerhq/speculos:latest \
	--model "$MODEL" \
	--display headless \
	--api-port 5000 \
	"${SEED_ARGS[@]}" \
	"/apps/$APP_FILENAME"
