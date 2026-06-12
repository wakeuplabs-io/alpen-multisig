# Admin Wallet (Mini Wallet) — Implementation Plan

Phase 1 delivers **US-H7** — see [`admin-wallet-regtest-commit-funding.md`](./admin-wallet-regtest-commit-funding.md).

**PRD compliance:** PASS / FAIL / PARTIAL per requirement is maintained in [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md). A phase or Release step marked ✅ here means the **engineering slice** shipped, not that every PRD MUST in that section is PASS.

## 1. Purpose and scope

The **Admin Wallet** is the signer's BIP-86 Taproot (`m/86'/0'/73'/n/n`) BTC custody layer used for mining-fee inputs, change, and (per PRD §4) Send/Receive. It is distinct from the **Admin ID** (`m/84'/0'/73'/0/0`, P2WPKH), which authenticates to the orchestrator and signs SPS-65 messages and must never sign Bitcoin transactions.

**In scope for this program**

- Authorities: **Strata Administrator** and **Alpen Administrator** only.
- Stack: `bdk_wallet` + **`bdk_electrum` for wallet sync/indexation** + **Bitcoin Core–compatible JSON-RPC** (`BITCOIN_RPC_URL`) for broadcast, fee estimates, and `submitpackage` — referred to below as **chain RPC** (protocol/transport), not “users must run Bitcoin Core.”
- **Release 2 (R2) ✅:** Electrum replaces block-by-block Core RPC sync — see [`admin-wallet-electrum-sync.md`](./admin-wallet-electrum-sync.md). Shipped (PRs #261, #262, #263).
- Governance commit/reveal: funding moves to Admin Wallet + BDK; protocol in [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md) unchanged.
- Later: PRD §4 wallet UI (Alta handoff), Send validations, fee-bump, receive rotation, Admin ID display, shared Send UX, direct Trezor/Ledger (no HWI).

**Explicit exclusions (not planned in any phase below)**

- Payout Administrator (`block_payout`, P2TR Admin ID for payout, US-I*, PRD §6).
- HWI (`hwi` CLI, POC-miniwallet HWI integration).
- Any indexer backend other than **Electrum** (R2).

**External references (visual / POC only — not workspace deps)**

- Alta UI: `miniwallet/Alpen-v0.1-Alta-handoff/Alpen v0.1 - Alta/` — WalletPanel, S9/S11 broadcast UX.
- POC: `miniwallet/poc-miniwallet/frontend` — reference only.

## 2. Traceability

| Phase | Name | Stories / specs |
|---|---|---|
| 1 ✅ | Regtest commit funding | US-H7, [`admin-wallet-regtest-commit-funding.md`](./admin-wallet-regtest-commit-funding.md) |
| 2 ✅ | Wallet core read path | PRD §4.3 read APIs (balance, UTXOs, addresses) — not §4.1–4.2 Admin ID; [`admin-wallet-core-read-path.md`](./admin-wallet-core-read-path.md) |
| 3 ✅ | Wallet UI shell | PRD §4.3 slide-over shell (Alta-inspired); full Alta tabs deferred to Phases 4–6 |
| 3.5 ✅ | Retire operator hot key | PRD §3.2 — folded reveal internal key into Admin Wallet seed at `m/86'/0'/73'/2/0` (superseded by R1.0: ephemeral reveal key) |
| 3.6 ✅ | Admin Wallet–only commit funding | Remove `BitcoindSendToAddress` variant and `COMMIT_FUNDING` toggle; Admin Wallet (BDK) is the sole commit funder from this phase onward |
| 3.7 ✅ | Session-bound Admin Wallet (mnemonic) | PRD §3.2 — wallet/commit/broadcast key from login session; `ADMIN_WALLET_REGTEST_MNEMONIC` removed (3.7c), [`admin-wallet-session-bound-mnemonic.md`](./admin-wallet-session-bound-mnemonic.md) |
| 3.8 ✅ | Watch-only Admin Wallet (HW login) | PRD §3.2 — HW login path gets a read-only BDK wallet from xpub; balance/addresses visible, signing deferred to R1.1 (broadcast) / Phase 8 (Send-on-HW) |
| R1.0 ✅ | Ephemeral reveal key | SPS-50 — per-broadcast envelope key, reveal change → Admin Wallet; supersedes `m/86'/0'/73'/2/0`; merged PR #195 |
| R1.0.1 ✅ | Sign commit + reveal before broadcast | SPS-50 — pre-sign both, broadcast commit→reveal (`submitpackage` if available, else sequential); closes the R1.0 crash window via atomicity; merged PR #198, [`admin-wallet-presign-commit-reveal.md`](./admin-wallet-presign-commit-reveal.md) |
| R1.1 ✅ | Session-driven broadcast signing (adds HW path) | PRD §3.2, §5.3.3, [`admin-wallet-session-driven-broadcast-signing.md`](./admin-wallet-session-driven-broadcast-signing.md) — unified `PsbtSigner` driven port; mnemonic login = software signer (simulated HW), HW login = on-device PSBT signer; reveal by ephemeral key; `ALLOW_DEV_MNEMONIC_SIGNING` replaced by per-signer network capability |
| R1.2 ✅ | Clean wallet UI | PRD §4, Alta WalletPanel, [`admin-wallet-clean-wallet-ui.md`](./admin-wallet-clean-wallet-ui.md) |
| R1.3 ✅ | Receive rotation | PRD §4.3.4 **rotation only** (PARTIAL in compliance matrix); QR/HW verify → Phase 7; [`admin-wallet-receive-rotation.md`](./admin-wallet-receive-rotation.md) |
| R1.4 ✅ | Remove connect-time derivation picking | PRD §3.2 — canonical paths only, [`admin-wallet-canonical-connect-paths.md`](./admin-wallet-canonical-connect-paths.md) |
| R1.5 ✅ | Balance UX (§4.3.1 PASS) | PRD §4.3.1 **PASS** in [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md); [`admin-wallet-balance-ux.md`](./admin-wallet-balance-ux.md); PR [#211](https://github.com/wakeuplabs-io/alpen-multisig/pull/211) |
| R1.6 ✅ | Addresses UX (§4.3.2 PASS) | PRD §4.3.2 **PASS** in compliance matrix; [`admin-wallet-addresses-ux.md`](./admin-wallet-addresses-ux.md); PR [#212](https://github.com/wakeuplabs-io/alpen-multisig/pull/212) |
| R1.7 ✅ | Wallet panel UI polish | Visual hierarchy + affordances; [`admin-wallet-wallet-panel-ui-polish.md`](./admin-wallet-wallet-panel-ui-polish.md); PR [#214](https://github.com/wakeuplabs-io/alpen-multisig/pull/214) |
| **R2** ✅ | **Electrum wallet sync (priority)** | PRD §2 production viability — [`admin-wallet-electrum-sync.md`](./admin-wallet-electrum-sync.md); slices **R2.1 → R2.2 → R2.3** |
| R2.1 ✅ | Electrum indexer infra | electrs in Docker + dev/staging/CI; synced to local regtest `bitcoind`; smoke verification |
| R2.2 ✅ | Admin Wallet sync migration | `WalletService` sync via `bdk_electrum`; fixed URL; broadcast/fees unchanged |
| R2.3 ✅ | Electrum URL in Node Config | Same pattern as BTC RPC / Strata — Local, Trusted, Custom |
| **4** ✅ | **Governance broadcast fee rate** | **US-H4** — sat/vB on commit broadcast; default from chain RPC; [`governance-broadcast-fee-selection.md`](./governance-broadcast-fee-selection.md); [`governance-broadcast-fee-selection-implementation.md`](./governance-broadcast-fee-selection-implementation.md); [`02-prd-update-impact.md`](../1-proposal/02-prd-update-impact.md) |
| 5 ✅ | Transactions + fee-bump | PRD §4.3.3 (RBF for sends, CPFP for governance commits); [`admin-wallet-transactions-fee-bump.md`](./admin-wallet-transactions-fee-bump.md) |
| 6 | Send BTC happy path | PRD §4.3.5 (regtest, dev mnemonic); reuses Phase 4 fee control pattern; [`admin-wallet-send-btc.md`](./admin-wallet-send-btc.md) — slices **P6.1 → P6.2 → P6.3 → P6.4** |
| 7 | Admin ID UI (receive rotation → R1.3) | PRD §4.1–4.2 |
| 8 | HW adapters — Send-on-HW (broadcast signing → R1.1) | PRD §3.2 (Trezor/Ledger PSBT, no HWI) |
| 9 | Shared Send + governance broadcast UX | Alta S9/S11, PRD §5.3.2 (shared Send chrome; fee entry → Phase 4/5) |
| 10 | Hardening + remote testnet/mainnet RPC | PRD §2 (no local node assumption) |

## 3. Architecture

### Components

```text
React (desktop-app/src)
  └─ IPC invoke (no secrets)
        └─ Tauri admin_wallet module
             ├─ bdk_wallet (descriptors, build, sign)
             ├─ bdk_electrum → Electrum server (wallet sync ✅, fee estimation ✅, broadcast ✅)
             ├─ HttpBitcoinRpcClient → chain RPC (BITCOIN_RPC_URL; broadcast fallback, fee estimation)
             └─ WalletService (commit funding, Send, fee inputs)
```

- **Secrets and signing** stay in Rust (Tauri). React shows addresses, balances, and confirmation UX only.
- **WalletService** is the single Rust service for commit funding, Send, and governance fee inputs. (Phase 1's pluggable `CommitFunding` abstraction was removed in Phase 3.6; the Admin Wallet/BDK is now the sole funder.)
- **Wallet sync (R2 ✅):** balance, UTXOs, addresses, and receive rotation sync via **Electrum** (`bdk_electrum`). Core RPC block-by-block sync retired.
- **Fee estimation (Phase 4 ✅):** `FeeEstimationService` with node RPC primary → Electrum fallback → cache → static fallback. `FeeRateSelector` UI in broadcast screens.
- **Broadcast (Phase 4 M3 ✅):** Electrum-first broadcast via `TxBroadcaster` port, node RPC fallback, manual copy-hex escape hatch.
- **Reveal** uses a per-broadcast **ephemeral** internal key (R1.0; SPS-50 script-path spend — not HW-signable). Phase 3.5/3.7b briefly used `m/86'/0'/73'/2/0`; that path is **retired**. Reveal change goes to Admin Wallet; **commit funding** is session-driven and HW-signable via `PsbtSigner` (R1.1).

### Chain access end state

| Role | Backend | Config |
|---|---|---|
| Wallet sync / indexation | **Electrum** (`bdk_electrum`) ✅ | Electrum URL (trusted preset or custom) — R2.3 |
| Fee estimation | **Node RPC** primary → **Electrum** fallback → cache → static fallback | Same endpoints; Phase 4 |
| Broadcast, `submitpackage` | **Electrum** primary → **Core-compatible JSON-RPC** fallback | Phase 4 M3 |

| Environment | Electrum | Chain RPC | Local `bitcoind` |
|---|---|---|---|
| Dev / CI | Local electrs against regtest `bitcoind` (R2.1 ✅) | `http://127.0.0.1:18443` via `scripts/bitcoind-asm-runner.sh` | Yes, scripts/CI only |
| Production end state (Phase 10) | Remote testnet/mainnet Electrum (trusted preset or custom) | Remote testnet/mainnet RPC (trusted preset or custom URL per PRD §2) | No product assumption |

**What went away (R2):** Local full node as a wallet-sync requirement; Core RPC block-scan for wallet sync (`bdk_bitcoind_rpc::Emitter` retired for read path).

**What stays:** Electrum for wallet indexation; a Bitcoin Core–compatible RPC **client** as broadcast fallback and fee estimation source; Electrum as primary broadcast path (Phase 4 M3).

### Phase dependency diagram

```mermaid
flowchart LR
  P1[Phase 1 Commit funding] --> P2[Phase 2 Read path]
  P2 --> P3[Phase 3 UI shell]
  P3 --> P35[Phase 3.5 Retire operator hot key]
  P35 --> P36[Phase 3.6 Admin Wallet-only commit funding]
  P36 --> P37[Phase 3.7 Session-bound wallet mnemonic]
  P37 --> P38[Phase 3.8 Watch-only wallet HW]
  P38 --> R1[Release 1: R1.0–R1.7 done]
  R1 --> R21[R2.1 Indexer infra ✅]
  R21 --> R22[R2.2 Wallet sync ✅]
  R22 --> R23[R2.3 Node Config URL ✅]
  R23 --> P4[Phase 4 Broadcast fee rate ✅]
  P4 --> P5[Phase 5 Tx list + fee-bump]
  P5 --> P6[Phase 6 Send happy path]
  P6 --> P7[Phase 7 Admin ID UI]
  P7 --> P8[Phase 8 HW Send-on-HW]
  P8 --> P9[Phase 9 Shared Gov + Send UX]
  P9 --> P10[Phase 10 Remote RPC hardening]
```

## 4. Phased plan

The plan has four parts: the completed **Foundation** (Phases 1–3.8), the completed **Release 1** (R1.0–R1.7), the completed **Release 2** (R2.1–R2.3 — Electrum wallet sync), and **Remaining phases (4–10)**. **Phase 4** (governance broadcast fee rate, US-H4) and **Phase 5** (Transactions + fee-bump, PRD §4.3.3) are also complete. **Next:** Phase 6 (Send BTC happy path). PRD status: [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md).

### Foundation (Phases 1–3.8) — done

Commit funding, wallet read path, UI shell, operator-key retirement, Admin-Wallet-only funding, session-bound mnemonic wallet, and watch-only HW wallet are all complete.

#### Phase 1 — Regtest commit funding (BDK + chain RPC)

**Goal:** US-H7 — fund governance commit from Admin Wallet on regtest; CI keeps legacy funding.

**In scope**

- Workspace crates: `bdk_wallet`, `bdk_bitcoind_rpc`.
- Descriptors: `tr(m/86'/0'/73'/0/*)` and `tr(m/86'/0'/73'/1/*)` on regtest.
- `CommitFunding` trait: `BitcoindSendToAddress` (default) | `BdkAdminWalletMnemonic`.
- Replace commit `sendtoaddress` only inside `broadcast_commit_then_reveal`; reveal unchanged.
- Env: `COMMIT_FUNDING`, `ADMIN_WALLET_REGTEST_MNEMONIC`, `ALLOW_DEV_MNEMONIC_SIGNING`, regtest guards.
- Minimal broadcast UI: mode, Admin Wallet address, balance.
- Tests: unit derivation; CI default legacy; manual regtest with flag.

**Out of scope**

- Full wallet tabs, Send form, HW commit sign, mainnet/testnet wallet features beyond R2 scope, Payout.

**Done when**

- With `COMMIT_FUNDING=admin_wallet` on regtest, an approved proposal commit is paid from `m/86'/0'/73'/0/0` and change goes to `…/1/*`; reveal and orchestrator PATCH match US-H6 behavior.
- CI/E2E pass with default legacy funding unchanged.

**Primary code areas**

- `desktop-app/src-tauri/Cargo.toml` — BDK deps
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/`
- `desktop-app/src-tauri/src/application/proposals.rs` — funding hook
- `desktop-app/src/screens/` (broadcast screen) — funding mode + balance
- `docs/specs/admin-wallet-regtest-commit-funding.md`

---

#### Phase 2 — Wallet core read path

**Spec:** [`admin-wallet-core-read-path.md`](./admin-wallet-core-read-path.md) — full technical design, IPC contracts, test plan.

**Goal:** BDK sync, balance, UTXOs, address list for Admin Wallet without Send UI.

**In scope:** `WalletService` read APIs over IPC; chain RPC sync; external/internal index display.

**Out of scope:** Send, fee-bump, HW signing, governance UX merge.

**Done when:** Signer sees correct balance and funded addresses on regtest via chain RPC.

**Primary code areas:** `admin_wallet` module, IPC commands, thin React hooks.

---

#### Phase 3 — Wallet UI shell

**Goal:** Introduce an Alta-inspired **slide-over** wallet panel (not a full tabbed WalletPanel route). Phase 3 shipped placeholders for Send, Transactions, and QR; **R1.2** removed those placeholders; **R1.3+** added receive rotation and production balance/address rows.

**In scope:** React slide-over shell, routing hooks, empty/loading states; visual reference `miniwallet/Alpen-v0.1-Alta-handoff/`.

**Out of scope:** Production Send, tx list, QR, HW verify (Phases 5–8); PRD §4.1–4.2 Admin ID UI (Phase 7).

**Done when:** Signer can open the panel from the dashboard/broadcast flows and see Phase 2 read data (superseded in UX by R1.2–R1.6).

**Primary code areas:** `desktop-app/src/domain/admin-wallet/components/`, dashboard/broadcast screens.

---

#### Phase 3.5 — Retire operator hot key (interim Admin Wallet derivation) ✅

**Goal:** Eliminate `OPERATOR_SECRET_KEY_HEX` as a separate hot key in environment. Derive the SPS-50 commit/reveal internal key from the Admin Wallet seed at a dedicated path so that — per PRD §3.2 — no signing material lives outside the Admin Wallet's secret zone. HW-mediated signing is deferred to Release 1 (R1.1); this phase keeps the dev mnemonic as the secret source, but consolidates it into a single key custody surface.

**Rationale:** The PRD never specifies a separate operator key. All signing flows are HW-wallet mediated (§3.2.2.5, §4.3.5.5.1, §5.3.3.2.2). `OPERATOR_SECRET_KEY_HEX` is dev scaffolding from POC days; carrying it as a parallel hot key through Phase 8 unnecessarily widens the secret-management surface. Retiring it before Phase 6 Send means the Send pipeline and the reveal pipeline share one signer infrastructure, which Release 1 (R1.1) then swaps to HW in a single coherent change.

**In scope**

- Derivation path: `m/86'/0'/73'/2/0` — dedicated chain `2` for SPS-50 commit/reveal internal key (distinct from external `0/*` and change `1/*`).
- `infrastructure/broadcast_env.rs`: drop `OPERATOR_SECRET_KEY_HEX` parsing; load the operator keypair via BDK from the Admin Wallet descriptor at the new path. Same `ALLOW_DEV_MNEMONIC_SIGNING` guard as Phase 1.
- Remove `OPERATOR_SECRET_KEY_HEX`, `ALLOW_DEV_OPERATOR_KEY`, and the well-known test-key rejection logic — superseded by the mnemonic guard.
- Update `application/proposals.rs`: `broadcast_commit_then_reveal` continues to receive an `&UntweakedKeypair`; only its source changes.
- Update `proposal-broadcast-commit-reveal.md` spec to reflect Admin Wallet-derived commit internal key.
- Update `desktop-app/e2e-webdriver/README.md`, CI workflows, and regtest scripts to drop `OPERATOR_SECRET_KEY_HEX` from setup recipes.
- Tests: `broadcast_env.rs` regressions; integration test that verifies the commit address is reproducible from the dev mnemonic; orchestrator claim/PATCH unchanged.

**Out of scope**

- HW PSBT signing for reveal (Release 1, R1.1).
- Removing `ADMIN_WALLET_REGTEST_MNEMONIC` (Phase 10 / 3.7c done).
- Changing SPS-50/51 envelope shape — only the internal key source changes; protocol semantics preserved.

**Done when**

- `OPERATOR_SECRET_KEY_HEX` and `ALLOW_DEV_OPERATOR_KEY` no longer exist in code, env, `.env.example`, runbooks, CI, or E2E setup.
- On regtest with `ALLOW_DEV_MNEMONIC_SIGNING=1`, commit and reveal still succeed; orchestrator txids and `PATCH` behavior unchanged.
- The commit address for a given proposal is deterministic from `ADMIN_WALLET_REGTEST_MNEMONIC` + payload.
- Regression suite (Phase 1 + Phase 2) green; no operator-key references remain in workspace `grep`.

**Primary code areas**

- `desktop-app/src-tauri/src/infrastructure/broadcast_env.rs` — replace env parsing with BDK derivation
- `desktop-app/src-tauri/src/infrastructure/broadcast_tx.rs` — no API change; consumes the same `UntweakedKeypair`
- `desktop-app/src-tauri/src/application/proposals.rs` — wiring only
- `docs/specs/proposal-broadcast-commit-reveal.md` — protocol-doc update
- `desktop-app/e2e-webdriver/README.md`, `scripts/`, CI workflows — env recipe cleanup

**Risks / notes**

- **Breaking change on regtest:** commit addresses change (different internal key). Acceptable because regtest state is ephemeral; document in changelog and reset E2E fixtures.
- **No on-chain consequence in production:** no mainnet/testnet state exists yet; this is a one-time clean swap.
- **Superseded by R1.0:** R1.0 replaces this seed-derived key at `m/86'/0'/73'/2/0` with a per-broadcast ephemeral key (the SPS-50 reveal is a script-path spend a HW cannot sign). Phase 3.5 still stands as the step that retired the standalone operator hot key.

---

#### Phase 3.6 — Admin Wallet–only commit funding ✅

**Status:** Complete — merged to `develop` as PR #187 (08dd1d4).
**Spec:** [`admin-wallet-commit-funding-only.md`](./admin-wallet-commit-funding-only.md).

**Goal:** Remove the `BitcoindSendToAddress` variant and the `COMMIT_FUNDING` environment variable toggle. From this phase onward, the commit transaction is always funded by the Admin Wallet (BDK), with no fallback to node-wallet `sendtoaddress`. This eliminates the dual-path bifurcation introduced in Phase 1 and ensures all development and testing work against the real Admin Wallet funding path.

**Rationale:** Phase 1 introduced `CommitFunding` as a pluggable trait to allow a gradual migration — CI/E2E could keep the legacy `sendtoaddress` path while the Admin Wallet path was validated. With Phase 3.5 complete (internal key consolidated) and Phase 3.7 (session-bound wallet) next in line, continuing to maintain two funding paths means all subsequent development — including Phase 6 Send and Phase 8 HW signing — would be built and tested against the wrong (legacy) path. Removing the bifurcation now ensures the Admin Wallet is the single source of truth for commit funding from this point forward.

**In scope**

- Remove `BitcoindSendToAddress` struct and its `CommitFunding` implementation from `application/commit_funding.rs`.
- Remove the `select_commit_funding` factory function and the `COMMIT_FUNDING` env var dispatch logic.
- `broadcast_commit_then_reveal` always uses `BdkAdminWalletMnemonic` (or its evolved `WalletService` form); no env-var switching.
- Remove `COMMIT_FUNDING` and `BITCOIN_WALLET_NAME` from `.env.example`, runbooks, CI workflows, scripts, and staging config.
- Update tests: remove tests that exercise `BitcoindSendToAddress` or the `bitcoind` mode; keep and strengthen BDK Admin Wallet funding tests.
- CI pipeline migrated to use `admin_wallet` mode exclusively — `ADMIN_WALLET_REGTEST_MNEMONIC` + funded Admin Wallet address as the only funding path.
- E2E regtest playbook updated: fund the Admin Wallet external address before running broadcast specs.

**Out of scope**

- Removing `ADMIN_WALLET_REGTEST_MNEMONIC` itself (done in 3.7c).
- Session-binding the wallet to login (Phase 3.7).
- Hardware wallet signing for commit (Release 1, R1.1).
- `CommitFunding` trait itself may be retained as an abstraction if R1.1 HW signing needs it — evaluate at implementation time. If the trait has no other implementors, remove it entirely.

**Done when**

- `BitcoindSendToAddress`, `select_commit_funding`, and `COMMIT_FUNDING` no longer exist anywhere in the codebase (code, env files, docs, CI, scripts).
- `BITCOIN_WALLET_NAME` removed from all broadcast-related configuration (may be retained if still needed for other RPC calls — evaluate at implementation time).
- On regtest with `ALLOW_DEV_MNEMONIC_SIGNING=1` and a funded Admin Wallet, commit and reveal succeed; orchestrator `PATCH` behavior unchanged.
- CI green with the Admin Wallet as the sole commit funder.
- `cargo test --workspace` and frontend CI pass.

**Primary code areas**

- `desktop-app/src-tauri/src/application/commit_funding.rs` — remove `BitcoindSendToAddress`, `select_commit_funding`, and `COMMIT_FUNDING` dispatch
- `desktop-app/src-tauri/src/commands/proposals.rs` — wire directly to BDK funding path; remove `select_commit_funding` call
- `desktop-app/.env.example`, CI workflows, `scripts/`, `staging/docker-compose.yml` — remove `COMMIT_FUNDING` and `BITCOIN_WALLET_NAME` from broadcast recipes
- `docs/specs/admin-wallet-regtest-commit-funding.md` — update to reflect single funding path

**Risks / notes**

- **CI regtest funding:** CI must have an Admin Wallet address pre-funded before running broadcast specs. Add a setup step to fund `m/86'/0'/73'/0/0` from the coinbase wallet before broadcast tests.
- **`BITCOIN_WALLET_NAME` scope:** verify whether any non-broadcast code path still uses `BITCOIN_WALLET_NAME` before removing it. If so, retain it scoped to that path only.
- **Release 1 (R1.1) future:** HW signing for commit will replace the BDK mnemonic signer at the `WalletService` level, not at the `CommitFunding` level. No further changes to the commit-funding wiring are expected after this phase.

---

#### Phase 3.7 — Session-bound Admin Wallet (mnemonic login) ✅

**Status:** Complete (3.7a session slot + 3.7b session-bound commit/reveal key + 3.7c `ADMIN_WALLET_REGTEST_MNEMONIC` removed). See [evolution](../evolution/2026-05-28-admin-wallet-session-bound-mnemonic.md) and [roadmap Phase 06](../feature/admin-wallet-session-bound-mnemonic/deliver/roadmap.json).
**Spec:** [`admin-wallet-session-bound-mnemonic.md`](./admin-wallet-session-bound-mnemonic.md).

**Goal:** Bind the `WalletService` lifecycle **and** the SPS-50 commit/reveal internal key to the user's login session so that when the user logs in with "Palabras" (dev mnemonic), the Admin Wallet, commit funding, and reveal signing material all derive from *that same mnemonic* — not from a separate `ADMIN_WALLET_REGTEST_MNEMONIC` env var. Closes the PRD §3.2 gap where Admin Wallet, Admin ID, and broadcast key were sourced independently.

**Rationale:** The PRD specifies a single hardware wallet as the source of both Admin ID (`m/84'/0'/73'/0/0`) and Admin Wallet (`m/86'/0'/73'/n/n`). Today `ADMIN_WALLET_REGTEST_MNEMONIC` is an independent env var with no runtime enforcement that it matches the login session. Any mismatch silently shows the wrong wallet. Phase 3.7 makes the Admin Wallet a first-class property of the session.

**In scope**

- `WalletService` becomes session-scoped: initialized at login time from the session mnemonic, cleared at logout. Tauri managed state changes from `Arc<WalletService>` (fixed at startup) to `Arc<RwLock<Option<WalletService>>>` (replaced per session).
- Mnemonic login (`auth_complete` IPC path): after successful orchestrator auth, derive and register the `WalletService` from the same mnemonic used to derive the Admin ID. Mnemonic never leaves Rust.
- Logout (`auth_logout` IPC path): drop the `WalletService` from managed state; panel returns to `Disabled` state.
- **3.7b:** Session-bound commit/reveal key at `m/86'/0'/73'/2/0` — cached in `WalletSession` at init; `load_broadcast_env` resolves keypair via session (not env) when logged in; `proposals_prepare_broadcast` takes `WalletSession` state.
- **`ADMIN_WALLET_REGTEST_MNEMONIC` removed (3.7c):** no prod/test reads; mnemonic only via `wallet_session_init`.
- `ALLOW_DEV_MNEMONIC_SIGNING` guard remains but is now implied by the mnemonic login type; still required as an explicit opt-in for regtest.
- HW login path: `WalletService` is not initialized (stays `None` / `Disabled`) until Phase 3.8 handles it.
- Tests: session init/teardown unit tests; regression that balance and addresses returned by IPC match the session mnemonic's derived wallet, not the env var wallet.

**Out of scope**

- HW login (Phase 3.8).
- Send/signing from the session wallet (Phase 6+).
- **Historical note:** early drafts kept `ADMIN_WALLET_REGTEST_MNEMONIC` for CI; **3.7c removed it** — mnemonic only via login session.

**Done when**

- Logging in with mnemonic A shows wallet A in the panel (no independent env mnemonic).
- **3.7b (historical):** commit/reveal key from session at `m/86'/0'/73'/2/0` until **R1.0** replaced reveal with ephemeral keys.
- Logging out clears the wallet panel to `Disabled` state and drops session wallet state.
- `cargo test --workspace` and full frontend CI green.

**Primary code areas**

- `desktop-app/src-tauri/src/main.rs` — managed state type change
- `desktop-app/src-tauri/src/commands/authentication.rs` — `auth_complete` initializes `WalletService`; `auth_logout` drops it
- `desktop-app/src-tauri/src/application/wallet_service.rs` — session lifecycle API
- `desktop-app/src-tauri/src/commands/admin_wallet.rs` — all commands guard against `None` session (return `Disabled`)

**Risks / notes**

- **Concurrent IPC calls during login**: brief window between `auth_complete` and first wallet IPC. Commands must handle `None` state gracefully (return `Disabled`, not panic).
- **Phase 3.8 handoff**: HW login calls the same session-init slot but passes an xpub instead of a mnemonic (watch-only). The `WalletService` API surface established here is the extension point Phase 3.8 fills; HW signing then lands in Release 1 (R1.1).

---

#### Phase 3.8 — Watch-only Admin Wallet (HW login) ✅

**Status:** Complete — merged to `develop` as PR #190 (5f9fffd).

**Goal:** When the user logs in with a hardware wallet (Trezor/Ledger), derive a **watch-only** BDK wallet from the device xpub at `m/86'/0'/73'` and register it as the session's `WalletService`. Balance and addresses become visible; all signing operations (Send, commit funding, reveal) remain disabled with a clear "Connect hardware wallet to sign" message. Signing lands later: the reveal key becomes ephemeral in R1.0, the commit funding tx is HW-signed in R1.1, and Send-on-HW in Phase 8.

**Rationale:** After Phase 3.7, HW users see the `Disabled` state in the wallet panel — a regression from the PRD intent. A watch-only wallet is trivially derivable from the xpub that the existing Trezor/Ledger IPC already surfaces, and gives HW users the same read-only visibility mnemonic users have, without requiring any Phase 8 signing infrastructure.

**In scope**

- Extend `auth_complete` (HW path): after successful orchestrator auth with a HW-derived Admin ID, also call the Trezor/Ledger IPC to retrieve the account xpub at `m/86'/0'/73'` and construct a watch-only `bdk_wallet::Wallet` (descriptor from xpub, no private key material).
- `WalletService` initialized from watch-only descriptor; `can_sign()` → `false`. All existing read IPC (balance, UTXOs, addresses, sync) works identically.
- Send and commit-funding IPC commands return a new `WalletError::ReadOnly` variant when `can_sign()` is false; UI surfaces "Hardware wallet required to sign".
- Logout clears the watch-only wallet from managed state (same as Phase 3.7).
- Tests: unit test that a watch-only `WalletService` returns `ReadOnly` on sign attempts; read-path IPC returns data normally.

**Out of scope**

- PSBT construction and HW signing (commit funding in Release 1 R1.1; reveal key ephemeral in R1.0; Send in Phase 8).
- Trezor/Ledger xpub export IPC if it does not already exist — if missing, scope it minimally here; do not build full PSBT flow.

**Done when**

- HW login shows correct balance and funded addresses in the wallet panel.
- Send button and broadcast commit are disabled with "Hardware wallet required to sign" (not hidden — visible but inoperable).
- Mnemonic login path unchanged; watch-only path has zero impact on Phase 3.7 behavior.
- `cargo test --workspace` and full frontend CI green.

**Primary code areas**

- `desktop-app/src-tauri/src/commands/authentication.rs` — HW `auth_complete` branch
- `desktop-app/src-tauri/src/application/wallet_service.rs` — `can_sign()` predicate; `WalletError::ReadOnly`
- `desktop-app/src-tauri/src/infrastructure/hw_wallet/` — xpub extraction (Trezor/Ledger)
- `desktop-app/src/domain/admin-wallet/` — UI: disable Send / show "requires HW" when `canSign: false`

**Risks / notes**

- **xpub IPC availability**: Trezor adapter in `hw_wallet/trezor.rs` may not yet expose account-level xpub export. Needs investigation at implementation time; if missing it is a small addition scoped to this phase.
- **Signing handoff**: Release 1 (R1.1) and Phase 8 replace the watch-only descriptor with a PSBT signer at the same managed state slot. No further changes to `auth_complete` or `WalletService` API are expected.

---

### Release 1

**Release 1 is complete.** Steps **R1.0–R1.7** are done (R1.5: PR [#211](https://github.com/wakeuplabs-io/alpen-multisig/pull/211); R1.6: PR [#212](https://github.com/wakeuplabs-io/alpen-multisig/pull/212); R1.7: PR [#214](https://github.com/wakeuplabs-io/alpen-multisig/pull/214)). PRD §4.3.1, §4.3.2, and **§4.3.4 rotation** are **PASS** in [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md). PRD §4.3.3 (tx list/RBF), §4.3.4 (QR/HW verify), §4.3.5 (Send), §4.1–4.2 (Admin ID), and **US-H4 broadcast fee** remain open (Phases 4–10). **Next:** **Release 2** (Electrum wallet sync).

**R1.0–R1.4 closure:** R1.4 merged via [PR #206](https://github.com/wakeuplabs-io/alpen-multisig/pull/206) (`9bf5c3f`, 2026-06-02). Evolution: [`2026-06-02-admin-wallet-canonical-connect-paths.md`](../evolution/2026-06-02-admin-wallet-canonical-connect-paths.md).

**R1.5 closure:** Branch `feature/admin-wallet-balance-ux`, PR [#211](https://github.com/wakeuplabs-io/alpen-multisig/pull/211). Evolution: [`2026-06-03-admin-wallet-balance-ux.md`](../evolution/2026-06-03-admin-wallet-balance-ux.md).

**R1.6 closure:** Branch `feature/admin-wallet-addresses-ux`, PR [#212](https://github.com/wakeuplabs-io/alpen-multisig/pull/212) (`0c0c01c` spec, `3d0a5e4` implementation). Evolution: [`2026-06-03-admin-wallet-addresses-ux.md`](../evolution/2026-06-03-admin-wallet-addresses-ux.md). Manual regtest: per-address confirmed/unconfirmed sub-lines verified.

**R1.7 closure:** Branch `feature/admin-wallet-r17-ui-polish`, PR [#214](https://github.com/wakeuplabs-io/alpen-multisig/pull/214). Two passes: (a) visual hierarchy + layout refinement, (b) affordances & polish (icon-only copy, wallet avatar, count badge, session chevron, drawer easing/shadow). Spec: [`admin-wallet-wallet-panel-ui-polish.md`](./admin-wallet-wallet-panel-ui-polish.md).

**Release 1 fully closed.** All R1.0–R1.7 slices shipped. Balance (§4.3.1), addresses (§4.3.2), receive rotation (§4.3.4 rotation), and panel UI polish complete. Next: **Release 2** (Electrum wallet sync) — also now complete, followed by Phase 4 (also complete). Next: **Phase 5** (Transactions + fee-bump) — also now complete ✅.

#### R1.0 — Ephemeral reveal key (decouple the envelope key from the seed) ✅

**Status:** Complete — merged to `develop` as PR #195 (2026-05-30).

**Goal:** Replace the commit/reveal internal key — currently derived from the session seed at `m/86'/0'/73'/2/0` (Phase 3.5/3.7b) — with a **per-broadcast ephemeral key** generated in the app. The envelope/carrier key is not custody-significant (governance authority lives in the SPS-65 `SignatureSet` inside the payload), so it does not need to come from the wallet seed. This makes reveal signing **login-agnostic** and shrinks R1.1 to "HW signs the commit funding". The reveal **change must be redirected to an Admin Wallet address** so no funds are stranded on the throwaway key.

**Done when:** On regtest, commit+reveal succeed using a fresh ephemeral key per broadcast (no `m/86'/0'/73'/2/0` derivation); the reveal change lands on an Admin Wallet address (not the ephemeral key); mnemonic-login broadcast behavior is otherwise unchanged; `cargo test --workspace` and frontend CI green.

**Why / notes:** Revisits the Phase 3.5 decision (which folded the operator key into the wallet seed). That rationale treated the envelope carrier key as custody-significant; it is not — and a HW cannot sign the SPS-50 reveal (taproot **script-path** over a custom envelope leaf) anyway, so an in-app key is unavoidable for the reveal. Known limitation until R1.0.1: the ephemeral key lives across the commit→reveal window; loss on crash is bounded to the commit dust + fee. See [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md) for the updated protocol description.

#### R1.0.1 — Build and sign commit + reveal before broadcasting ✅

**Spec:** [`admin-wallet-presign-commit-reveal.md`](./admin-wallet-presign-commit-reveal.md) — full technical design, decisions, and test plan.

**Status:** Done — merged PR #198 (2026-05-30). 8 TDD steps; `submit_package` + sequential fallback, in-memory `PendingReveals` + `proposals_resubmit_reveal`, single regtest mine, `commit_confirmed` PATCH dropped. Evolution: [`docs/evolution/2026-05-30-admin-wallet-presign-commit-reveal.md`](../evolution/2026-05-30-admin-wallet-presign-commit-reveal.md).

**Goal:** Reorder the broadcast flow so the commit and the reveal are both built and signed **before either is broadcast**, then broadcast commit→reveal (atomically via `submitpackage` when the node supports it, otherwise sequentially). Drop the ephemeral key immediately after both are signed. This removes the crash-loss window R1.0 introduces: once the reveal is signed the ephemeral key is no longer needed.

**Done when:** On regtest, an approved proposal broadcasts with the reveal already signed before the commit hits the network; `submitpackage` atomicity means a crash before the broadcast leaves nothing on-chain (clean retry), and a transient broadcast failure within the session is recoverable via the `proposals_resubmit_reveal` IPC command (re-sends the in-memory signed reveal, no ephemeral key needed); commit→reveal still confirm; `cargo test --workspace` and frontend CI green.

**Why / notes:** Today `broadcast_commit_then_reveal` broadcasts the commit first (Step 1) and only builds/signs the reveal afterward (Step 3, via `get_raw_transaction`). R1.0.1 splits commit funding into build-and-sign (returning the full signed `Transaction`) from broadcast, so the reveal is built locally without the round-trip. `submitpackage` is best-effort (Core 24+); sequential commit→reveal is the fallback. **Persistence scope (decided):** the signed reveal is **not** durably persisted — the window is closed by `submitpackage` atomicity, and a session-scoped in-memory store backs the resubmit IPC; a hard process crash on the sequential-fallback path (pre-24 node) is an accepted, documented limitation (durable orchestrator-stored persistence is a possible future hardening). RBF of the commit is impossible without re-deriving the key — Phase 5 bumps pending commits via CPFP on the reveal's change output instead.

#### R1.1 — Session-driven broadcast signing (adds HW path) ✅

**Status:** Complete — merged to `develop`. Unified `PsbtSigner` port (mnemonic + Ledger on-device); `ALLOW_DEV_MNEMONIC_SIGNING` fully removed as a signing/broadcast gate and replaced by the per-signer `allowed_on(network)` capability (it survives only as the dev-only mnemonic-login IPC exposure gate in `dev_secrets.rs`, P-040). Evolution: [`admin-wallet-session-driven-broadcast-signing-evolution.md`](../evolution/admin-wallet-session-driven-broadcast-signing-evolution.md).

**Spec:** [`admin-wallet-session-driven-broadcast-signing.md`](./admin-wallet-session-driven-broadcast-signing.md) — full technical design, decisions (D1–D7), and test plan.

**Goal:** Unify broadcast signing behind a single `PsbtSigner` **driven port** on `WalletService`. The commit PSBT is always built by BDK from the wallet descriptor (fully annotated with taproot/BIP32 derivation) and handed to the selected signer; only the signer differs — `MnemonicPsbtSigner` (software, a **simulated hardware wallet** for the "Palabras"/mnemonic login) or `HwPsbtSigner` (real Trezor/Ledger **on-device, PSBT key-path** for the HW login). The mnemonic path therefore exercises the *exact same unified flow* as the HW path, end-to-end on regtest with no device. The reveal stays signed by the ephemeral envelope key in both cases and is never routed to the signer. This unblocks HW logins (today watch-only → `ReadOnly`) from broadcasting. Downstream `CommitFunding` and `broadcast_commit_then_reveal` are unchanged.

`ALLOW_DEV_MNEMONIC_SIGNING` is **removed** and replaced by a typed, per-signer network capability (`signer.allowed_on(network)`): `MnemonicPsbtSigner` is allowed on **regtest/testnet only** (a software hot key must never sign mainnet, per PRD §3.2); `HwPsbtSigner` is allowed on **any** network. `WalletService.can_sign: bool` becomes an optional signer capability.

Sliced in two steps (both ship under R1.1): (a) `PsbtSigner` port + `MnemonicPsbtSigner` + flow unification + flag removal — the walking skeleton, verifiable on regtest with the mnemonic login and zero device; then (b) `HwPsbtSigner` real on-device taproot key-path PSBT signing. (a) de-risks (b).

**Done when:**
- A unified `PsbtSigner` port exists with `MnemonicPsbtSigner` and `HwPsbtSigner` implementors; `WalletService` builds the PSBT via BDK for both paths, signs through the port, finalizes and extracts the tx.
- On regtest, the mnemonic login broadcasts an approved proposal end-to-end with **no device** (slice a); the HW login broadcasts with real on-device taproot key-path signing (slice b); the reveal is signed by the ephemeral key in both.
- `ALLOW_DEV_MNEMONIC_SIGNING` is removed from code, CI, e2e, and `.env`; the mnemonic signer is rejected on mainnet with `SignerNotAllowedOnNetwork` and accepted on regtest/testnet; the HW signer is allowed on any network.
- Device-absent / user-refusal returns a typed error **before** any broadcast (nothing hits the network).
- Existing `CommitFunding` and `broadcast_commit_then_reveal` tests stay green; no out-of-session custody key is consulted.

#### R1.2 — Clean wallet UI ✅

**Status:** Complete — branch `feature/admin-wallet-clean-wallet-ui` (commit `138412d`, 2026-06-02). Spec: [`admin-wallet-clean-wallet-ui.md`](./admin-wallet-clean-wallet-ui.md). Evolution: [`docs/evolution/2026-06-02-admin-wallet-clean-wallet-ui.md`](../evolution/2026-06-02-admin-wallet-clean-wallet-ui.md).

**Goal:** Bring the wallet panel to production quality — remove dev-only affordances and placeholders, consistent loading/empty/error states.

**Done when:** The wallet panel shows balance, addresses, and receive cleanly with no dev-only controls, at visual parity with the Alta WalletPanel.

#### R1.3 — Receive rotation ✅

**Status:** Complete — branch `feature/admin-wallet-receive-rotation` (commit `788d4eb`, 2026-06-02). Evolution: [`docs/evolution/2026-06-02-admin-wallet-receive-rotation.md`](../evolution/2026-06-02-admin-wallet-receive-rotation.md).

**Spec:** [`admin-wallet-receive-rotation.md`](./admin-wallet-receive-rotation.md) — full technical design, decisions, and test plan.

**Goal:** Issue a fresh receive address and rotate after credit (PRD §4.3.4.3). QR and verify-on-device remain Phase 7 — see compliance matrix **PARTIAL** for §4.3.4 overall.

**Design:** A `WalletService::next_receive_address` method backed by BDK's gap-aware `next_unused_address(External)`, exposed via the `admin_wallet_next_receive_address` IPC command and a `useAdminWalletReceiveAddress` hook. The method is idempotent until the displayed address is observed in a transaction during sync, then rotates — replacing the prior front-end `find((a) => !a.isUsed)` window scan. Pure public derivation, so it works for mnemonic and HW/watch-only sessions alike.

**Done when:** After incoming funds confirm, the displayed receive address rotates to the next unused index on regtest.

#### R1.4 — Remove connect-time derivation picking ✅

**Status:** Complete — merged to `develop` via [PR #206](https://github.com/wakeuplabs-io/alpen-multisig/pull/206) (`9bf5c3f`, 2026-06-02). Spec: [`admin-wallet-canonical-connect-paths.md`](./admin-wallet-canonical-connect-paths.md). Evolution: [`docs/evolution/2026-06-02-admin-wallet-canonical-connect-paths.md`](../evolution/2026-06-02-admin-wallet-canonical-connect-paths.md).

**Goal:** Drop the connect-flow step where the user manually picks a derivation path/account; derive Admin ID and Admin Wallet automatically at their canonical paths.

**Done when:** Connecting a HW wallet derives Admin ID (`m/84'/0'/73'/0/0`; Ledger regtest/testnet follows its existing `m/84'/1'/73'/0/0` app convention) and Admin Wallet (`m/86'/0'/73'/n/n`; Ledger regtest/testnet uses `m/86'/1'/73'`) with no manual path-selection UI.

**Later (optional, not in Release 1):** Admin ID display/copy (Phase 7), Send-on-HW + verify-on-device (Phase 8), QR for receive, fee-bump (Phase 5), broadcast fee (Phase 4) — pull forward only if a Release 1 step needs them.

#### R1.5 — Balance UX (PRD §4.3.1 complete) ✅

**Status:** Complete — branch `feature/admin-wallet-balance-ux`, PR [#211](https://github.com/wakeuplabs-io/alpen-multisig/pull/211). Evolution: [`2026-06-03-admin-wallet-balance-ux.md`](../evolution/2026-06-03-admin-wallet-balance-ux.md).

**Spec:** [`admin-wallet-balance-ux.md`](./admin-wallet-balance-ux.md) — UX copy, view-model contracts, test plan, mempool sync amendment.

**Goal:** Close PRD §4.3.1 in the wallet slide-over: confirmed hero balance plus a separate signed unconfirmed line when pending activity exists.

**Delivered**

- **Frontend:** `WalletBalance` shows `confirmedSats` as hero and `+N sats unconfirmed` / `−N sats unconfirmed` when `unconfirmedSats !== 0`; wired on dashboard and broadcast panels via `formatUnconfirmedBalanceLine`.
- **Sync (scope amendment):** `WalletService::do_sync` applies `Emitter::mempool()` + `apply_unconfirmed_txs` after block sync so regtest `sendtoaddress` without mining updates balance and receive rotation (fixes block-only gap found in QA).
- **Tests:** `format-unconfirmed-balance-line` unit tests; architecture Rule 5 wiring guard.

**Done when:** Met on regtest — unconfirmed receive without mining shows tertiary line and rotates receive address after Refresh; line hidden when fully confirmed; CI green.

#### R1.6 — Addresses UX (PRD §4.3.2 complete) ✅

**Status:** Complete — branch `feature/admin-wallet-addresses-ux`, PR [#212](https://github.com/wakeuplabs-io/alpen-multisig/pull/212). Evolution: [`2026-06-03-admin-wallet-addresses-ux.md`](../evolution/2026-06-03-admin-wallet-addresses-ux.md).

**Spec:** [`admin-wallet-addresses-ux.md`](./admin-wallet-addresses-ux.md) — UX copy, wireframes, view-model contracts, and test plan.

**Goal:** Close the remaining PRD §4.3.2 gap. The signer MUST see each address that holds a balance with its current balance **net of unconfirmed transactions**, and unconfirmed effects per address must be visible separately where non-zero. Phase 2 `UtxoDto.confirmations` and `composeAddressesWithBalance` already aggregate sats by derivation index — R1.6 splits confirmed vs unconfirmed per address in the view-model and renders it in the addresses table.

**PRD gap (today):** `AddressWithBalanceView` exposes a single `balanceSats` (all UTXOs summed); rows do not distinguish pending credits/debits.

**In scope (frontend-only)**

- **View-model:** extend `composeAddressesWithBalance` (or adjacent mapper) to produce `confirmedSats` and `unconfirmedSats` per row (`confirmations === 0` → unconfirmed bucket).
- **`AddressRow` / `AddressesWithBalanceList`:** hero column shows confirmed balance; when `unconfirmedSats !== 0`, show a muted sub-line (`±N sats unconfirmed`) matching R1.5 copy conventions.
- **Accordion UX:** header copy `Addresses with balance · N` (drop redundant "All"); keep default collapsed; empty/loading/error states from R1.2 unchanged.
- **Optional polish:** per-row copy address via shared `CopyButton`; full address on `title` / expand — spec decides.
- **Header / capability:** `Admin Wallet` title + session subtitle if not done in R1.5; subtle watch-only badge when `canSign === false` (uses existing `useAdminWalletCapability`).
- **Tests:** unit tests for confirmed/unconfirmed split in `compose-addresses-with-balance`; row formatter tests if extracted.

**Out of scope**

- Wallet-level unconfirmed line → R1.5 (must ship first or in parallel only after R1.5 merge).
- Internal/change address listing policy changes (external-only with balance remains default).
- Send, QR, new IPC, pagination beyond existing address page.

**Done when**

- Fund two external indices on regtest; credit one with unconfirmed UTXOs — expanded list shows per-address confirmed balance and unconfirmed sub-line where applicable.
- PRD §4.3.2 **PASS** in [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md); R1.0–R1.6 shipped; **R1.7** wallet UI polish remains; §4.3.4 rotation PASS, QR/HW verify FAIL.
- Frontend CI green.

**Primary code areas:** `domain/admin-wallet/model/compose-addresses-with-balance.ts`, `components/address-row.tsx`, `components/addresses-with-balance-list.tsx`, `components/wallet-panel-header.tsx`, `hooks/use-admin-wallet-capability.ts`.

**Delivered**

- **View-model:** `groupUtxoBalancesByDerivation` + `composeAddressesWithBalance` with `confirmedSats` / `unconfirmedSats` per row.
- **UI:** `AddressRow` confirmed hero + unconfirmed sub-line; accordion `Addresses with balance · N`; per-row `CopyButton`.
- **Header:** `Admin Wallet` title, session/signer subtitle, **Watch-only** badge when `canSign === false`.
- **Tests:** compose/group model tests, address-row contract test, architecture Rule 6.

**R1.6 closure:** PR [#212](https://github.com/wakeuplabs-io/alpen-multisig/pull/212). **Next in Release 1:** R1.7 (wallet panel UI polish).

#### R1.7 — Wallet panel UI polish ✅

**Status:** Complete — PR [#214](https://github.com/wakeuplabs-io/alpen-multisig/pull/214). Spec: [`admin-wallet-wallet-panel-ui-polish.md`](./admin-wallet-wallet-panel-ui-polish.md).

**Goal:** Visual and interaction quality pass on the **Admin Wallet slide-over** only — closer to Alta `WalletPanel` handoff (balance, receive, addresses-with-balance, sync footer).

**Done when:** Met — Release 1 fully closed.

---

### Release 2

**Release 2 is complete.** Steps **R2.1–R2.3** are done (R2.1: PR [#261](https://github.com/wakeuplabs-io/alpen-multisig/pull/261); R2.2: PR [#262](https://github.com/wakeuplabs-io/alpen-multisig/pull/262); R2.3: PR [#263](https://github.com/wakeuplabs-io/alpen-multisig/pull/263)). Wallet sync now uses **Electrum** (`bdk_electrum`) instead of Core RPC block-scan. Broadcast, `submitpackage`, and fee estimation continue to use chain RPC.

**Release 2 fully closed.** All R2.1–R2.3 slices shipped. Electrum infra, wallet sync migration, and Node Config URL complete. Next: **Phase 4** (governance broadcast fee rate) — also now complete.

**Spec:** [`admin-wallet-electrum-sync.md`](./admin-wallet-electrum-sync.md) — full slice breakdown (R2.1–R2.3).

**Goal:** Wallet sync (balance, UTXOs, addresses, receive rotation) uses an Electrum-protocol indexer. Broadcast, `submitpackage`, and fee estimation continue to use `BITCOIN_RPC_URL`.

**Slices (in order)**

| Slice | Goal |
|-------|------|
| **R2.1** | electrs in Docker + dev/staging/CI/scripts; synced to local regtest `bitcoind`; smoke verification — **no app code** |
| **R2.2** | Migrate `WalletService` sync to `bdk_electrum` in one step; fixed Electrum URL; broadcast/fees unchanged |
| **R2.3** | Electrum URL in `NodeConfig` (Local / Trusted / Custom), same pattern as BTC RPC and Strata |

**Out of scope (whole release)**

- Any indexer backend other than Electrum protocol.
- Send (Phase 6), tx list / RBF (Phase 5), Admin ID UI (Phase 7), shared Send UX (Phase 9).
- Changing commit/reveal protocol or `PsbtSigner` flows.

**Done when:** R2.1–R2.3 complete — wallet panel read path syncs in production-viable time; Release 1 wallet UX parity; governance broadcast unchanged; CI green.

**Prerequisite for:** Phases 5–10 on testnet/mainnet (R2 ✅). **Phase 4** (US-H4 broadcast fee rate) is also complete ✅. **Phase 5** (Transactions + fee-bump) is complete ✅. **Next:** Phase 6 (Send BTC happy path).

**Supersedes:** [`admin-wallet-sync-progress.md`](./admin-wallet-sync-progress.md) as the primary mitigation for slow sync — block-scan progress UI remains **deferred** unless still needed post-R2.2.

#### R2.1 — Electrum indexer infra ✅

**Status:** Complete — PR [#261](https://github.com/wakeuplabs-io/alpen-multisig/pull/261).

**Goal:** Regtest Electrum indexer (electrs) available in Docker, local dev, staging, and CI, backed by the existing regtest `bitcoind`.

**Done when:** Met — electrs in `staging/docker-compose.yml` and `docker-compose.local.yml`; smoke verification after funded address.

#### R2.2 — Admin Wallet sync migration ✅

**Status:** Complete — PR [#262](https://github.com/wakeuplabs-io/alpen-multisig/pull/262).

**Goal:** Replace Core RPC block-scan in `WalletService` with `bdk_electrum`; prove read path on regtest with a fixed Electrum URL.

**Done when:** Met — `WalletService::do_sync` uses `bdk_electrum`; broadcast/fee paths unchanged; Release 1 wallet UX parity.

#### R2.3 — Electrum URL in Node Config ✅

**Status:** Complete — PR [#263](https://github.com/wakeuplabs-io/alpen-multisig/pull/263).

**Goal:** Configurable Electrum URL in the app (Rust `NodeConfig`, IPC, connect-screen modal) — Local / Trusted / Custom, mirroring BTC RPC and Strata.

**Done when:** Met — `NodeConfig` exposes `custom_electrum_url`; `electrum_url()` resolves Local/Trusted/Custom; wallet sync, fee estimation, and broadcast all use the configured URL.

---

### Remaining phases (5–10)

Phases 5–10 continue after **Release 2** and **Phase 4** (both complete). **Phase 5** (Transactions + fee-bump) is complete ✅; **Phase 6** (Send BTC happy path) is next. **Phase 7 (receive QR) and Phase 8 (HW Send)** overlap with work already started in Release 1 — entries below list only what remains.

#### Phase 4 — Governance broadcast fee rate ✅

**Status:** Complete — M1: PR [#267](https://github.com/wakeuplabs-io/alpen-multisig/pull/267); M2+M3: PR [#273](https://github.com/wakeuplabs-io/alpen-multisig/pull/273).

**Specs:** [`governance-broadcast-fee-selection.md`](./governance-broadcast-fee-selection.md) (functional); [`governance-broadcast-fee-selection-implementation.md`](./governance-broadcast-fee-selection-implementation.md) (technical).

**Goal:** [**US-H4**](../3-stories/story-map.md) — on governance **broadcast** (commit funding), let the signer set fee rate in **sat/vB** (0.1 increments, max 10 000); default **Medium** preset from the connected chain RPC. Per [`02-prd-update-impact.md`](../1-proposal/02-prd-update-impact.md): proposal/PRD expect **fee-rate controls on broadcast** before the full wallet Send surface; new PRD delegates pending-update Send fee UX to the wallet-send pattern (§4.3.5.3), but commit/reveal broadcast still needs an explicit control now.

**Delivered:**
- **M1:** Domain `FeeRate` type (sat/kvB), `FeeEstimationService` (node + static fallback), `FeeRateSelector` UI component, `useFeePresets` hook, rate plumbed through all broadcast commands (local, manual, cancel), RBF regression tests.
- **M2:** Electrum fee estimation fallback + last-known-good in-memory cache.
- **M3:** `TxBroadcaster` port: Electrum-first broadcast, node RPC fallback, manual copy-hex escape hatch (`AllBroadcastersFailed` with structured error DTO).

**Done when:** Met — regtest approved-proposal broadcast succeeds with default or user-selected sat/vB; US-H4 acceptance signals met; Electrum broadcast path with node fallback operational.

---

#### Phase 5 — Transactions + fee-bump (RBF / CPFP) ✅

**Status:** Complete — PR [#276](https://github.com/wakeuplabs-io/alpen-multisig/pull/276).

**Spec:** [`admin-wallet-transactions-fee-bump.md`](./admin-wallet-transactions-fee-bump.md) — full technical design, decisions, and test plan.

**Goal:** Unconfirmed tx list and fee bump per PRD §4.3.3.

**In scope:** RBF bump for plain sends via BDK `build_fee_bump`; **CPFP** bump for pending governance commits (child spending the reveal's wallet-owned change, sized to lift the package rate); both signed via session `PsbtSigner` (R1.1) + Electrum-first broadcast with node fallback; error surfaces for non-RBF txs and unavailable CPFP anchors.

**Out of scope:** CPFP for non-governance txs (RBF covers them), payout txs.

**Delivered:**
- `application/wallet_transactions.rs`: `list_unconfirmed_sent_txs` (unconfirmed txs with wallet-owned inputs, fee/rate/RBF flag, package stats for governance commits) and `bump_fee` (RBF/CPFP dispatch → sign via `PsbtSigner` port → broadcast → result), typed `BumpFeeError`.
- `TxBroadcaster::broadcast_one` + `broadcast_single_with_fallback` (Electrum → node, already-known idempotency).
- **Governance acceleration (CPFP):** commits with a pending pre-signed reveal (`PendingReveals`) cannot be RBF-replaced — that would invalidate the reveal (R1.0.1, ephemeral key dropped after signing) — so the bump builds a child on the reveal's change output; the requested rate applies to the whole commit+reveal+child package.
- IPC: `admin_wallet_list_unconfirmed_txs`, `admin_wallet_bump_fee` (both handler sets; capability per-signer).
- UI: `Pending transactions` accordion in the wallet slide-over; per-row Bump with inline 0.1 sat/vB stepper (suggested default from Fast preset), success/new-txid and tagged error surfaces; governance rows show package fee/rate and bump via CPFP; Bump disabled for watch-only / non-RBF rows.

**Done when:** User can bump an unconfirmed Admin Wallet send (RBF) and a pending governance commit (CPFP) on regtest — met (Rust unit + IPC contract suites; manual path: broadcast a proposal without mining, then Bump fee in the wallet panel).

**Primary code areas:** tx list IPC, bump command, UI actions.

---

#### Phase 6 — Send BTC happy path (regtest, dev mnemonic)

**Spec:** [`admin-wallet-send-btc.md`](./admin-wallet-send-btc.md) — roadmap with per-increment PRD §4.3.5 traceability; detailed slice specs + TDD authored when each increment is picked up.

**Goal:** PRD §4.3.5 Send with validations (address network, amount, fee control aligned with Phase 4, change to `…/1/*`).

**In scope:** Build/sign/broadcast via BDK; dev mnemonic on regtest; Confirm gate. Composes existing pieces (Phase 4 fee control, R1.1 `PsbtSigner`, Phase 4 M3 `TxBroadcaster`, R1.3 change-index discipline) — no new protocol or custody primitive.

**Out of scope:** Hardware confirm (Phase 8), mainnet (Phase 10), full governance Send chrome (Phase 9).

**Slices (in order — each shippable on regtest, dev mnemonic):**

| Slice | Goal | PRD §4.3.5 |
|---|---|---|
| **P6.1** | Send pipeline walking skeleton: build → sign → broadcast, change to first unused internal index, minimal Confirm → txid | §4.3.5.4, §4.3.5.5 / §4.3.5.5.1 (thin fee reuse of §4.3.5.3) |
| **P6.2** | Destination validation — standard types accepted; network / non-address rejected with exact PRD copy | §4.3.5.1 |
| **P6.3** | Amount + fee contract + **Max** — `amount ≤ balance − fee`, "Insufficient funds"; default next-block, 0.1 step, max 10 000 | §4.3.5.2, §4.3.5.3 |
| **P6.4** | Confirm gate + result / reject-retry surfaces | §4.3.5.5, §4.3.5.5.1 |

**Done when:** Regtest send succeeds with change to first unused internal index; every §4.3.5 MUST met on the dev-mnemonic path; watch-only/HW sessions see Send disabled ("Hardware wallet required to sign"); §4.3.5 **PASS (regtest / dev mnemonic)** in the compliance matrix.

**Primary code areas:** `WalletService` send path, Send screen, validation helpers, reused `fee-selection/` selector.

---

#### Phase 7 — Admin ID UI (receive rotation shipped in Release 1)

> Receive-address rotation is delivered in **Release 1 (R1.3)**. This phase covers only the remainder.

**Goal:** PRD §4.1–4.2 Admin ID display/copy + QR.

**In scope:** Admin ID `m/84'/0'/73'/0/0` shown and copyable in UI; QR for receive/Admin ID.

**Out of scope:** Receive rotation (Release 1, R1.3); HW verify-on-device (Phase 8).

**Done when:** Admin ID visible and copyable per PRD.

**Primary code areas:** settings/header Admin ID, wallet Receive tab (display only).

---

#### Phase 8 — Hardware wallet direct adapters (Send-on-HW; broadcast signing shipped in Release 1)

> HW signing for the governance **broadcast** is delivered in Release 1 (commit-funding HW signing in R1.1; reveal by an ephemeral key in R1.0). This phase covers only the remainder: HW signing of the Admin Wallet **Send** path and verify-on-device.

**Goal:** Trezor/Ledger PSBT sign for the Admin Wallet Send path (Phase 6) per PRD §3.2; reuse existing device adapters; verify-address-on-device.

**In scope:** PSBT preview + signing on device for Send; receive-address verify-on-device; reuse of the adapters established in Release 1.

**Out of scope:** HWI CLI, POC-miniwallet integration paths; governance broadcast signing (Release 1, R1.0/R1.1).

**Done when:** Regtest/testnet Admin Wallet send is HW-signed without a mnemonic; dev-mnemonic guard becomes unreachable on release builds (full removal in Phase 10).

**Primary code areas:** `infrastructure/hw_wallet/`, PSBT pipeline in `admin_wallet` (send path).

---

#### Phase 9 — Shared Send + governance broadcast UX

**Goal:** Alta S9/S11-style **shared Send** component; pending-quorum “Send” flow per PRD §5.3.2.3 (fee entry reuses Phase 4/6 controls).

**In scope:** Unified send form/validation chrome across wallet Send, governance broadcast confirm, and (later) payout Send.

**Out of scope:** Payout swimlane; first broadcast fee slice (Phase 4).

**Done when:** Governance broadcast and wallet Send share components and validation patterns.

**Primary code areas:** broadcast screen refactor, shared `send/` components.

---

#### Phase 10 — Hardening + remote testnet/mainnet

**Goal:** No local node assumption for end users; trusted/custom presets for **both** Electrum (R2) and chain RPC; production capability flags.

**In scope:** Network presets, TLS/auth for remote chain RPC and Electrum, remove dev mnemonics from release builds, deprecate `BITCOIN_WALLET_NAME` for product flows, runbooks for remote testnet/mainnet without bundled `bitcoind`.

**Out of scope:** Wallet sync backend changes (delivered in R2).

**Done when:** Testnet/mainnet operate against remote Electrum + chain RPC only; documentation and runbooks updated.

**Primary code areas:** `broadcast_env.rs`, `node_config_store.rs`, config UI, release CI matrix without bundled bitcoind for app users.

## 5. Baseline and current state

**Pre-Foundation** (before Phase 1) vs **now (after Release 1)**. Use [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md) for PRD PASS/FAIL.

| Area | Pre-Foundation | Now (after Release 2 + Phase 4) |
|---|---|---|
| Governance broadcast | Commit via `sendtoaddress`; reveal via `OPERATOR_SECRET_KEY_HEX` + `send_raw_transaction` | Commit funded/signed via Admin Wallet + `PsbtSigner` (R1.1); reveal via **ephemeral** key pre-signed before broadcast (R1.0, R1.0.1); **fee rate selected by signer** (Phase 4) |
| Chain access | `HttpBitcoinRpcClient` in `infrastructure/bitcoin_rpc.rs` | Dual endpoint: **Electrum** for wallet sync (R2) + Core RPC for broadcast/fees |
| Wallet sync | — | **Electrum** (`bdk_electrum`, R2.2); configurable URL via Node Config (R2.3); mempool visibility preserved |
| Fee estimation | Hardcoded 1 sat/vB | `FeeEstimationService`: node → Electrum → cache → static fallback (Phase 4); `FeeRateSelector` UI with Slow/Medium/Fast presets + Custom |
| Broadcast path | Node RPC only | **Electrum first**, node RPC fallback, manual copy-hex escape (Phase 4 M3) |
| Operator / reveal | `OPERATOR_SECRET_KEY_HEX`, `ALLOW_DEV_OPERATOR_KEY` | Retired Phase 3.5; seed path `m/86'/0'/73'/2/0` retired **R1.0** (ephemeral reveal) |
| Admin Wallet session | Fixed env mnemonic (`ADMIN_WALLET_REGTEST_MNEMONIC`) | Session-bound at login (3.7); env mnemonic removed (3.7c); HW watch-only (3.8) + HW broadcast sign (R1.1) |
| Admin ID HW | BIP-84 paths in adapters | Canonical connect (R1.4); **PRD §4.1–4.2 UI still FAIL** (Phase 7) |
| Wallet panel UI | None | Slide-over: balance (R1.5), receive+rotation (R1.3), addresses (R1.6); no Send/tx/QR tabs |
| Broadcast UI | `/proposals/:actionId/broadcast`, orchestrator claim + PATCH | + `submitpackage` / resubmit reveal (R1.0.1); + fee rate selector (Phase 4); + manual broadcast panel (Phase 4 M3) |
| BDK | Not in workspace | `bdk_wallet`, `bdk_electrum` (R2.2) |
| Broadcast fee (US-H4) | Not exposed in UI | **PASS** — Phase 4 (M1+M2+M3 complete) |
| Product chain assumption | Local regtest `bitcoind`; `BITCOIN_WALLET_NAME` for legacy commit | Local `bitcoind` for dev/CI; wallet sync via **Electrum** (R2); Node Config default local (PRD §2.2) until Phase 10 |

Spec: [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md).

## 6. Configuration

| Variable / config | Role | Direction |
|---|---|---|
| Electrum URL (`NodeConfig` / env) | Wallet sync / indexation (`bdk_electrum`), fee estimation, broadcast | **Done (R2.3)** — Local / Trusted / Custom; used by wallet sync, fee estimation, and Electrum-first broadcast |
| `BITCOIN_RPC_URL` | Chain RPC base URL — broadcast fallback, fees, `submitpackage` | Keep; used as broadcast fallback and fee estimation source (Electrum is primary for broadcast per Phase 4 M3) |
| `BITCOIN_RPC_USER` / `BITCOIN_RPC_PASS` | RPC auth | Keep |
| `BITCOIN_NETWORK` | `regtest` / `testnet` / `mainnet` | Keep |
| `BITCOIN_WALLET_NAME` | Legacy bitcoind wallet for `sendtoaddress` | **Removed in Phase 3.6** (broadcast path); verify non-broadcast usages before full removal |
| `COMMIT_FUNDING` | `bitcoind` (default) \| `admin_wallet` | **Removed in Phase 3.6** — Admin Wallet is the sole commit funder from Phase 3.6 onward |
| `ADMIN_WALLET_REGTEST_MNEMONIC` | **Removed in Phase 3.7c.** Admin Wallet mnemonic and commit/reveal key come from the login session (`wallet_session_init`) only. `.env` keeps RPC/asm vars. |
| `ALLOW_DEV_MNEMONIC_SIGNING` | **Retired as a signing/broadcast gate in R1.1** — broadcast capability is now decided per-signer by `allowed_on(network)` (`MnemonicPsbtSigner`: regtest/testnet only; `HwPsbtSigner`: any). The env name survives **only** as the dev-only exposure gate for the mnemonic/raw-key signing IPC in release builds (`dev_secrets.rs`, P-040; debug builds enable it automatically). No longer required in `.env` for broadcast. See [`admin-wallet-session-driven-broadcast-signing.md`](./admin-wallet-session-driven-broadcast-signing.md). | Partially retired |
| `OPERATOR_SECRET_KEY_HEX` | **Removed in Phase 3.5.** Was dev hot key for envelope; briefly superseded by `m/86'/0'/73'/2/0` (3.5–3.7b), then **R1.0 ephemeral reveal** | Retired |
| `ALLOW_DEV_OPERATOR_KEY` | **Removed in Phase 3.5.** Was a guard against the well-known POC test operator key; no longer applicable once operator key derives from the Admin Wallet | Retired |

Local `bitcoind` remains in `scripts/bitcoind-asm-runner.sh` and CI as the **chain RPC** source for broadcast and fees until Phase 10; a local **Electrum indexer** (e.g. electrs) is added for wallet sync in R2 dev/CI. End users target remote Electrum + remote chain RPC. The node's **wallet** (`BITCOIN_WALLET_NAME`) is no longer used for commit funding from Phase 3.6 onward.

## 7. Risks and backends

**Core RPC wallet sync (resolved by R2 ✅):** Block-by-block `Emitter` sync was replaced by Electrum in R2.2. [`admin-wallet-sync-progress.md`](./admin-wallet-sync-progress.md) (block-scan progress UI) remains **deferred** — not needed post-R2.

**Electrum (R2 ✅, Phase 4 ✅):** Wallet sync, fee estimation, and broadcast all use Electrum as primary. Public or shared Electrum servers may rate-limit or lag; trusted presets and failure surfaces must be validated on testnet before mainnet (Phase 10 hardening). Regtest requires a local Electrum indexer in dev/CI (R2.1 ✅). Phase 4 M3 adds node-RPC fallback and manual copy-hex escape for broadcast.

**Dual endpoints:** Wallet sync (Electrum) and broadcast/fees (Electrum primary, chain RPC fallback) are configured via the same `NodeConfig` Electrum URL plus `BITCOIN_RPC_URL`. Misconfiguration (wrong network, mismatched presets) must surface high-signal errors.

**Chain RPC (broadcast fallback):** Public Core RPC endpoints may rate-limit or lack `submitpackage`; Phase 10 hardening covers broadcast path resilience. `submitpackage` remains Core-specific. Phase 4 M3 `TxBroadcaster` handles the fallback chain automatically.

**Payout path collision (future Payout program):** PRD §3.2.1.1 defines the **Payout Admin ID** at `m/86'/0'/73'/0/0` — the **same path** this program uses for the Admin Wallet external address index 0. When Payout is later in scope, decide explicitly between (a) shifting the Admin Wallet external start index for Payout, or (b) treating the Payout Admin ID as a dual-use key (auth + funding). Out of scope here; documented so the assumption is not lost.

## 8. Explicitly not in this program

- Payout Administrator flows (PRD §6, US-I*, Slice 4 payout stories in the story map).
- HWI and POC-miniwallet integration paths.
- Any indexer backend other than **Electrum** (R2).
- Requiring signers to run a local full node in production.
- Changing commit/reveal protocol semantics in [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md) beyond commit **funding source**.
