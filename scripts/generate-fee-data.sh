#!/usr/bin/env bash
# generate-fee-data.sh — Populate regtest with transactions to enable fee estimation.
#
# Bitcoin Core's `estimatesmartfee` requires transaction history to produce estimates.
# On a fresh regtest chain, it returns errors until enough blocks contain confirmed
# transactions with fee data. This script generates that data by sending self-transactions
# and mining blocks in rounds.
#
# Usage:
#   ./generate-fee-data.sh [--rounds N] [--txs-per-round N] [--mine-extra N] [--vary-fees] [--check] [-h]
#
# Options:
#   --rounds N          Number of send-then-mine rounds (default: 25)
#   --txs-per-round N   Transactions per round (default: 5)
#   --mine-extra N      Extra empty blocks to mine at the end (default: 6)
#   --vary-fees         Generate different fee rates for Fast/Medium/Slow targets
#   --check             Show current estimatesmartfee results without generating data
#   -h                  Show help
#
# Prerequisites:
#   - Docker stack running (./local-stack.sh)
#   - curl and jq in PATH
#
# After running, the app's fee estimation (targets 1, 6, 12 blocks) will return
# valid estimates instead of falling back to min-relay rates.
#
# Fee variation strategy (--vary-fees):
#   - Fast tier: TXs at 30 sat/vB, mined immediately (1 block)
#   - Medium tier: TXs at 15 sat/vB, mined after 5 blocks
#   - Slow tier: TXs at 3 sat/vB, mined after 11 blocks
#   This creates a fee rate ladder that estimatesmartfee can differentiate.

set -euo pipefail

RPC_URL="http://127.0.0.1:18443"
WALLET_URL="$RPC_URL/wallet/staging"
RPC_USER="user"
RPC_PASS="password"

ROUNDS=25
TXS_PER_ROUND=5
MINE_EXTRA=6
CHECK_ONLY=0
VARY_FEES=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rounds)
      ROUNDS="${2:-25}"
      [[ "$ROUNDS" =~ ^[0-9]+$ ]] || ROUNDS=25
      shift 2
      ;;
    --txs-per-round)
      TXS_PER_ROUND="${2:-5}"
      [[ "$TXS_PER_ROUND" =~ ^[0-9]+$ ]] || TXS_PER_ROUND=5
      shift 2
      ;;
    --mine-extra)
      MINE_EXTRA="${2:-6}"
      [[ "$MINE_EXTRA" =~ ^[0-9]+$ ]] || MINE_EXTRA=6
      shift 2
      ;;
    --vary-fees)
      VARY_FEES=1
      shift
      ;;
    --check)
      CHECK_ONLY=1
      shift
      ;;
    -h|--help)
      sed -n '2,/^$/s/^# \{0,1\}//p' "$0"
      exit 0
      ;;
    *)
      echo "usage: $0 [--rounds N] [--txs-per-round N] [--mine-extra N] [--vary-fees] [--check] [-h]" >&2
      exit 1
      ;;
  esac
done

die() {
  echo "error: $*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "'$1' not found in PATH — $2"
}

require_cmd curl "install curl"
require_cmd jq "install jq"

rpc_call() {
  local url=$1
  local method=$2
  local params=${3:-"[]"}
  
  curl -sf --user "$RPC_USER:$RPC_PASS" \
    --data-binary "{\"jsonrpc\":\"1.0\",\"id\":\"script\",\"method\":\"$method\",\"params\":$params}" \
    -H "Content-Type: application/json" \
    "$url"
}

check_rpc() {
  local result
  result=$(rpc_call "$RPC_URL" "getblockchaininfo" 2>&1) || {
    die "cannot connect to bitcoind at $RPC_URL (is the Docker stack running?)"
  }
  
  local chain
  chain=$(echo "$result" | jq -r '.result.chain // empty' 2>/dev/null)
  [[ "$chain" == "regtest" ]] || die "not connected to regtest (got chain: $chain)"
}

check_wallet() {
  local result
  result=$(rpc_call "$RPC_URL" "listwallets" 2>&1) || {
    die "cannot list wallets (is bitcoind running?)"
  }
  
  local wallets
  wallets=$(echo "$result" | jq -r '.result[]' 2>/dev/null)
  
  if ! echo "$wallets" | grep -q "^staging$"; then
    die "wallet 'staging' not loaded (expected from Docker stack entrypoint)"
  fi
}

show_estimates() {
  echo ""
  echo "=== current fee estimates ==="
  
  for target in 1 6 12; do
    local result
    result=$(rpc_call "$RPC_URL" "estimatesmartfee" "[$target]" 2>&1) || {
      echo "  target=$target: ERROR (RPC call failed)"
      continue
    }
    
    local feerate
    feerate=$(echo "$result" | jq -r '.result.feerate // empty' 2>/dev/null)
    local errors
    errors=$(echo "$result" | jq -r '.result.errors[0] // empty' 2>/dev/null)
    local blocks
    blocks=$(echo "$result" | jq -r '.result.blocks // empty' 2>/dev/null)
    
    if [[ -n "$errors" ]]; then
      echo "  target=$target: ERROR ($errors)"
    elif [[ -z "$feerate" ]]; then
      echo "  target=$target: NO ESTIMATE (insufficient data)"
    else
      local sat_per_kvb
      sat_per_kvb=$(echo "$feerate * 100000000" | bc | cut -d. -f1)
      local sat_per_vb
      sat_per_vb=$(echo "scale=2; $sat_per_kvb / 1000" | bc)
      echo "  target=$target: $feerate BTC/kvB ($sat_per_kvb sat/kvB, ~$sat_per_vb sat/vB) [blocks=$blocks]"
    fi
  done
}

generate_data() {
  local total_txs=0
  
  echo ""
  echo "=== generating fee estimation data ==="
  echo "  Rounds: $ROUNDS"
  echo "  Transactions per round: $TXS_PER_ROUND"
  echo "  Extra blocks at end: $MINE_EXTRA"
  echo ""
  
  for ((round=1; round<=ROUNDS; round++)); do
    echo -n "  Round $round/$ROUNDS: sending $TXS_PER_ROUND txs..."
    
    for ((tx=1; tx<=TXS_PER_ROUND; tx++)); do
      local addr
      addr=$(rpc_call "$WALLET_URL" "getnewaddress" "[]" | jq -r '.result' 2>/dev/null) || {
        echo " FAILED (getnewaddress)"
        return 1
      }
      
      local amount
      amount=$(printf "%.8f" "$(echo "scale=8; 0.001 + ($RANDOM % 100) / 100000" | bc)")
      
      rpc_call "$WALLET_URL" "sendtoaddress" "[\"$addr\", $amount]" >/dev/null 2>&1 || {
        echo " FAILED (sendtoaddress)"
        return 1
      }
      
      total_txs=$((total_txs + 1))
    done
    
    local miner_addr
    miner_addr=$(rpc_call "$WALLET_URL" "getnewaddress" "[]" | jq -r '.result' 2>/dev/null) || {
      echo " FAILED (getnewaddress for mining)"
      return 1
    }
    
    rpc_call "$RPC_URL" "generatetoaddress" "[1, \"$miner_addr\"]" >/dev/null 2>&1 || {
      echo " FAILED (generatetoaddress)"
      return 1
    }
    
    echo " OK"
  done
  
  if [[ "$MINE_EXTRA" -gt 0 ]]; then
    echo ""
    echo -n "  Mining $MINE_EXTRA extra blocks..."
    local miner_addr
    miner_addr=$(rpc_call "$WALLET_URL" "getnewaddress" "[]" | jq -r '.result' 2>/dev/null) || {
      echo " FAILED"
      return 1
    }
    
    rpc_call "$RPC_URL" "generatetoaddress" "[$MINE_EXTRA, \"$miner_addr\"]" >/dev/null 2>&1 || {
      echo " FAILED"
      return 1
    }
    
    echo " OK"
  fi
  
  echo ""
  echo "=== generation complete ==="
  echo "  Total transactions sent: $total_txs"
  echo "  Total blocks mined: $((ROUNDS + MINE_EXTRA))"
}

generate_varied_fees() {
  local total_txs=0
  local txs_per_tier=100
  local congestion_txs=200
  
  echo ""
  echo "=== generating varied fee data with congestion ==="
  echo "  Congestion TXs (1 sat/vB): $congestion_txs"
  echo "  Fast tier TXs (30 sat/vB): $txs_per_tier"
  echo "  Medium tier TXs (15 sat/vB): $txs_per_tier"
  echo "  Slow tier TXs (3 sat/vB): $txs_per_tier"
  echo ""
  
  echo "--- Creating mempool congestion (200 txs at 1 sat/vB) ---"
  echo -n "  Sending $congestion_txs low-fee txs..."
  for ((tx=1; tx<=congestion_txs; tx++)); do
    local addr
    addr=$(rpc_call "$WALLET_URL" "getnewaddress" "[]" | jq -r '.result' 2>/dev/null) || {
      echo " FAILED (getnewaddress)"
      return 1
    }
    
    local amount
    amount=$(printf "%.8f" "$(echo "scale=8; 0.0001 + ($RANDOM % 10) / 1000000" | bc)")
    
    rpc_call "$WALLET_URL" "send" "[[{\"$addr\":$amount}], null, null, 1]" >/dev/null 2>&1 || {
      echo " FAILED at tx $tx"
      return 1
    }
    
    total_txs=$((total_txs + 1))
    
    if ((tx % 50 == 0)); then
      echo -n " $tx..."
    fi
  done
  echo " OK"
  
  echo ""
  echo "--- Fast tier (30 sat/vB) ---"
  echo -n "  Sending $txs_per_tier txs at 30 sat/vB..."
  for ((tx=1; tx<=txs_per_tier; tx++)); do
    local addr
    addr=$(rpc_call "$WALLET_URL" "getnewaddress" "[]" | jq -r '.result' 2>/dev/null) || {
      echo " FAILED (getnewaddress)"
      return 1
    }
    
    local amount
    amount=$(printf "%.8f" "$(echo "scale=8; 0.001 + ($RANDOM % 100) / 100000" | bc)")
    
    rpc_call "$WALLET_URL" "send" "[[{\"$addr\":$amount}], null, null, 30]" >/dev/null 2>&1 || {
      echo " FAILED (send with fee_rate 30)"
      return 1
    }
    
    total_txs=$((total_txs + 1))
  done
  echo " OK"
  
  echo -n "  Mining 1 block (should include fast tier)..."
  local miner_addr
  miner_addr=$(rpc_call "$WALLET_URL" "getnewaddress" "[]" | jq -r '.result' 2>/dev/null) || {
    echo " FAILED"
    return 1
  }
  rpc_call "$RPC_URL" "generatetoaddress" "[1, \"$miner_addr\"]" >/dev/null 2>&1 || {
    echo " FAILED"
    return 1
  }
  echo " OK"
  
  echo ""
  echo "--- Medium tier (15 sat/vB) ---"
  echo -n "  Sending $txs_per_tier txs at 15 sat/vB..."
  for ((tx=1; tx<=txs_per_tier; tx++)); do
    local addr
    addr=$(rpc_call "$WALLET_URL" "getnewaddress" "[]" | jq -r '.result' 2>/dev/null) || {
      echo " FAILED (getnewaddress)"
      return 1
    }
    
    local amount
    amount=$(printf "%.8f" "$(echo "scale=8; 0.001 + ($RANDOM % 100) / 100000" | bc)")
    
    rpc_call "$WALLET_URL" "send" "[[{\"$addr\":$amount}], null, null, 15]" >/dev/null 2>&1 || {
      echo " FAILED (send with fee_rate 15)"
      return 1
    }
    
    total_txs=$((total_txs + 1))
  done
  echo " OK"
  
  echo -n "  Mining 5 blocks (should include medium tier)..."
  rpc_call "$RPC_URL" "generatetoaddress" "[5, \"$miner_addr\"]" >/dev/null 2>&1 || {
    echo " FAILED"
    return 1
  }
  echo " OK"
  
  echo ""
  echo "--- Slow tier (3 sat/vB) ---"
  echo -n "  Sending $txs_per_tier txs at 3 sat/vB..."
  for ((tx=1; tx<=txs_per_tier; tx++)); do
    local addr
    addr=$(rpc_call "$WALLET_URL" "getnewaddress" "[]" | jq -r '.result' 2>/dev/null) || {
      echo " FAILED (getnewaddress)"
      return 1
    }
    
    local amount
    amount=$(printf "%.8f" "$(echo "scale=8; 0.001 + ($RANDOM % 100) / 100000" | bc)")
    
    rpc_call "$WALLET_URL" "send" "[[{\"$addr\":$amount}], null, null, 3]" >/dev/null 2>&1 || {
      echo " FAILED (send with fee_rate 3)"
      return 1
    }
    
    total_txs=$((total_txs + 1))
  done
  echo " OK"
  
  echo -n "  Mining 11 blocks (should include slow tier)..."
  rpc_call "$RPC_URL" "generatetoaddress" "[11, \"$miner_addr\"]" >/dev/null 2>&1 || {
    echo " FAILED"
    return 1
  }
  echo " OK"
  
  if [[ "$MINE_EXTRA" -gt 0 ]]; then
    echo ""
    echo -n "  Mining $MINE_EXTRA extra blocks..."
    rpc_call "$RPC_URL" "generatetoaddress" "[$MINE_EXTRA, \"$miner_addr\"]" >/dev/null 2>&1 || {
      echo " FAILED"
      return 1
    }
    echo " OK"
  fi
  
  echo ""
  echo "=== generation complete ==="
  echo "  Total transactions sent: $total_txs"
  echo "  Strategy: congestion (1 sat/vB) + fast (30) + medium (15) + slow (3)"
  echo "  Note: Regtest blocks are large, so fee differentiation may be limited"
}

main() {
  echo ""
  echo "=== fee estimation data generator ==="
  
  check_rpc
  check_wallet
  
  if [[ "$CHECK_ONLY" == "1" ]]; then
    show_estimates
    exit 0
  fi
  
  if [[ "$VARY_FEES" == "1" ]]; then
    generate_varied_fees
  else
    generate_data
  fi
  
  show_estimates
  
  echo ""
  echo "Done. The app's fee estimation should now return valid estimates."
  echo "To verify: ./generate-fee-data.sh --check"
}

main "$@"
