#!/usr/bin/env bash
# docker-start-stack.sh — Bootstrap the full Docker local stack (bitcoin + asm + orchestrator + regtest-dev-api).
#
# Usage:
#   ./docker-start-stack.sh --clean [--orchestrator] [--regtest-dev-api] [--no-build] [--no-orchestrator]
#   ./docker-start-stack.sh --status
#   ./docker-start-stack.sh --stop
#   ./docker-start-stack.sh -h
#
# Options:
#   --clean                 Clean volumes (bitcoin-data, asm-data) and residual state
#   --orchestrator          With --clean: also prune orchestrator build cache
#   --regtest-dev-api       With --clean: also prune regtest-dev-api build cache
#   --no-build              Skip docker build (use existing images)
#   --no-orchestrator       Don't start orchestrator (for local dev scenarios)
#
# Prerequisites:
#   - docker available in PATH
#   - asm/ submodule populated (run: git submodule update --init asm)
#
# Port map:
#   localhost:18443  → bitcoin (RPC)
#   localhost:8080   → asm (strata-asm-runner admin RPC)
#   localhost:3000   → orchestrator (backend API)
#   localhost:3001   → regtest-dev-api (mine/faucet)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_DIR="$SCRIPT_DIR"
COMPOSE_FILE="$COMPOSE_DIR/docker-compose.yml"

CLEAN=0
CLEAN_ORCHESTRATOR=0
CLEAN_REGTEST_DEV_API=0
NO_BUILD=0
NO_ORCHESTRATOR=0
STOP=0
STATUS=0

for arg in "$@"; do
  case "$arg" in
    --clean) CLEAN=1 ;;
    --orchestrator) CLEAN_ORCHESTRATOR=1 ;;
    --regtest-dev-api) CLEAN_REGTEST_DEV_API=1 ;;
    --no-build) NO_BUILD=1 ;;
    --no-orchestrator) NO_ORCHESTRATOR=1 ;;
    --stop) STOP=1 ;;
    --status) STATUS=1 ;;
    -h|--help) STATUS=1; HELP=1 ;;
    *)
      echo "usage: $0 [--clean [--orchestrator] [--regtest-dev-api]] [--no-build] [--no-orchestrator] [--stop] [--status] [-h]" >&2
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

require_cmd docker "install docker or add it to PATH"

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

# shellcheck disable=SC1090
source "$REPO_DIR/runtests/env.sh" 2>/dev/null || true

show_help() {
  cat <<'EOF'
docker-start-stack.sh — Bootstrap the full Docker local stack.

Usage:
  ./docker-start-stack.sh --clean [--orchestrator] [--regtest-dev-api] [--no-build] [--no-orchestrator]
  ./docker-start-stack.sh --status
  ./docker-start-stack.sh --stop
  ./docker-start-stack.sh -h

Options:
  --clean                 Clean volumes (bitcoin-data, asm-data) and residual state
  --orchestrator          With --clean: also prune orchestrator build cache
  --regtest-dev-api       With --clean: also prune regtest-dev-api build cache
  --no-build              Skip docker build (use existing images)
  --no-orchestrator       Don't start orchestrator container (use when running orchestrator locally)

Prerequisites:
  - docker in PATH
  - asm/ submodule populated (git submodule update --init asm)

Ports:
  18443  bitcoin      (RPC)
  8080   asm          (strata-asm-runner admin RPC)
  3000   orchestrator (backend API) — skipped with --no-orchestrator
  3001   regtest-dev-api (mine/faucet)

Volumes (clean with --clean):
  bitcoin-data  — bitcoin chain data
  asm-data      — ASM sled DB + asm-params.json
  postgres-data — NOT cleaned (intentional)

First run: bitcoin entrypoint creates 'staging' wallet + mines 101 blocks.
            asm entrypoint generates asm-params.json from template.

EOF
}

show_status() {
  echo ""
  echo "Docker compose stack: $COMPOSE_DIR"
  echo ""
  "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" ps --format table 2>/dev/null || true
  echo ""
  echo "Ports:"
  echo "  18443  bitcoin"
  echo "  8080   asm"
  if [[ "$NO_ORCHESTRATOR" == "1" ]]; then
    echo "  3000   orchestrator (skipped --no-orchestrator)"
  else
    echo "  3000   orchestrator"
  fi
  echo "  3001   regtest-dev-api"
}

do_stop() {
  echo "Stopping stack..."
  "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" down 2>/dev/null || true
  echo "OK stopped"
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
      status=$(docker inspect --format='{{.State.Health.Status}}' "local-${svc}-1" 2>/dev/null || echo "")
      if [[ -z "$status" ]]; then
        local running
        running=$(docker inspect --format='{{.State.Running}}' "local-${svc}-1" 2>/dev/null || echo "false")
        if [[ "$running" == "true" ]]; then
          echo " ready"
          return 0
        fi
      elif [[ "$status" == "healthy" ]]; then
        echo " healthy"
        return 0
      elif [[ "$status" == "<no value>" ]]; then
        local running
        running=$(docker inspect --format='{{.State.Running}}' "local-${svc}-1" 2>/dev/null || echo "false")
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
echo "Ready. To stop: ./docker-start-stack.sh --stop"
echo "To clean and restart: ./docker-start-stack.sh --clean"