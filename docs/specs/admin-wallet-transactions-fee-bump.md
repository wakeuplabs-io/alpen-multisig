# Spec: Admin Wallet — Transactions + fee-bump (RBF) — Phase 5

**PRD:** [`03-prd-update.md`](../0-prd/03-prd-update.md) §4.3.3 — *"Transactions: The user MUST be able to see each unconfirmed transaction sent from the Admin Wallet and have the ability to bump the fee."*
**Plan:** [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) Phase 5 (Transactions + fee-bump, RBF-first).
**Compliance:** [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md) §4.3.3 — **PASS** (this phase).
**Status:** Implemented — branch `feature/admin-wallet-transactions-fee-bump`.

## Objective

Let the signer see every unconfirmed transaction **sent from** the Admin Wallet in the wallet slide-over, and replace (RBF) any such transaction with a higher-fee version, on regtest with the mnemonic login. This closes PRD §4.3.3.

Why now: Phase 4 delivered the fee-rate domain (`FeeRate`, `FeeEstimationService`, `FeeRateSelector` pattern) and R2 delivered Electrum sync with mempool visibility — both prerequisites for a correct unconfirmed-tx view and replacement broadcast.

## Scope

**In scope**

- List unconfirmed transactions where the Admin Wallet contributed inputs (sent), with txid, net amount, current fee / fee rate, vsize, RBF-signaling flag, and first-seen time.
- Fee-bump (RBF) of an unconfirmed, RBF-signaling wallet transaction: build via BDK `build_fee_bump`, sign via the session `PsbtSigner` (R1.1 port — mnemonic software signer; Ledger path reuses the same device flow as commit signing), broadcast Electrum-first with node-RPC fallback.
- Single-transaction broadcast capability on the existing `TxBroadcaster` port (today it only broadcasts commit+reveal pairs).
- Guard: a pending **governance commit** (txid held in `PendingReveals`) must not be bumpable — replacing it would invalidate the pre-signed reveal (R1.0.1).
- Wallet panel UI: a "Pending transactions" accordion section (same conventions as Addresses) with per-row Bump action, inline fee-rate entry (0.1 sat/vB step, max 10,000 sat/vB, suggested default from `fee_rates_estimate`), explicit Confirm, success (new txid) and error surfaces.
- Watch-only sessions: list is visible; Bump is visible but disabled ("Hardware wallet required to sign" pattern from 3.8 — actual HW Send signing is Phase 8; Ledger commit-style PSBT signing is reused where the session signer supports it).

**Not in scope**

- CPFP (PRD/plan exclusion).
- Payout transactions (PRD §6 — excluded from program).
- Confirmed transaction history (PRD only requires unconfirmed sent txs).
- Incoming-only unconfirmed txs in this list (balance/addresses already surface them per R1.5/R1.6; PRD §4.3.3 says "sent from the Admin Wallet").
- Send form (Phase 6), QR (Phase 7), HW verify-on-device (Phase 8).
- Durable persistence of replaced-tx bookkeeping — BDK canonicalization + Electrum sync already converge the wallet state.

## Technical Design

### Flow

```text
React (wallet panel)
  ├─ admin_wallet_list_unconfirmed_txs ──► WalletService::list_unconfirmed_sent_txs
  │                                          wallet.transactions() → filter unconfirmed ∧ sent>0
  │                                          + calculate_fee / calculate_fee_rate / vsize / RBF flag
  │                                          + PendingReveals txid cross-check (is_governance_commit)
  │
  └─ admin_wallet_bump_fee { txid, feeRateSatPerKvb }
       0. validate rate (FeeRate::new) → InvalidFeeRate; best-effort sync()
       └─► WalletService::bump_fee                  (no network I/O of its own except broadcast)
             1. signer present? else ReadOnly       (before any wallet lock or broadcast)
             2. signer.allowed_on(network)? else SignerNotAllowedOnNetwork
             3. txid in PendingReveals? → GovernanceCommitNotReplaceable; parse txid → InvalidTxid
             4. wallet.build_fee_bump(txid) + fee_rate(new) → PSBT
                (BDK rejects confirmed / non-RBF / unknown txid; FeeTooLow/FeeRateTooLow if not above old)
             5. sign via PsbtSigner port (shared sign_and_finalize_psbt helper, same as commit)
             6. broadcast single tx: Electrum first → node RPC fallback (TxBroadcaster::broadcast_one)
             7. return { newTxid, replacedTxid, feeSats, feeRateSatPerKvb }
       8. best-effort sync() so the panel reflects the replacement
```

### Rust — types and functions

**`application/wallet_transactions.rs`** (new module; `impl WalletService` extension + DTOs + error)

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnconfirmedTxDto {
    pub txid: String,
    /// Sats sent from the wallet (sum of wallet-owned inputs).
    pub sent_sats: u64,
    /// Sats received back to the wallet (change + self-transfers).
    pub received_sats: u64,
    /// received - sent. Negative for sends.
    pub net_sats: i64,
    /// Absolute fee. None when an input's prev-txout is unknown to the wallet.
    pub fee_sats: Option<u64>,
    /// Current fee rate in sat/kvB (Phase 4 unit convention). None when fee is unknown.
    pub fee_rate_sat_per_kvb: Option<u64>,
    pub vsize_vbytes: u64,
    /// True when at least one input signals BIP-125 replaceability.
    pub is_rbf_signaling: bool,
    /// True when this txid is a governance commit with a pending pre-signed reveal —
    /// bumping it would invalidate the reveal, so the UI must not offer Bump.
    pub is_governance_commit: bool,
    /// Mempool last-seen, unix seconds. None when the indexer gave no timestamp.
    pub last_seen_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpFeeResultDto {
    pub new_txid: String,
    pub replaced_txid: String,
    pub fee_sats: u64,
    pub fee_rate_sat_per_kvb: u64,
}

// As implemented (display strings via #[error] in wallet_transactions.rs;
// bump_error_code() provides the stable tagged code per variant)
#[derive(Debug, thiserror::Error)]
pub enum BumpFeeError {
    ReadOnly,
    SignerNotAllowedOnNetwork,
    InvalidTxid { txid: String },
    TxNotFound { txid: String },
    TxAlreadyConfirmed { txid: String },
    TxNotReplaceable { txid: String },
    GovernanceCommitNotReplaceable { txid: String },
    /// BDK `CreateTxError::FeeTooLow` — replacement absolute fee not above the original.
    FeeTooLow { required_fee_sats: u64 },
    /// BDK `CreateTxError::FeeRateTooLow` — replacement rate not above the original (sat/kvB).
    FeeRateTooLow { required_sat_per_kvb: u64 },
    InsufficientFunds { message: String },
    BuildFailed { message: String },
    SignFailed { message: String },
    InvalidFeeRate(#[from] crate::domain::fee_rate::FeeRateError),
    BroadcastFailed { message: String },
}
```

Functions (production):

- `WalletService::list_unconfirmed_sent_txs(&self, pending_commit_txids: &HashSet<Txid>) -> Result<Vec<UnconfirmedTxDto>, AdminWalletError>`
  Pure read over the BDK graph: `wallet.transactions()` filtered to `ChainPosition::Unconfirmed` AND `sent > 0` (`sent_and_received`). Fee via `calculate_fee` (→ `None` on `CalculateFeeError`, never an error for the whole list). RBF flag: any input `sequence <= 0xFFFFFFFD`. Sorted newest-first by `last_seen`.
- `WalletService::bump_fee(&self, txid: &str, new_rate: FeeRate, pending_commit_txids: &HashSet<String>, broadcasters: &[Arc<dyn TxBroadcaster>]) -> Result<BumpFeeResultDto, BumpFeeError>`
  Steps 1–7 and 9 above; **does not sync** — the IPC command syncs best-effort before and after (decision: keeps the use-case hermetic and unit-testable without network; a stale view is ultimately caught by the node rejecting the replacement). `BuildFeeBumpError` maps: `TransactionNotFound|UnknownUtxo → TxNotFound`, `TransactionConfirmed → TxAlreadyConfirmed`, `IrreplaceableTransaction → TxNotReplaceable`, `FeeRateUnavailable → BuildFailed`. `CreateTxError` maps: `FeeTooLow → FeeTooLow` (sats), `FeeRateTooLow → FeeRateTooLow` (sat/kvB), `CoinSelection → InsufficientFunds`.
- `WalletService::sign_and_finalize_psbt(&self, psbt) -> Result<Transaction, AdminWalletError>` — **refactor**: extract the existing signer-dispatch + finalize + extract block from `build_and_sign_tx` so commit funding and fee-bump share one signing path (no behavior change for commits).

**`application/tx_broadcaster.rs`** — extend the port:

```rust
#[async_trait]
pub trait TxBroadcaster: Send + Sync {
    fn name(&self) -> &'static str;
    async fn broadcast_pair(&self, commit_hex: &str, reveal_hex: &str) -> Result<(), TxBroadcastError>;
    /// Submit a single signed transaction. Same idempotency rule (already-known ⇒ Ok).
    async fn broadcast_one(&self, tx_hex: &str) -> Result<(), TxBroadcastError>;
}
```

Plus a free function `broadcast_single_with_fallback(broadcasters, tx_hex) -> Result<(), Vec<TxBroadcastError>>` mirroring the pair-broadcast fallback walk in `submit_commit_then_reveal`. RBF rejections from the node/Electrum ("insufficient fee", "min relay fee not met") surface verbatim in `BroadcastFailed`.

**Implementations:** `ElectrumBroadcaster::broadcast_one` (its private helper already exists — promote through the trait), `NodeBroadcaster::broadcast_one` (`sendrawtransaction`).

**`commands/admin_wallet.rs`** — two new IPC commands, registered in **both** `attach_production` and `attach_with_dev_signing` (capability is enforced per-signer at runtime, same as broadcast):

- `admin_wallet_list_unconfirmed_txs(wallet_session, pending_reveals) -> Result<Vec<UnconfirmedTxDto>, String>`
- `admin_wallet_bump_fee(input: { txid, fee_rate_sat_per_kvb }, wallet_session, pending_reveals, node_config, btc_rpc) -> Result<BumpFeeResultDto, String>`
  Builds the broadcaster chain exactly like `commands/proposals.rs` (`ElectrumBroadcaster::new(cfg.electrum_url())`, `NodeBroadcaster::new(btc_rpc)`); validates the rate with `FeeRate::new` against the live min-relay. Errors serialize tagged `{ "type", "message" }` like `serialize_wallet_error`.

`PendingReveals` already lives in Tauri managed state (R1.0.1); the commands read its commit txids. If its current API does not expose them, add a read-only `pending_commit_txids()` accessor (no lifecycle change).

### Frontend — React

**`api/admin-wallet.ts`** (transport):

- `UnconfirmedTxDto`, `BumpFeeInput`, `BumpFeeResultDto` types (camelCase mirror).
- `listAdminWalletUnconfirmedTxs(): Promise<ApiResult<UnconfirmedTxDto[]>>`
- `bumpAdminWalletFee(input: BumpFeeInput): Promise<ApiResult<BumpFeeResultDto>>`
- Extend `AdminWalletError` union with the new tagged bump variants (`TxNotFound`, `TxAlreadyConfirmed`, `TxNotReplaceable`, `GovernanceCommitNotReplaceable`, `FeeRateTooLow`, `BuildFailed`, `SignFailed`, `BroadcastFailed`, `InvalidFeeRate`).

**`domain/admin-wallet/model/`** (pure view-model, unit-tested):

- `compose-unconfirmed-tx-rows.ts` — DTO → `UnconfirmedTxView`: truncated txid (reuse `trunc-txid.ts`), signed net amount (reuse `format-signed-sats.ts`), fee-rate label in sat/vB (`satPerKvb / 1000`, one decimal), relative time (reuse `relative-time.ts`), `canBump` = `isRbfSignaling && !isGovernanceCommit`, `bumpDisabledReason` (`'not-rbf' | 'governance-commit' | null`).
- `format-admin-wallet-error.ts` — add copy for the new error variants (high-signal, action-oriented).

**`domain/admin-wallet/hooks/`**:

- `use-unconfirmed-txs.ts` — fetch/refresh, `{ data, isLoading, error, refresh }`, same contract shape as `use-addresses-with-balance`. `refresh` is a stable `useCallback` (wallet-panel rule: on-open effect deps must be stable callbacks only).
- `use-bump-fee.ts` — `{ bump(txid, satPerKvb), isSubmitting, result, error, reset }`; on success triggers the panel's `syncAndRefresh`.
- `use-wallet-panel-data.ts` — wire the new hook; add `'transactions'` to `WalletPanelSection`; include the unconfirmed list refresh in `syncAndRefresh`.

**`domain/admin-wallet/components/`**:

- `unconfirmed-txs-list.tsx` — accordion section `Pending transactions · N` (same chrome as `AddressesWithBalanceList`: collapsed by default, loading skeleton, error row, empty state "No pending transactions").
- `unconfirmed-tx-row.tsx` — txid (truncated + `CopyButton`), signed amount, current fee rate, RBF/`Not replaceable`/`Governance` badge, relative time, `Bump fee` button (hidden→disabled with reason when `!canBump` or watch-only).
- `bump-fee-form.tsx` — inline expansion under the row (no modal; consistent with the slide-over): current rate → new-rate stepper input (0.1 sat/vB step, min = current + 0.1, max 10,000 — reuse `fee-selection/model/fee-rate.ts` helpers and `useFeePresets` for the suggested default = Fast preset clamped to ≥ min), estimated new total fee (`feeSats(rate, vsize)`), explicit **Confirm bump** button, busy state, success state with new txid + copy, error state with mapped message.
- `wallet-panel-content.tsx` — render the section between Addresses and the sync footer.

### Production code vs. test helpers

- **Production:** everything listed above (Rust module, trait method, IPC commands, API client, hooks, view-models, components).
- **Test helpers (never registered as Tauri commands / exported from production paths):**
  - Rust: wallet fixtures that insert funded + unconfirmed txs into a BDK wallet for `list`/`bump` unit tests — `#[cfg(test)]` module helpers (using `bdk_wallet`'s `test-utils` feature as a dev-dependency if needed, else hand-built `Update`s).
  - Rust: `MockBroadcaster` already in `tx_broadcaster.rs::tests` — extend with `broadcast_one` recording.
  - TS: `__fixtures__/make-unconfirmed-tx.ts` following `make-utxo.ts`.

## Test Cases

Tests target production functions only.

**Rust — `list_unconfirmed_sent_txs`**
1. Fresh wallet → empty vec (happy empty path).
2. Wallet with a confirmed funding tx only → empty (confirmed txs excluded).
3. Unconfirmed tx spending a wallet UTXO to an external script → one row: `sent_sats > 0`, `net_sats < 0`, correct `fee_sats`/`vsize`, `is_rbf_signaling = true` (BDK default sequence).
4. Unconfirmed **incoming-only** tx (received, no wallet inputs) → excluded from the list.
5. Tx whose txid is in `pending_commit_txids` → `is_governance_commit = true`.
6. Non-RBF tx (all sequences `0xFFFFFFFE+`) → listed with `is_rbf_signaling = false`.
7. Fee unknown (foreign input) → `fee_sats = None`, row still listed.

**Rust — `bump_fee`**
8. Watch-only session → `ReadOnly` before any RPC/Electrum contact (no broadcasters touched).
9. Unknown txid → `TxNotFound`.
10. Confirmed txid → `TxAlreadyConfirmed`.
11. Non-RBF txid → `TxNotReplaceable`.
12. Txid in `pending_commit_txids` → `GovernanceCommitNotReplaceable` (checked before BDK build).
13. New rate ≤ current rate → `FeeRateTooLow` carrying the required rate (from BDK `FeeTooLow`).
14. Happy path (mnemonic signer, mock broadcaster): returns new txid ≠ replaced txid, fee strictly greater than original; replacement tx pays to the same recipient script; broadcaster received exactly one tx.
15. All broadcasters fail → `BroadcastFailed` aggregating both source errors; wallet state not corrupted (replacement not marked broadcast).
16. First broadcaster fails, second succeeds → Ok (fallback walk).
17. "already known" broadcaster response → Ok (idempotency, reuse `is_already_known`).

**Rust — `TxBroadcaster::broadcast_one`**
18. `NodeBroadcaster::broadcast_one` happy + error mapping (stub RPC, mirrors existing pair tests).
19. `broadcast_single_with_fallback` ordering + error aggregation.

**Rust — IPC layer**
20. Commands registered in both handler sets (extend the existing `invoke.rs` source-inclusion test pattern).
21. No-session → tagged `Disabled` error (same as other admin-wallet commands).
22. Invalid `fee_rate_sat_per_kvb` (0, > max) → tagged `InvalidFeeRate` without touching the wallet.

**TS — view-model / hooks / components**
23. `compose-unconfirmed-tx-rows`: net amount sign, fee-rate formatting (sat/kvB → sat/vB one decimal), `canBump` matrix (rbf × governance), sort order preserved.
24. `format-admin-wallet-error`: copy for each new variant.
25. `use-unconfirmed-txs` contract test (load → data; error path) following `use-addresses-with-balance.test.ts`.
26. `bump-fee-form`: Confirm disabled until rate > current; min/max clamping; success shows new txid; error renders mapped message.
27. Watch-only: row renders Bump disabled with reason (component test).
28. Architecture test (`architecture.test.ts` rules): components import domain types from `model/types.ts`, new hook wired in panel data.

**Manual regtest acceptance (Done-when)**
29. Fund Admin Wallet → send (via existing commit broadcast or `bitcoin-cli` spend of a wallet UTXO is not wallet-sent; use a governance commit broadcast **without** mining) → panel lists the unconfirmed tx → bump with higher rate → node accepts replacement → mine → list empties, balance correct. *(Until Phase 6 Send exists, the only wallet-sent tx on regtest is the governance commit — which is bump-blocked while its reveal is pending. For manual verification, broadcast a plain send from the wallet via a dev-only test route is NOT added; instead verify with a commit whose reveal already confirmed… see Risks below — the practical manual path is documented in the PR test plan.)*

## Module structure

| File | Single responsibility |
|---|---|
| `src-tauri/src/application/wallet_transactions.rs` | Unconfirmed-sent-tx listing and RBF fee-bump use-cases over `WalletService` (DTOs + `BumpFeeError` + impl block) |
| `src-tauri/src/application/tx_broadcaster.rs` (edit) | Broadcast port — gains single-tx submission alongside the pair |
| `src-tauri/src/infrastructure/electrum_broadcaster.rs` / `node_broadcaster.rs` (edit) | Transport implementations of the port |
| `src-tauri/src/commands/admin_wallet.rs` (edit) | IPC boundary: session/state lookup, DTO/error serialization |
| `src/api/admin-wallet.ts` (edit) | Typed transport client for the two new commands |
| `src/domain/admin-wallet/model/compose-unconfirmed-tx-rows.ts` | Pure DTO → view-model mapping for the tx list |
| `src/domain/admin-wallet/hooks/use-unconfirmed-txs.ts` | Fetch/refresh state for the unconfirmed tx list |
| `src/domain/admin-wallet/hooks/use-bump-fee.ts` | Bump submission state machine (idle → submitting → success/error) |
| `src/domain/admin-wallet/components/unconfirmed-txs-list.tsx` | Accordion section rendering rows + states |
| `src/domain/admin-wallet/components/unconfirmed-tx-row.tsx` | One transaction row + bump affordance |
| `src/domain/admin-wallet/components/bump-fee-form.tsx` | Inline rate entry + confirm for one bump |

Dependency direction: `wallet_transactions.rs` (business logic) depends on the `TxBroadcaster` trait and `PsbtSigner` port — never on concrete transports. DTOs live with the use-case module; broadcast error types live with the port. React components receive prepared view-models and emit intents; only hooks call the API layer.

## Risks / notes

- **Manual-verification dependency:** until Phase 6 (Send) ships, the only app-generated wallet spend is the governance commit, which is intentionally bump-blocked while its reveal is pending. The PR test plan will verify bump on regtest by clearing the pending reveal first (mine the reveal? no — reveal spends the commit; instead: broadcast a commit whose reveal broadcast **failed** is also blocked…). Practical path: a regtest-only manual scenario where the unconfirmed sent tx is produced by importing the Admin Wallet descriptor into `bitcoin-cli`/BDK externally is heavyweight — accepted approach: **temporarily exercise `bump_fee` via Rust integration test against the local stack** (test creates, signs, and broadcasts a send through `WalletService` internals) + UI verified with the list rendering and the bump flow against that tx. This keeps production surface clean (no dev Send IPC) while proving the e2e path on regtest.
- **Replacement visibility:** between broadcast and next Electrum sync both txs may appear; step 8's immediate `sync()` plus the list's canonical filter (BDK `transactions()` is canonical-only) collapse this.
- **`FeeRate` unit:** all IPC uses sat/kvB (Phase 4 convention); UI converts to sat/vB for display only.

## Done when

- Regtest, mnemonic login: panel shows unconfirmed sent txs; bumping one with a higher rate yields a node-accepted replacement and the panel converges after sync.
- PRD §4.3.3 → **PASS** in `admin-wallet-prd-compliance.md`; plan traceability table fixed (Phase 5 = Transactions + fee-bump, Phase 6 = Send) and Phase 5 marked ✅ with PR link.
- All CI green: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run format:check && npm run lint && npm run build`.
