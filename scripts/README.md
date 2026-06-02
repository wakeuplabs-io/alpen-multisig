# Scripts

Helper scripts for running a local Bitcoin regtest node and a Trezor emulator stack needed for development and testing.

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
