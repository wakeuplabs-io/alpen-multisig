# Spec: Admin Wallet–only commit funding (Phase 3.6)

> Traceability: Phase 3.6 of [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md)
> (section 4 — Phase 3.6; section 6 — Configuration). Supersedes the dual-path funding model
> introduced in Phase 1 and documented in [`admin-wallet-regtest-commit-funding.md`](./admin-wallet-regtest-commit-funding.md).

## Objective

Make the **Admin Wallet (BDK)** the **sole** commit funder for governance broadcast. Remove the
dual-path bifurcation from Phase 1: delete the legacy node-wallet `sendtoaddress` funding path
(`BitcoindSendToAddress`) and the `COMMIT_FUNDING` environment-variable toggle entirely. From this
phase on there is no fallback and no env switch — the commit transaction is always funded from the
Admin Wallet descriptors at `m/86'/0'/73'/0/*` (external) with change to `…/1/*`.

**Why:** carrying two funding paths means all subsequent work (Phase 4 Send, Phase 7 HW signing)
would be built and tested against the wrong (legacy) path. Collapsing to one path now makes the
Admin Wallet the single source of truth for commit funding before Phase 3.7 (session-bound wallet).

## Scope

### In scope

- Remove `BitcoindSendToAddress` (struct + `CommitFunding` impl) and the `select_commit_funding`
  factory + `COMMIT_FUNDING` dispatch from `application/commit_funding.rs`.
- Rewire `proposals_broadcast` to construct `BdkAdminWalletMnemonic` directly.
- Rewrite the `WalletService::check_enabled()` guard so it **no longer reads `COMMIT_FUNDING`**.
- Remove `COMMIT_FUNDING` and the desktop-side `BITCOIN_WALLET_NAME` from desktop broadcast code,
  `desktop-app/.env.example`, and all desktop tests / doc comments / error messages.
- Update the manual **WebDriver E2E** broadcast flow so it funds the Admin Wallet external address
  before broadcasting (the only funding path now requires spendable Admin Wallet UTXOs).
- Update `admin-wallet-regtest-commit-funding.md` to describe the single funding path.

### NOT in scope

- Removing `ADMIN_WALLET_REGTEST_MNEMONIC` (Phase 9 / superseded by Phase 3.7 session binding).
- Session-binding the wallet to login (Phase 3.7).
- Hardware-wallet signing for commit (Phase 7).
- **`BITCOIN_WALLET_NAME` for `orchestrator-be`** — it is still used there for wallet-scoped RPC
  (`getnewaddress` / `generatetoaddress`). It MUST be retained in `orchestrator-be/.env.example`,
  `orchestrator-be/src/config.rs`, and `staging/docker-compose.yml`.
- Changing SPS-50/51 envelope shape or commit/reveal protocol semantics — only the funding source.

## Technical Design

### Funding seam (kept)

`CommitFunding` (async trait, `fund_commit(commit_address, amount_sats, fee_rate) -> Result<String, _>`)
is **retained** as the abstraction seam consumed by `broadcast_commit_then_reveal(&dyn CommitFunding)`.
`BdkAdminWalletMnemonic` becomes its **only** implementor. Phase 7 HW signing plugs in at the
`WalletService` level, not here, so the trait stays. The trait's signature does **not** change.

### Guard relocation — the linchpin

Today the regtest/dev guard lived in **two** places:

1. `select_commit_funding` returned `CommitFundingError::NotRegtest` for non-regtest networks.
2. `WalletService::check_enabled()` required `COMMIT_FUNDING == "admin_wallet"` **and**
   `BITCOIN_NETWORK == "regtest"` **and** `ALLOW_DEV_MNEMONIC_SIGNING == "1"`.

After this phase the factory is gone, so the guard lives **only** in `check_enabled()`, rewritten to:

```rust
// COMMIT_FUNDING condition removed.
if bitcoin_network != "regtest" || allow_dev != "1" {
    return Err(AdminWalletError::Disabled);
}
```

`check_enabled()` is already invoked inside `WalletService::sync()` and `WalletService::fund_commit()`,
so every funding/read path remains guarded; no per-construction network check is needed in the command.

### Code touchpoints (production)

| File | Change | Single responsibility after change |
|---|---|---|
| `application/commit_funding.rs` | Remove `BitcoindSendToAddress` + impl, `select_commit_funding`, `COMMIT_FUNDING` dispatch. Prune now-unused `CommitFundingError` variants (`NotRegtest`, `MissingEnv`, `BitcoinRpc`); keep `AdminWallet`. Keep `CommitFunding` trait + `BdkAdminWalletMnemonic`. | Defines the commit-funding seam and its single BDK implementor. |
| `application/wallet_service.rs` | `check_enabled()`: drop the `COMMIT_FUNDING` condition; keep regtest + `ALLOW_DEV_MNEMONIC_SIGNING`. Update the two `check_enabled_*` unit tests. | Owns the BDK wallet lifecycle, sync, balance, addresses, commit funding, and the dev/regtest guard. |
| `commands/proposals.rs` | In `proposals_broadcast`, replace `select_commit_funding(btc_rpc, network, Some(ws))` with `BdkAdminWalletMnemonic::new(env.network, Arc::clone(&wallet_service))`. Remove the `select_commit_funding` import. | Tauri IPC for proposal lifecycle + broadcast wiring. |
| `commands/admin_wallet.rs` | Update the `admin_wallet_info` doc comment (drop the `COMMIT_FUNDING=admin_wallet` clause) and rewrite the tests that set/clear `COMMIT_FUNDING` (incl. the `bitcoind`-mode rejection test) to the new two-condition guard. | Tauri IPC for Admin Wallet read info. |
| `infrastructure/broadcast_env.rs` | Remove the `btc_wallet_name` field, the `BITCOIN_WALLET_NAME` parse, and the related env tests. | Loads + validates desktop broadcast env (RPC, network, keypair, magic bytes, timeouts). |
| `infrastructure/bitcoin_rpc.rs` (desktop) | Remove `send_to_address` from the desktop `BitcoinRpcClient` trait + impl, the `wallet_name` ctor param, and the `/wallet/{name}` URL handling — **only after grep confirms no other desktop consumer**. Keep node-level methods (`send_raw_transaction`, confirmations, fee estimate, `get_raw_transaction`, `mine_blocks`). | Desktop node-level Bitcoin JSON-RPC client. |
| `desktop-app/src/domain/admin-wallet/model/format-admin-wallet-error.ts` | Update the `Disabled` message: drop `COMMIT_FUNDING=admin_wallet`; keep `BITCOIN_NETWORK=regtest` + `ALLOW_DEV_MNEMONIC_SIGNING=1`. | Maps `AdminWalletError` codes to user-facing copy. |

### Config / docs touchpoints

| File | Change |
|---|---|
| `desktop-app/.env.example` | Remove the `COMMIT_FUNDING` comment block + `# COMMIT_FUNDING=admin_wallet` and `BITCOIN_WALLET_NAME`. Keep `ADMIN_WALLET_REGTEST_MNEMONIC` + `ALLOW_DEV_MNEMONIC_SIGNING=1`. |
| `docs/specs/admin-wallet-regtest-commit-funding.md` | Update to reflect the single funding path (no `COMMIT_FUNDING` toggle, no `sendtoaddress` fallback). |
| `orchestrator-be/.env.example`, `orchestrator-be/src/config.rs`, `staging/docker-compose.yml` | **Untouched** — `BITCOIN_WALLET_NAME` stays for the orchestrator. |

### WebDriver E2E (`desktop-app/e2e-webdriver`)

With Admin Wallet funding as the only path, the Admin Wallet external address
(`m/86'/0'/73'/0/0`) MUST hold spendable regtest UTXOs before **Confirm & Broadcast** or commit
funding fails with an insufficient-funds error.

- **New test helper** `test/helpers/fund-admin-wallet.mjs` (analogous to `mine-regtest-blocks.mjs`):
  funds the Admin Wallet external address on regtest, then mines enough blocks to make the UTXO
  spendable. Mirrors the existing integration-test approach (`get_external_address` +
  node-wallet send + `generatetoaddress`) in `desktop-app/src-tauri/tests/admin_wallet_integration.rs`.
  - **Address source** (implementation picks the robust option): (a) read the Admin Wallet receive
    address from the wallet panel UI via a `data-testid`, or (b) derive `m/86'/0'/73'/0/0` in JS
    from `DEMO_MNEMONIC`. Prefer reading from the UI/IPC to avoid duplicating derivation logic.
- **Wire** the funding step into `test/specs/proposal-broadcast-quorum.e2e.js` before clicking
  Confirm & Broadcast (and/or document it as a required manual step in the README playbook).
- **README** (`desktop-app/e2e-webdriver/README.md`): drop any `COMMIT_FUNDING` / `BITCOIN_WALLET_NAME`
  from setup recipes; add the "fund the Admin Wallet external address" prerequisite; keep
  `ADMIN_WALLET_REGTEST_MNEMONIC` + `ALLOW_DEV_MNEMONIC_SIGNING`. Fix broadcast-spec wording that
  still says "operator key" where it now means the Admin-Wallet-derived signer.

### Production code vs. test helpers

- **Production:** the `CommitFunding` trait, `BdkAdminWalletMnemonic`, the rewritten
  `check_enabled()`, and the `proposals_broadcast` wiring.
- **Test helpers:** `fund-admin-wallet.mjs` (E2E setup only) and the Rust `#[cfg(test)]` fixtures.
  None are registered as Tauri commands or exposed in production APIs.

## Test Cases

Target production functions only.

**Happy path**
- With `BITCOIN_NETWORK=regtest`, `ALLOW_DEV_MNEMONIC_SIGNING=1`, and a funded Admin Wallet, commit
  and reveal succeed; orchestrator `PATCH` behavior is unchanged (matches Phase 1/3.5 behavior).
- `check_enabled()` returns `Ok` when regtest + `ALLOW_DEV_MNEMONIC_SIGNING=1` are set, **without**
  `COMMIT_FUNDING`.

**Edge / guard**
- `check_enabled()` returns `Disabled` when `BITCOIN_NETWORK != regtest`.
- `check_enabled()` returns `Disabled` when `ALLOW_DEV_MNEMONIC_SIGNING != 1`.
- Setting `COMMIT_FUNDING` to any value has **no effect** on `check_enabled()` (it is no longer read).
- `BdkAdminWalletMnemonic::new` stores the injected `WalletService` (no ephemeral wallet created).

**Expected errors**
- Broadcast attempted with an unfunded / insufficient Admin Wallet surfaces a clear funding error
  (`AdminWalletError::WalletCreation` / typed), not a panic.

**Regression**
- Workspace grep is clean of `COMMIT_FUNDING`, `BitcoindSendToAddress`, and `select_commit_funding`
  (code, env, docs, scripts).
- Desktop broadcast no longer reads `BITCOIN_WALLET_NAME`; orchestrator still does.
- Phase 1 + Phase 2 + Phase 3.5 regression suites stay green.

## Module structure

No new production modules. The change is **subtractive** (remove a variant + factory) plus a guard
rewrite, so cohesion improves: `commit_funding.rs` collapses to one trait + one implementor, and the
dev/regtest guard becomes single-sourced in `WalletService::check_enabled()`. Dependency direction is
preserved: `broadcast_commit_then_reveal` depends on the `CommitFunding` abstraction, not the concrete
BDK type. One new **test-only** file (`fund-admin-wallet.mjs`) whose single responsibility is "fund the
Admin Wallet external address on regtest for E2E setup."

## Verification

```bash
# Rust (repo root) — exact CI flags
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Frontend (desktop-app/)
npm run format:check
npm run lint
npm run build
```

Plus a manual regtest run of the WebDriver playbook
(`proposal-add-signer` → `proposal-broadcast-quorum`) with the new Admin Wallet funding step,
confirming the broadcast reaches `e2e-broadcast-done-banner`.

## Done when

- `BitcoindSendToAddress`, `select_commit_funding`, and `COMMIT_FUNDING` no longer exist anywhere
  (code, env files, docs, CI, scripts) — workspace grep clean.
- `BITCOIN_WALLET_NAME` removed from the desktop broadcast surface; retained for `orchestrator-be`.
- On regtest with `ALLOW_DEV_MNEMONIC_SIGNING=1` and a funded Admin Wallet, commit + reveal succeed;
  orchestrator `PATCH` unchanged.
- The WebDriver `proposal-broadcast-quorum` flow passes with Admin Wallet funding, and its README
  reflects the single funding path.
- All Rust and frontend CI checks above are green.
