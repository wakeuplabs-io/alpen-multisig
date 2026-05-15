# Desktop WebDriver smoke (Tauri)

End-to-end UI tests for the **Alpen Multisig** Tauri app using [WebdriverIO](https://webdriver.io/) and [`tauri-driver`](https://v2.tauri.app/develop/tests/webdriver/), aligned with the upstream [webdriver-example](https://github.com/tauri-apps/webdriver-example/tree/main/v2/webdriver/webdriverio).

**Platform:** Linux (and Windows) per [Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/). macOS desktop WebDriver is not supported by this stack.

## Prerequisites

### Ubuntu / Debian (WebKit WebDriver)

Install the native driver (provides the **`WebKitWebDriver`** binary):

```bash
sudo apt update
sudo apt install -y webkit2gtk-driver
which WebKitWebDriver
```

### Rust: `tauri-driver`

```bash
cargo install tauri-driver --locked
which tauri-driver   # usually ~/.cargo/bin/tauri-driver
```

If `npm run test:e2e` cannot find it, either add `~/.cargo/bin` to your **`PATH`** or set:

```bash
export TAURI_DRIVER_PATH="$HOME/.cargo/bin/tauri-driver"
```

| Requirement | Notes |
|-------------|--------|
| `WebKitWebDriver` on `PATH` | `which WebKitWebDriver` — often package **`webkit2gtk-driver`** (Debian/Ubuntu) |
| `tauri-driver` | `cargo install tauri-driver --locked` |
| Node 18+ | `npm` in this directory |
| Graphical session | Run from your desktop session (not SSH-only without display) |
| **Real backend stack** | bitcoind regtest, `strata-asm-runner`, orchestrator, Postgres — same as manual E2E |
| **Env files** | `orchestrator-be/.env`, `desktop-app/.env`, `desktop-app/src-tauri/.env` must point RPC/asm/orchestrator at your running services |

## Build the app under test

The WebDriver suite runs **`npm run tauri build -- --debug --no-bundle`** from **`desktop-app/`** on each run (unless **`SKIP_E2E_BUILD=1`**) so the Vite bundle is rebuilt and embedded in the binary. Plain **`cargo build -p desktop-app`** can leave an **out-of-date embedded UI** (e.g. missing new `data-testid` attributes).

If you use **`SKIP_E2E_BUILD=1`**, run a full Tauri debug build yourself after any frontend change:

```bash
cd desktop-app
npm run tauri build -- --debug --no-bundle
```

Binary output: **`target/debug/desktop-app`** (workspace root).

## Run tests

```bash
cd desktop-app/e2e-webdriver
npm install
npm run test:e2e
```

Skip the automatic Tauri build (if you already ran `npm run tauri build -- --debug --no-bundle`):

```bash
SKIP_E2E_BUILD=1 npm run test:e2e
```

## What the smoke test does

[`test/specs/wallet-smoke.e2e.js`](test/specs/wallet-smoke.e2e.js):

1. Fills the mnemonic textarea (demo regtest phrase).
2. Clicks **Palabras** then **Connect with words**.
3. Selects address row **#0** and **Continue →**.
4. Confirms authority step and **Select ->**.
5. Clicks **Authenticate with Trezor** (label is Trezor-specific; button has `data-testid="e2e-authenticate-submit"`).
6. Waits until the window URL contains **`/proposals`**.

Selectors use `data-testid` attributes on the React side (`e2e-*`).

## Troubleshooting

| Symptom | Likely fix |
|---------|------------|
| `tauri-driver` not found | `cargo install tauri-driver --locked` or set **`TAURI_DRIVER_PATH`** to the binary |
| Cannot connect to WebDriver | Install/start **`WebKitWebDriver`**; confirm nothing else uses port **4444** |
| Binary not found | From `desktop-app/`: `npm run tauri build -- --debug --no-bundle` |
| Test hangs at connect | Stack not running (ASM/orchestrator), or wrong `.env` URLs |
| `-28` / RPC errors | Bitcoin Core still loading — unrelated to WebDriver; fix bitcoind first |

## Further reading

- [Tauri v2 — WebdriverIO example](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)
- [Tauri v2 — WebDriver overview](https://v2.tauri.app/develop/tests/webdriver/)
