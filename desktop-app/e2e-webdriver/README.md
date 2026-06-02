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

| Requirement                 | Notes                                                                                               |
| --------------------------- | --------------------------------------------------------------------------------------------------- |
| `WebKitWebDriver` on `PATH` | `which WebKitWebDriver` — often package **`webkit2gtk-driver`** (Debian/Ubuntu)                     |
| `tauri-driver`              | `cargo install tauri-driver --locked`                                                               |
| Node 18+                    | `npm` in this directory                                                                             |
| Graphical session           | Run from your desktop session (not SSH-only without display)                                        |
| **Real backend stack**      | bitcoind regtest, `strata-asm-runner`, orchestrator, Postgres — same as manual E2E                  |
| **Env files**               | `orchestrator-be/.env`, `desktop-app/.env` must point RPC/asm/orchestrator at your running services |

### Secret custody (Wave 2 Decision #2)

| Variable                        | When                                                                                                                                                                                                                               |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ADMIN_WALLET_REGTEST_MNEMONIC` | **Removed (Phase 3.7c).** Do not set in `.env`. E2E flows must log in via Palabras so `wallet_session_init` binds the session mnemonic.                                                                                            |
| `ALLOW_DEV_MNEMONIC_SIGNING=1`  | **No longer gates broadcast** (R1.1 — capability is per-signer via `allowed_on(network)`). Only needed to expose the mnemonic-login signing IPC in **release** builds; debug builds (`npm run tauri dev`) enable it automatically. |

See `docs/specs/secret-custody-wave2.md` (Track A) for full policy.

### Admin Wallet pre-funding (required for broadcast spec)

From Phase 3.6, the commit transaction is always funded from the Admin Wallet (BDK) — there is no fallback to node-wallet `sendtoaddress`. Before "Confirm & Broadcast", the Admin Wallet external address (`m/86'/0'/73'/0/0`) must hold spendable regtest UTXOs.

The `proposal-broadcast-quorum` spec handles this automatically: it calls `fundAdminWallet()` (`test/helpers/fund-admin-wallet.mjs`) after prepare-broadcast, which reads the Admin Wallet address from the broadcast screen's **Funding Source** card (`data-testid="e2e-admin-wallet-external-address-0"`) and funds it from the `asm-runner` wallet via `bitcoin-cli -rpcwallet=asm-runner sendtoaddress` + `generatetoaddress` — the same wallet-scoped path used by `mine-regtest-blocks.mjs` and `runtests/mine-blocks.sh`.

No manual funding step is required as long as the regtest stack (`runtests/env.sh` + `asm-runner` wallet) is running.

## Build the app under test

The WebDriver suite runs **`npm run tauri build -- --debug --no-bundle`** from **`desktop-app/`** on each run (unless **`SKIP_E2E_BUILD=1`**) so the Vite bundle is rebuilt and embedded in the binary. Plain **`cargo build -p desktop-app`** can leave an **out-of-date embedded UI** (e.g. missing new `data-testid` attributes).

If you use **`SKIP_E2E_BUILD=1`**, run a full Tauri debug build yourself after any frontend change:

```bash
cd desktop-app
npm run tauri build -- --debug --no-bundle
```

Binary output: **`target/debug/desktop-app`** (workspace root).

## Run tests

Specs are **not** all run by default: `npm run test:e2e` runs only the wallet smoke spec. Run other flows **one at a time** with the scripts below (they each trigger a Tauri build unless `SKIP_E2E_BUILD=1`).

```bash
cd desktop-app/e2e-webdriver
npm install
npm run test:e2e                    # wallet smoke only (canonical Admin ID)
npm run test:e2e:all                # every *.e2e.js spec in one run (discouraged for heavy flows)
npm run test:e2e:wallet-smoke       # same as default test:e2e
npm run test:e2e:proposal-add-signer   # create signer-update proposal (canonical Admin ID)
npm run test:e2e:proposal-broadcast-quorum # broadcast first quorum-ready proposal (manual step 3)
```

Skip the automatic Tauri build (if you already ran `npm run tauri build -- --debug --no-bundle`):

```bash
SKIP_E2E_BUILD=1 npm run test:e2e
SKIP_E2E_BUILD=1 npm run test:e2e:proposal-add-signer
SKIP_E2E_BUILD=1 npm run test:e2e:proposal-broadcast-quorum
```

### Manual two-step: create → broadcast

1. **`npm run test:e2e:proposal-add-signer`** — connects with the canonical Admin ID, creates and signs the draft.
2. **`npm run test:e2e:proposal-broadcast-quorum`** — connects again with the canonical Admin ID, opens the first **Broadcast** in **Quorum reached**, runs **Prepare broadcast** then **Confirm & Broadcast**, and waits for **Proposal enacted onchain** (`e2e-broadcast-done-banner`). Requires Bitcoin RPC / operator env vars in **`desktop-app/.env`** (see `use-broadcast-proposal`).

Broadcast uses the **first** `e2e-proposal-broadcast-button` in **Quorum reached**.

## What the tests do

### [`test/specs/wallet-smoke.e2e.js`](test/specs/wallet-smoke.e2e.js)

Shared steps live in [`test/helpers/login-mnemonic.mjs`](test/helpers/login-mnemonic.mjs). The smoke spec:

1. Fills the mnemonic textarea (demo regtest phrase).
2. Clicks **Palabras** then **Connect with words**.
3. Confirms authority step and **Select ->**.
4. Clicks **Authenticate with Trezor** (label is Trezor-specific; button has `data-testid="e2e-authenticate-submit"`).
5. Waits until the window URL contains **`/proposals`**.

### [`test/specs/proposal-add-signer.e2e.js`](test/specs/proposal-add-signer.e2e.js)

After the same login helper, clicks **Create proposal** on the dashboard (client-side route to **`/proposals/create`** — avoid `browser.url(…/proposals/create)` in Tauri builds: the custom protocol has no SPA fallback and returns “asset not found”). Then keeps **Signer update**, sets a title, adds compressed pubkey **`03dd6d7…427c`** via **+ Add**, opens **Preview and Create**, then **Sign and Create Proposal**, and waits for the **Signature collected** success panel (`data-testid="e2e-proposal-signature-success"`).

### [`test/specs/proposal-broadcast-quorum.e2e.js`](test/specs/proposal-broadcast-quorum.e2e.js)

Canonical Admin ID session after login; clicks the first **Broadcast** in **Quorum reached**, **Prepare broadcast**, **Confirm & Broadcast**, then waits for **`e2e-broadcast-done-banner`**. Needs regtest bitcoind + broadcast env in **`desktop-app/.env`** (RPC, mnemonic, magic bytes, ASM URL — see `desktop-app/.env.example`) and a **pre-funded Admin Wallet external address** (see "Admin Wallet pre-funding").

Selectors use `data-testid` attributes on the React side (`e2e-*`).

## Troubleshooting

| Symptom                         | Likely fix                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tauri-driver` not found        | `cargo install tauri-driver --locked` or set **`TAURI_DRIVER_PATH`** to the binary                                                                                                                                                                                                                                                                                                                  |
| Cannot connect to WebDriver     | Install/start **`WebKitWebDriver`**; confirm nothing else uses port **4444**                                                                                                                                                                                                                                                                                                                        |
| Binary not found                | From `desktop-app/`: `npm run tauri build -- --debug --no-bundle`                                                                                                                                                                                                                                                                                                                                   |
| Test hangs at connect           | Stack not running (ASM/orchestrator), or wrong `.env` URLs                                                                                                                                                                                                                                                                                                                                          |
| Broadcast spec fails at prepare | Check **`desktop-app/.env`** (Tauri process): `BITCOIN_RPC_*`, `BITCOIN_MAGIC_BYTES_HEX`, `STRATA_ADMIN_STATE_RPC_URL`, `BITCOIN_NETWORK` — see `desktop-app/.env.example`. Confirm mnemonic login ran (`wallet_session_init`) on regtest/testnet (the mnemonic signer is rejected on mainnet). Also ensure the Admin Wallet external address is pre-funded (see "Admin Wallet pre-funding" above). |

## Further reading

- [Tauri v2 — WebdriverIO example](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)
- [Tauri v2 — WebDriver overview](https://v2.tauri.app/develop/tests/webdriver/)
