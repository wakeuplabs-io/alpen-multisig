#!/usr/bin/env bash
# run-e2e.sh — Bring up the full Docker local stack and run all WebDriver e2e tests.
#
# Usage:
#   ./run-e2e.sh [--clean] [--no-build] [--stop-after] [--skip-stack] [--skip-tauri-build] [--help]
#
# Options:
#   --clean             Clean volumes before starting the stack
#   --no-build          Skip Docker image build (use existing images)
#   --stop-after        Stop the stack after tests complete
#   --skip-stack        Assume the stack is already running (only run tests)
#   --skip-tauri-build  Skip Tauri binary build (use existing binary at target/debug/desktop-app)
#   --help              Show this help
#
# Prerequisites:
#   - docker + docker compose
#   - Node 18+ with npm
#   - cargo (for tauri-driver)
#   - WebKitWebDriver (sudo apt install webkit2gtk-driver on Ubuntu/Debian)
#   - tauri-driver (cargo install tauri-driver --locked)
#   - curl and jq in PATH
#
# Test execution order (dependencies respected):
#   1. wallet-smoke              — independent
#   2. admin-wallet-panel         — independent (tests wallet panel lifecycle)
#   3. wallet-panel-sync-on-open  — independent
#   4. proposal-add-signer        — independent (creates a proposal)
#   5. proposal-co-sign-mnemonic  — requires add-signer first
#   6. proposal-broadcast-quorum  — requires co-sign first (quorum reached)
#   7. broadcast-mnemonic-walking-skeleton — requires a quorum-ready proposal

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

CLEAN=0
NO_BUILD=0
STOP_AFTER=0
SKIP_STACK=0
SKIP_TAURI_BUILD=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean) CLEAN=1 ;;
    --no-build) NO_BUILD=1 ;;
    --stop-after) STOP_AFTER=1 ;;
    --skip-stack) SKIP_STACK=1 ;;
    --skip-tauri-build) SKIP_TAURI_BUILD=1 ;;
    -h|--help)
      head -22 "$0" | tail -20
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      exit 1
      ;;
  esac
  shift
done

die() { echo "error: $*" >&2; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "'$1' not found in PATH — $2"
}

# ── Prerequisites ────────────────────────────────────────────────────────────

echo "=== checking prerequisites ==="
require_cmd docker "install docker"
require_cmd node "install Node.js 18+"
require_cmd npm "install npm"
require_cmd jq "install jq"
require_cmd curl "install curl"

if [[ "$SKIP_STACK" == "0" ]]; then
  require_cmd cargo "install Rust/cargo (needed for tauri-driver)"
fi

# Check tauri-driver
TAURI_DRIVER_BIN=""
if [[ -n "${TAURI_DRIVER_PATH:-}" ]]; then
  TAURI_DRIVER_BIN="$TAURI_DRIVER_PATH"
elif [[ -x "$HOME/.cargo/bin/tauri-driver" ]]; then
  TAURI_DRIVER_BIN="$HOME/.cargo/bin/tauri-driver"
elif command -v tauri-driver >/dev/null 2>&1; then
  TAURI_DRIVER_BIN="tauri-driver"
fi

if [[ -z "$TAURI_DRIVER_BIN" ]]; then
  echo "WARN: tauri-driver not found. Install with: cargo install tauri-driver --locked" >&2
  echo "      or set TAURI_DRIVER_PATH to the binary location." >&2
  die "tauri-driver required for WebDriver e2e tests"
fi
echo "  tauri-driver: $TAURI_DRIVER_BIN"

# Check WebKitWebDriver
if command -v WebKitWebDriver >/dev/null 2>&1; then
  echo "  WebKitWebDriver: $(which WebKitWebDriver)"
else
  die "WebKitWebDriver not found — install webkit2gtk-driver (sudo apt install webkit2gtk-driver)"
fi

echo ""

# ── Setup .env files ─────────────────────────────────────────────────────────

echo "=== setting up .env files ==="

# desktop-app/.env — read by Tauri at startup (not exposed to webview).
# WalletService reads BITCOIN_RPC_* env vars for Admin Wallet sync — must match
# the Docker stack credentials (user/password in docker-compose.local.yml).
DESKTOP_ENV="$REPO_DIR/desktop-app/.env"
cat > "$DESKTOP_ENV" <<'EOF'
VITE_ORCHESTRATOR_BASE_URL=http://127.0.0.1:3000/api/v1
BITCOIN_NETWORK=regtest
BITCOIN_MAGIC_BYTES_HEX=414c504e
BITCOIN_RPC_URL=http://127.0.0.1:18443
BITCOIN_RPC_USER=user
BITCOIN_RPC_PASS=password
STRATA_ADMIN_STATE_RPC_URL=http://127.0.0.1:8080
EOF
echo "  desktop-app/.env → created"

# orchestrator-be/.env — only needed if running orchestrator locally (not in Docker)
ORCHESTRATOR_ENV="$REPO_DIR/orchestrator-be/.env"
if [[ ! -f "$ORCHESTRATOR_ENV" ]]; then
  cat > "$ORCHESTRATOR_ENV" <<'EOF'
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
LOG_LEVEL=info
AUTH_CHALLENGE_TTL_MS=120000
AUTH_SESSION_TTL_MS=1800000
STRATA_ADMIN_STATE_RPC_URL=http://127.0.0.1:8080
BITCOIN_RPC_URL=http://127.0.0.1:18443
BITCOIN_RPC_USER=user
BITCOIN_RPC_PASS=password
BITCOIN_WALLET_NAME=staging
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/postgres
DEV_MINE_ENABLED=1
ALLOW_DEV_OPERATOR_KEY=1
BITCOIN_NETWORK=regtest
BITCOIN_MAGIC_BYTES_HEX=414c504e
ALLOW_INSECURE_HTTP_URL=1
ADMIN_WALLET_REGTEST_MNEMONIC=multiply toss magic exclude crawl obey garden black apart room village neglect
COMMIT_FUNDING=admin_wallet
EOF
  echo "  orchestrator-be/.env → created"
else
  echo "  orchestrator-be/.env → already exists (skipped)"
fi

echo ""

# ── Start the stack ──────────────────────────────────────────────────────────

if [[ "$SKIP_STACK" == "0" ]]; then
  echo "=== starting local stack ==="

  LOCAL_STACK_ARGS=()
  if [[ "$CLEAN" == "1" ]]; then
    LOCAL_STACK_ARGS+=(--clean)
  fi
  if [[ "$NO_BUILD" == "1" ]]; then
    LOCAL_STACK_ARGS+=(--no-build)
  fi

  # local-stack.sh does its own health checks and returns when the stack is ready.
  # Containers run in detached mode (docker compose up -d) so they stay up after exit.
  "$SCRIPT_DIR/local-stack.sh" "${LOCAL_STACK_ARGS[@]}"

  echo ""
  echo "=== stack is ready ==="
else
  echo "=== skipping stack start (--skip-stack) ==="
  echo ""
  echo "=== verifying stack health ==="
  # All checks via HTTP — the Docker stack exposes everything on localhost
  curl -sf -m 3 -X POST http://127.0.0.1:3001/mine >/dev/null 2>&1 || die "regtest-dev-api not reachable (port 3001)"
  curl -sf -m 3 -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"strata_asm_getStatus","id":1}' \
    http://127.0.0.1:8080/ >/dev/null 2>&1 || die "asm not reachable (port 8080)"
  curl -sf -m 3 http://127.0.0.1:3000/api/v1/health >/dev/null 2>&1 || die "orchestrator not reachable (port 3000)"
  echo "  all services healthy"
  echo ""
fi

# ── Build Tauri binary (once, shared by all specs) ────────────────────────────

DESKTOP_BINARY="$REPO_DIR/target/debug/desktop-app"

if [[ "$SKIP_TAURI_BUILD" == "1" ]]; then
  echo "=== skipping Tauri build (--skip-tauri-build) ==="
  if [[ ! -x "$DESKTOP_BINARY" ]]; then
    die "SKIP_TAURI_BUILD=1 but binary missing: $DESKTOP_BINARY"
  fi
  echo "  using existing binary: $DESKTOP_BINARY"
else
  echo "=== installing desktop-app dependencies ==="
  (cd "$REPO_DIR/desktop-app" && npm install --silent)
  echo "  npm install complete"
  echo ""

  echo "=== building Tauri binary (debug, no bundle) ==="
  echo "  This takes a few minutes on first run. Subsequent runs are faster."
  (cd "$REPO_DIR/desktop-app" && npm run tauri build -- --debug --no-bundle)
  if [[ ! -x "$DESKTOP_BINARY" ]]; then
    die "Tauri build succeeded but binary not found: $DESKTOP_BINARY"
  fi
  echo "  binary: $DESKTOP_BINARY"
fi
echo ""

# ── Install e2e dependencies ─────────────────────────────────────────────────

E2E_DIR="$REPO_DIR/desktop-app/e2e-webdriver"

echo "=== installing e2e dependencies ==="
npm install --prefix "$E2E_DIR" --silent
echo "  npm install complete"
echo ""

# ── Clean up stale tauri-driver processes ─────────────────────────────────────

echo "=== cleaning up stale processes ==="
pkill -f tauri-driver 2>/dev/null && echo "  killed stale tauri-driver processes" || echo "  no stale tauri-driver processes"
sleep 1
echo ""

# ── Run e2e tests ────────────────────────────────────────────────────────────

echo "========================================"
echo "  Running WebDriver e2e tests"
echo "========================================"
echo ""

TEST_RESULTS=()
FAILED=0

run_test() {
  local name=$1
  local spec=$2
  local spec_file="$E2E_DIR/$spec"

  if [[ ! -f "$spec_file" ]]; then
    echo "SKIP: $name (spec not found: $spec)"
    TEST_RESULTS+=("SKIP:$name")
    return 0
  fi

  echo ""
  echo "--- $name ---"
  echo "  spec: $spec"
  echo ""

  if (cd "$E2E_DIR" && SKIP_E2E_BUILD=1 npx wdio run wdio.conf.js --spec "./$spec"); then
    echo ""
    echo "PASS: $name"
    TEST_RESULTS+=("PASS:$name")
  else
    echo ""
    echo "FAIL: $name"
    TEST_RESULTS+=("FAIL:$name")
    FAILED=$((FAILED + 1))
  fi
}

# Run tests in dependency order:
# 1. wallet-smoke (independent)
# 2. admin-wallet-panel (independent — tests wallet panel lifecycle)
# 3. proposal-add-signer (creates a proposal)
# 4. proposal-co-sign-mnemonic (requires add-signer)
# 5. proposal-broadcast-quorum (requires co-sign → quorum reached)

run_test "wallet-smoke" "test/specs/wallet-smoke.e2e.js"
run_test "admin-wallet-panel" "test/specs/admin-wallet-panel.e2e.js"
run_test "proposal-add-signer" "test/specs/proposal-add-signer.e2e.js"
run_test "proposal-co-sign-mnemonic" "test/specs/proposal-co-sign-mnemonic.e2e.js"
run_test "proposal-broadcast-quorum" "test/specs/proposal-broadcast-quorum.e2e.js"

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "========================================"
echo "  Test Results Summary"
echo "========================================"
echo ""

PASSED=0
SKIPPED=0
for result in "${TEST_RESULTS[@]}"; do
  status="${result%%:*}"
  name="${result#*:}"
  case "$status" in
    PASS) echo "  ✅ PASS  $name"; PASSED=$((PASSED + 1)) ;;
    FAIL) echo "  ❌ FAIL  $name" ;;
    SKIP) echo "  ⏭️  SKIP  $name"; SKIPPED=$((SKIPPED + 1)) ;;
  esac
done

echo ""
echo "  Passed:  $PASSED"
echo "  Failed:  $FAILED"
echo "  Skipped: $SKIPPED"
echo "  Total:   ${#TEST_RESULTS[@]}"
echo ""

# ── Cleanup ──────────────────────────────────────────────────────────────────

if [[ "$STOP_AFTER" == "1" ]]; then
  echo "=== stopping stack (--stop-after) ==="
  "$SCRIPT_DIR/local-stack.sh" --stop
fi

if [[ "$FAILED" -gt 0 ]]; then
  echo "Some tests failed. Check output above for details."
  exit 1
else
  echo "All tests passed!"
  exit 0
fi
