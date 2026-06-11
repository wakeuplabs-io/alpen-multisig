#!/usr/bin/env bash
# local-stack.sh — Bootstrap the full Docker local stack (bitcoin + asm + orchestrator + regtest-dev-api).
#
# Usage:
#   ./local-stack.sh --clean [--orchestrator] [--regtest-dev-api] [--no-build] [--no-orchestrator]
#   ./local-stack.sh --status
#   ./local-stack.sh --stop
#   ./local-stack.sh --mine [N]
#   ./local-stack.sh --fund <address> [amount_btc]
#   ./local-stack.sh -h
#
# Options:
#   --clean                 Clean volumes (bitcoin-data, asm-data, electrs-data) and residual state
#   --orchestrator          With --clean: also prune orchestrator build cache
#   --regtest-dev-api       With --clean: also prune regtest-dev-api build cache
#   --no-build              Skip docker build (use existing images)
#   --no-orchestrator       Don't start orchestrator (for local dev scenarios)
#   --mine [N]              Mine N blocks (default: 1)
#   --fund <address> [N]    Fund address with N BTC (default: 1 BTC)
#
# Prerequisites:
#   - docker available in PATH
#   - asm/ submodule populated (run: git submodule update --init asm)
#
# Port map:
#   localhost:18443  → bitcoin (RPC)
#   localhost:60401  → electrs (Electrum indexer)
#   localhost:8080   → asm (strata-asm-runner admin RPC)
#   localhost:3000   → orchestrator (backend API)
#   localhost:3001   → regtest-dev-api (mine/faucet)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_DIR="$SCRIPT_DIR/../staging"
COMPOSE_FILE="$COMPOSE_DIR/docker-compose.local.yml"
COMPOSE_PROJECT=$(basename "$COMPOSE_DIR" | tr '[:lower:]' '[:upper:]')
CONTAINER_PREFIX="${COMPOSE_PROJECT,,}"

CLEAN=0
CLEAN_ORCHESTRATOR=0
CLEAN_REGTEST_DEV_API=0
NO_BUILD=0
NO_ORCHESTRATOR=0
STOP=0
STATUS=0
MINE_COUNT=0
FUND_ADDRESS=""
FUND_AMOUNT=1.0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean) CLEAN=1 ;;
    --orchestrator) CLEAN_ORCHESTRATOR=1 ;;
    --regtest-dev-api) CLEAN_REGTEST_DEV_API=1 ;;
    --no-build) NO_BUILD=1 ;;
    --no-orchestrator) NO_ORCHESTRATOR=1 ;;
    --stop) STOP=1 ;;
    --status) STATUS=1 ;;
    --mine)
      MINE_COUNT="${2:-1}"
      [[ "$MINE_COUNT" =~ ^[0-9]+$ ]] || MINE_COUNT=1
      [[ "$MINE_COUNT" =~ ^[0-9]+$ ]] && shift || true
      ;;
    --fund)
      FUND_ADDRESS="${2:-}"
      FUND_AMOUNT="${3:-1.0}"
      if [[ -z "$FUND_ADDRESS" ]]; then
        echo "error: --fund requires an address" >&2
        exit 1
      fi
      if [[ $# -ge 2 ]]; then
        shift 2
      else
        shift
      fi
      ;;
    -h|--help) HELP=1 ;;
    *)
      echo "usage: $0 [--clean [--orchestrator] [--regtest-dev-api]] [--no-build] [--no-orchestrator] [--stop] [--status] [--mine N] [--fund <address> [btc]] [-h]" >&2
      exit 1
      ;;
  esac
  if [[ $# -gt 0 ]]; then shift; fi
done

die() {
  echo "error: $*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "'$1' not found in PATH — $2"
}

require_cmd docker "install docker or add it to PATH"
require_cmd curl "install curl"
require_cmd jq "install jq"

# Detect docker compose variant
DOCKER_COMPOSE=(docker)
if docker compose version >/dev/null 2>&1; then
  DOCKER_COMPOSE=(docker compose)
elif docker-compose --version >/dev/null 2>&1; then
  DOCKER_COMPOSE=(docker-compose)
else
  die "docker compose not found"
fi

# Validate compose file
[[ -f "$COMPOSE_FILE" ]] || die "docker-compose.yml not found at $COMPOSE_FILE"

# Check asm submodule is populated
REPO_DIR="$(cd "$COMPOSE_DIR/.." && pwd)"
ASM_DIR="$REPO_DIR/asm"
if [[ ! -f "$ASM_DIR/Cargo.toml" ]]; then
  die "asm submodule not populated.
  Run: cd \"$REPO_DIR\" && git submodule update --init asm"
fi

show_help() {
  cat <<'EOF'
local-stack.sh — Bootstrap the full Docker local stack.

Usage:
  ./local-stack.sh --clean [--orchestrator] [--regtest-dev-api] [--no-build] [--no-orchestrator]
  ./local-stack.sh --status
  ./local-stack.sh --stop
  ./local-stack.sh --mine [N]
  ./local-stack.sh --fund <address> [amount_btc]
  ./local-stack.sh -h

Options:
  --clean                 Clean volumes (bitcoin-data, asm-data, electrs-data) and residual state
  --orchestrator          With --clean: also prune orchestrator build cache
  --regtest-dev-api       With --clean: also prune regtest-dev-api build cache
  --no-build              Skip docker build (use existing images)
  --no-orchestrator       Don't start orchestrator container (use when running orchestrator locally)
  --mine [N]              Mine N blocks on regtest (default: 1)
  --fund <address> [N]    Fund address with N BTC (default: 1 BTC)

Prerequisites:
  - docker in PATH
  - curl and jq in PATH
  - asm/ submodule populated (git submodule update --init asm)

Ports:
  18443  bitcoin      (RPC)
  60401  electrs      (Electrum indexer)
  8080   asm          (strata-asm-runner admin RPC)
  3000   orchestrator (backend API) — skipped with --no-orchestrator
  3001   regtest-dev-api (mine/faucet)

Volumes (clean with --clean):
  bitcoin-data  — bitcoin chain data
  asm-data      — ASM sled DB + asm-params.json
  electrs-data  — Electrum index data
  postgres-data — NOT cleaned (intentional)

First run: bitcoin entrypoint creates 'staging' wallet + mines 101 blocks.
            asm entrypoint generates asm-params.json from template.

Examples:
  ./local-stack.sh --mine 5
  ./local-stack.sh --fund bcrt1q... 0.5

EOF
}

show_status() {
  echo ""
  echo "Docker compose stack: $COMPOSE_DIR"
  echo ""
  "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" ps --format table 2>/dev/null || true

  echo ""
  echo "Live health checks:"
  check_service() {
    local name=$1
    local port=$2
    local endpoint=${3:-}
    local status="❌ not running"
    local detail=""

    if docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${CONTAINER_PREFIX}-${name}-1$"; then
      if [[ -n "$endpoint" ]]; then
        local result
        result=$(curl -sf -m 3 "http://localhost:${port}${endpoint}" 2>/dev/null || echo "FAIL")
        if [[ "$result" != "FAIL" ]]; then
          status="✅ healthy"
        else
          status="⚠️  running (endpoint failed)"
        fi
      else
        local health
        health=$(docker inspect --format='{{.State.Health.Status}}' "${CONTAINER_PREFIX}-${name}-1" 2>/dev/null || echo "none")
        case "$health" in
          healthy) status="✅ healthy" ;;
          starting) status="🔄 starting" ;;
          unhealthy) status="❌ unhealthy" ;;
          "<no value>"|none) status="✅ running" ;;
          *) status="⚠️  $health" ;;
        esac
      fi
    else
      status="— stopped"
    fi
    printf "  %-20s %-12s %s\n" "$name" "$status" "(:${port})"
  }

  check_service "bitcoin" "18443"
  check_service "electrs" "60401"
  check_service "asm" "8080" "/"
  check_service "postgres" "5432"
  if [[ "$NO_ORCHESTRATOR" == "1" ]]; then
    printf "  %-20s %-12s %s\n" "orchestrator" "— skipped" "(--no-orchestrator)"
  else
    check_service "orchestrator" "3000" "/api/v1/health"
  fi
  check_service "regtest-dev-api" "3001" "/mine"
}

do_stop() {
  echo "Stopping stack..."
  "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" down 2>/dev/null || true
  echo "OK stopped"
}

do_mine() {
  local count=${1:-1}
  echo ""
  echo "=== mining ${count} block(s) ==="
  local result
  result=$(curl -sf -X POST "http://localhost:3001/mine?count=${count}" 2>&1) || {
    echo "ERROR: failed to mine blocks (is regtest-dev-api running?)" >&2
    echo "$result" >&2
    return 1
  }
  echo "$result" | jq -r '.block_hashes[]' 2>/dev/null | while read -r hash; do
    echo "  Block: $hash"
  done
  echo "Done."
}

do_fund() {
  local address=$1
  local amount=${2:-1.0}
  echo ""
  echo "=== funding ${address} with ${amount} BTC ==="
  local result
  result=$(curl -sf -X POST "http://localhost:3001/faucet" \
    -H "Content-Type: application/json" \
    -d "{\"address\":\"${address}\",\"amount_btc\":${amount}}" 2>&1) \
    || { echo "ERROR: failed to fund address (is regtest-dev-api running?)" >&2; echo "$result" >&2; return 1; }
  local txid=$(echo "$result" | jq -r '.txid' 2>/dev/null)
  local block=$(echo "$result" | jq -r '.block_hash' 2>/dev/null)
  echo "  TXID: $txid"
  echo "  Block: $block"
  echo "Done."
}

do_clean() {
  echo ""
  echo "=== cleaning volumes ==="
  "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" down -v 2>/dev/null || true

  echo "=== cleaning residual state ==="
  rm -f "$REPO_DIR/scripts/asm-params.json" 2>/dev/null || true
  rm -f "$COMPOSE_DIR/.stack-pids" 2>/dev/null || true

  if [[ "$CLEAN_ORCHESTRATOR" == "1" ]]; then
    echo "=== pruning local-orchestrator build cache ==="
    docker builder prune -a --filter label=local-orchestrator 2>/dev/null || true
  fi

  if [[ "$CLEAN_REGTEST_DEV_API" == "1" ]]; then
    echo "=== pruning local-regtest-dev-api build cache ==="
    docker builder prune -a --filter label=local-regtest-dev-api 2>/dev/null || true
  fi

  echo "OK clean complete"
}

do_start() {
  if [[ "$NO_ORCHESTRATOR" == "1" ]]; then
    echo ""
    echo "=== docker compose build (excluding orchestrator) ==="
    "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" build postgres asm bitcoin regtest-dev-api

    echo ""
    echo "=== docker compose up (excluding orchestrator) ==="
    "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" up -d postgres asm bitcoin regtest-dev-api
  else
    echo ""
    echo "=== docker compose build ==="
    "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" build

    echo ""
    echo "=== docker compose up (detached) ==="
    "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" up -d
  fi

  echo ""
  echo "=== waiting for services ==="

  wait_for_service() {
    local svc=$1
    local max=${2:-300}
    local i=0
    echo -n "  waiting for $svc..."
    while (( i < max )); do
      local status
      status=$(docker inspect --format='{{.State.Health.Status}}' "${CONTAINER_PREFIX}-${svc}-1" 2>/dev/null || echo "")
      if [[ -z "$status" ]]; then
        local running
        running=$(docker inspect --format='{{.State.Running}}' "${CONTAINER_PREFIX}-${svc}-1" 2>/dev/null || echo "false")
        if [[ "$running" == "true" ]]; then
          echo " ready"
          return 0
        fi
      elif [[ "$status" == "healthy" ]]; then
        echo " healthy"
        return 0
      elif [[ "$status" == "<no value>" ]]; then
        local running
        running=$(docker inspect --format='{{.State.Running}}' "${CONTAINER_PREFIX}-${svc}-1" 2>/dev/null || echo "false")
        if [[ "$running" == "true" ]]; then
          echo " ready"
          return 0
        fi
      fi
      sleep 2
      i=$((i + 2))
    done
    echo " TIMEOUT (check: docker compose logs $svc)"
    return 1
  }

  echo ""
  echo "=== waiting for services to stabilize ==="
  wait_for_service bitcoin 180
  wait_for_service asm 300
  if [[ "$NO_ORCHESTRATOR" == "0" ]]; then
    wait_for_service orchestrator 120
  fi

  echo ""
  echo "=== waiting extra time for ASM to finish init ==="
  sleep 10

  echo ""
  echo "=== copying asm-params.json from asm container ==="
  local asm_container
  asm_container=$("${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" ps -q asm 2>/dev/null | head -n1)
  if [[ -n "$asm_container" ]]; then
    docker cp "$asm_container:/data/asm-params.json" "$REPO_DIR/scripts/asm-params.json" 2>&1 \
      && echo "OK asm-params.json → $REPO_DIR/scripts/asm-params.json" \
      || echo "WARN could not copy asm-params.json (asm may still be initializing)"
  else
    echo "WARN asm container not found"
  fi

  echo ""
  echo "=== stack is up ==="
  show_status
}

cleanup() {
  set +e
  echo ""
  echo "Stopping stack..."
  "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" down 2>/dev/null || true
  rm -f "$COMPOSE_DIR/.stack-pids" 2>/dev/null || true
  set -e
}

# ── Main ─────────────────────────────────────────────────────────────────────

if [[ "${HELP:-0}" == "1" ]]; then
  show_help
  exit 0
fi

if [[ "$STATUS" == "1" ]]; then
  show_status
  exit 0
fi

if [[ "$STOP" == "1" ]]; then
  do_stop
  exit 0
fi

# These commands don't need the stack to be running
if [[ "$MINE_COUNT" -gt 0 ]]; then
  do_mine "$MINE_COUNT"
  exit 0
fi

if [[ -n "$FUND_ADDRESS" ]]; then
  do_fund "$FUND_ADDRESS" "$FUND_AMOUNT"
  exit 0
fi

if [[ "$CLEAN" == "1" ]]; then
  do_clean
fi

if [[ "$NO_BUILD" == "0" ]]; then
  echo ""
  echo "=== docker compose build ==="
  "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" build
else
  echo ""
  echo "=== skipping build (--no-build) ==="
fi

trap 'echo "Interrupted"; do_stop; exit 1' INT TERM

do_start

echo ""
echo "Ready. To stop: ./local-stack.sh --stop"
echo "To clean and restart: ./local-stack.sh --clean"