# Scripts

Helper scripts for development and testing of the Alpen Multisig project.

## Table of Contents

- [Local Docker Stack](#local-docker-stack) — Full stack via Docker Compose
- [WebDriver E2E Tests](#webdriver-e2e-tests) — Full UI test suite on real Tauri binary
- [Bitcoin regtest node](#bitcoin-regtest-node) — Standalone bitcoind for ASM runner
- [ASM Runner](#asm-runner) — Strata ASM binary
- [Trezor emulator](#trezor-emulator) — Emulated Trezor device
- [Ledger emulator](#ledger-emulator-speculos) — Emulated Ledger device

---

## Local Docker Stack

Manages a complete local development stack via Docker Compose: Bitcoin, ASM, PostgreSQL, Orchestrator, and Regtest Dev API.

**Compose file:** `staging/docker-compose.local.yml`

### Prerequisites

- Docker and Docker Compose v2 in PATH
- ASM submodule populated: `git submodule update --init asm`

### Quick Start

```bash
# Start the full stack
./scripts/local-stack.sh

# Check status
./scripts/local-stack.sh --status

# Stop the stack
./scripts/local-stack.sh --stop

# Clean start (removes volumes)
./scripts/local-stack.sh --clean
```

### All Options

| Flag | Description |
|---|---|
| `--clean` | Remove `bitcoin-data` and `asm-data` volumes before starting |
| `--orchestrator` | With `--clean`: also prune orchestrator Docker build cache |
| `--regtest-dev-api` | With `--clean`: also prune regtest-dev-api Docker build cache |
| `--no-build` | Use existing Docker images (skip build) |
| `--no-orchestrator` | Don't start orchestrator container (for local `cargo run -p orchestrator-be`) |
| `--stop` | Stop all containers |
| `--status` | Show container status |
| `-h` | Show help |

### Services and Ports

| Service | Port | Description |
|---|---|---|
| `bitcoin` | 18443 | Bitcoin Core regtest RPC |
| `asm` | 8080 | Strata ASM runner admin RPC |
| `postgres` | 5432 | PostgreSQL database |
| `orchestrator` | 3000 | Backend API (skipped with `--no-orchestrator`) |
| `regtest-dev-api` | 3001 | Mining/faucet helper |

### Development Workflows

**Full stack locally:**
```bash
./scripts/local-stack.sh
```

**With orchestrator running from source (for hot reload):**
```bash
# Terminal 1: start stack without orchestrator
./scripts/local-stack.sh --no-orchestrator

# Terminal 2: run orchestrator from source
cargo run -p orchestrator-be

# Terminal 3: (optional) watch logs
docker compose -f staging/docker-compose.local.yml logs -f
```

**Clean rebuild of specific service:**
```bash
./scripts/local-stack.sh --clean --orchestrator
```

### Local helpers (no stack needed)

These commands work against the regtest-dev-api running in Docker:

```bash
# Mine blocks
./scripts/local-stack.sh --mine 5

# Fund an address (default: 1 BTC)
./scripts/local-stack.sh --fund bcrt1q... 0.5

# Mine 1 block
./scripts/local-stack.sh --mine
```

### Endpoints

```bash
# Health check
curl http://localhost:3000/api/v1/health

# Mine blocks
curl -X POST http://localhost:3001/mine?count=1

# Fund address
curl -X POST http://localhost:3001/faucet \
  -H "Content-Type: application/json" \
  -d '{"address":"bcrt1q...","amount_btc":1.0}'

# ASM status
curl -X POST http://localhost:8080/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"strata_asm_getStatus","id":1}'
```

---

## WebDriver E2E Tests

End-to-end UI tests for the Tauri desktop app using WebdriverIO + `tauri-driver` + WebKitWebDriver. Runs against the real Tauri binary with the full Docker stack.

### Prerequisites

| Requirement | How |
|---|---|
| Docker + Docker Compose | Already installed |
| Node 18+ | `nvm install 18` |
| `tauri-driver` | `cargo install tauri-driver --locked` |
| `WebKitWebDriver` | `sudo apt install webkit2gtk-driver` |
| Graphical session | Run from desktop (not SSH-only) |

### Quick Start

```bash
# Full run: clean stack → build Tauri → run all 4 e2e specs → stop stack
./scripts/run-e2e.sh --clean --stop-after
```

### All Options

| Flag | Description |
|---|---|
| `--clean` | Clean Docker volumes before starting the stack |
| `--no-build` | Skip Docker image build (use existing images) |
| `--stop-after` | Stop the stack after tests complete |
| `--skip-stack` | Assume the stack is already running (only run tests) |
| `--skip-tauri-build` | Skip Tauri binary build (use existing `target/debug/desktop-app`) |
| `-h` | Show help |

### Common Workflows

```bash
# First run (clean everything, full build)
./scripts/run-e2e.sh --clean --stop-after

# Re-run after code changes (reuse Docker images, rebuild Tauri)
./scripts/run-e2e.sh --stop-after

# Fast re-run (reuse Docker images + Tauri binary)
./scripts/run-e2e.sh --skip-stack --skip-tauri-build --stop-after

# Keep stack running after tests (for debugging)
./scripts/run-e2e.sh --skip-stack --skip-tauri-build
```

### Test Suite (4 specs, run in dependency order)

| # | Test | Purpose |
|---|---|---|
| 1 | `wallet-smoke` | Login with mnemonic → reach `/proposals` |
| 2 | `proposal-add-signer` | Create signer-update proposal + sign |
| 3 | `proposal-co-sign-mnemonic` | Co-sign with second mnemonic (quorum) |
| 4 | `proposal-broadcast-quorum` | Broadcast proposal → commit→reveal→confirmed |

Specs 2–4 are sequential: each depends on the previous one's state.

### Individual Specs (manual)

```bash
cd desktop-app/e2e-webdriver

# Build the Tauri binary first (if not already built)
cd ../ && npm run tauri build -- --debug --no-bundle && cd e2e-webdriver

# Run a single spec (skip Tauri rebuild)
SKIP_E2E_BUILD=1 npx wdio run wdio.conf.js --spec ./test/specs/wallet-smoke.e2e.js
SKIP_E2E_BUILD=1 npx wdio run wdio.conf.js --spec ./test/specs/proposal-add-signer.e2e.js
SKIP_E2E_BUILD=1 npx wdio run wdio.conf.js --spec ./test/specs/proposal-co-sign-mnemonic.e2e.js
SKIP_E2E_BUILD=1 npx wdio run wdio.conf.js --spec ./test/specs/proposal-broadcast-quorum.e2e.js
```

### Troubleshooting

| Symptom | Fix |
|---|---|
| `tauri-driver not found` | `cargo install tauri-driver --locked` or set `TAURI_DRIVER_PATH` |
| Port 4444 in use | `pkill -f tauri-driver` |
| Tests hang at login | Stack not running or `.env` URLs wrong |
| Broadcast fails at prepare | Check `desktop-app/.env` has `BITCOIN_RPC_*` vars matching Docker creds |
| `e2e-admin-wallet-external-address-0` not found | Admin Wallet sync failed — verify `BITCOIN_RPC_URL`, `BITCOIN_RPC_USER`, `BITCOIN_RPC_PASS` in `desktop-app/.env` |

---

## Bitcoin regtest node

## Prerequisites

- `bitcoind` / `bitcoin-cli` in PATH
- `trezord-go` in PATH (Trezor Bridge)
- `uv` (Python package manager used by the Trezor firmware repo)
- A local clone of [`trezor-firmware`](https://github.com/trezor/trezor-firmware)

## Configuration

[`config.json`](config.json) is read by the Trezor scripts. Copy and edit it before first use:

| Key | Description |
|---|---|
| `trezor_repo` | Absolute path to your local `trezor-firmware` clone |
| `mnemonic` | BIP-39 mnemonic loaded into the emulator |
| `trezor_port` | UDP port for the emulator (default: `21324`) |
| `trezor_model` | Emulator model code (default: `T2B1`). See table below. |

**Supported model codes:**

| Code | Device |
|---|---|
| `T1B1` | Trezor One |
| `T2T1` | Trezor Model T |
| `T2B1` | Trezor Safe 3 *(default)* |
| `T3T1` | Trezor Safe 5 |

```json
{
  "trezor_repo": "/path/to/trezor-firmware",
  "trezor_model": "T2B1",
  "mnemonic": "word1 word2 ... word12"
}
```

---

## Bitcoin regtest node

### Start

```bash
./scripts/bitcoind-asm-runner.sh start
```

Starts `bitcoind` in regtest mode with RPC and ZMQ settings that match `asm-config.toml`. Safe to call if already running — exits cleanly.

### Stop / Restart / Status

```bash
./scripts/bitcoind-asm-runner.sh stop
./scripts/bitcoind-asm-runner.sh restart
./scripts/bitcoind-asm-runner.sh status
```

### Environment overrides

| Variable | Default |
|---|---|
| `ASM_RUNNER_CONFIG` | `scripts/asm-config.toml` |
| `BITCOIND_DATADIR` | `~/.bitcoin/asm-runner-regtest` |

### Mine blocks

Use this to advance the chain (e.g. reach the genesis height of 101, or confirm transactions):

```bash
bitcoin-cli -regtest -datadir="$HOME/.bitcoin/asm-runner-regtest" \
  -rpcconnect=127.0.0.1 -rpcport=18443 -rpcuser=user -rpcpassword=password \
  generatetoaddress 144 "$(bitcoin-cli -regtest -datadir="$HOME/.bitcoin/asm-runner-regtest" \
    -rpcconnect=127.0.0.1 -rpcport=18443 -rpcuser=user -rpcpassword=password getnewaddress)"
```

---

## ASM Runner

The ASM Runner is an external binary (not part of this repo). It reads `asm-config.toml` and `asm-params.json` from the `scripts/` directory.

### Run with default config

```bash
cargo run --bin strata-asm-runner
```

### Run with explicit config files

```bash
cargo run --bin strata-asm-runner -- \
  --config ../scripts/asm-config.toml \
  --params ../scripts/asm-params.json
```

> Paths are relative to the crate root where the binary lives. Adjust as needed.

---

## Trezor emulator

### Start

```bash
./scripts/trezor-up.sh
```

Reads `trezor_repo` and `mnemonic` from `config.json`, then:

1. Kills any existing emulator processes
2. Starts `trezord-go` (bridge)
3. Starts the T2B1 emulator via `emu.py`
4. Loads the mnemonic and PIN (`1234`) into the device

#### Options

```bash
# Override the repo path without editing config.json
./scripts/trezor-up.sh /abs/path/to/trezor-firmware

# Compile firmware before starting (first run or after firmware changes)
./scripts/trezor-up.sh --build
./scripts/trezor-up.sh /abs/path/to/trezor-firmware --build

# Override the device model without editing config.json
./scripts/trezor-up.sh --model T3T1
./scripts/trezor-up.sh --model T1B1 --build
```

### Stop

```bash
./scripts/trezor-down.sh
```

Kills `trezord-go` and the emulator process.

### Logs

```
/tmp/trezord-go.log
/tmp/trezor-emu.log
```

---

## Ledger emulator (Speculos)

Requires Docker.

### Get the app binary

Download the Bitcoin app ELF for your target model from [app-bitcoin-new releases](https://github.com/LedgerHQ/app-bitcoin-new/releases):

| File | Model |
|---|---|
| `bitcoin_testnet_nanosp.elf` | Nano S+ *(recommended)* |
| `bitcoin_testnet_nanos.elf` | Nano S |
| `bitcoin_testnet_nanox.elf` | Nano X |

### Start

```bash
./scripts/ledger-up.sh ~/ledger-apps/bitcoin_testnet_nanosp.elf
# or for a different model:
./scripts/ledger-up.sh ~/ledger-apps/bitcoin_testnet_nanos.elf --model nanos
# optional BIP-39 seed (must match your dev wallet if you want the same addresses):
./scripts/ledger-up.sh ~/ledger-apps/bitcoin_testnet_nanosp.elf \
  --seed "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
```

Runs Speculos on `http://localhost:5001`.

### Configure the desktop app

Add to `desktop-app/.env`:

```
LEDGER_SPECULOS_URL=http://localhost:5001
```

Then restart `npm run tauri dev`. The Tauri backend will route all Ledger calls through the emulator instead of HID.

#### Manual vs auto approval

By default, broadcast PSBT signing on Speculos is **auto-approved** (the backend uploads `/automation` rules), so the flow runs without interaction — handy for the integration test / CI.

To observe the real on-device interaction (and exercise the frontend "Confirm on your device" prompt + the 180s timeout, just like a physical Ledger), disable it:

```
LEDGER_SPECULOS_URL=http://localhost:5001
LEDGER_SPECULOS_AUTO_APPROVE=0
```

With `LEDGER_SPECULOS_AUTO_APPROVE=0` you must approve the transaction yourself on the Speculos screen (web UI at the emulator port, or the emulated buttons).

### Physical Ledger device

Leave `LEDGER_SPECULOS_URL` commented out. Connect the device via USB, unlock it, and open the **Bitcoin** app before connecting from the desktop app.

---

## Typical local dev flow

```bash
# 1. Start Bitcoin node
./scripts/bitcoind-asm-runner.sh start

# 2. Mine enough blocks to reach genesis height
bitcoin-cli -regtest -datadir="$HOME/.bitcoin/asm-runner-regtest" \
  -rpcconnect=127.0.0.1 -rpcport=18443 -rpcuser=user -rpcpassword=password \
  generatetoaddress 101 "$(bitcoin-cli -regtest -datadir="$HOME/.bitcoin/asm-runner-regtest" \
    -rpcconnect=127.0.0.1 -rpcport=18443 -rpcuser=user -rpcpassword=password getnewaddress)"

# 3. Start ASM Runner
cargo run --bin strata-asm-runner -- \
  --config scripts/asm-config.toml \
  --params scripts/asm-params.json

# 4. (Optional) Start Trezor emulator for signing flows
./scripts/trezor-up.sh
```
