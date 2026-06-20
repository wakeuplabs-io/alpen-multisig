# Spec: Admin Wallet — Transactions + fee-bump (RBF / CPFP) — Phase 5

**PRD:** [`03-prd-update.md`](../0-prd/03-prd-update.md) §4.3.3 — *"Transactions: The user MUST be able to see each unconfirmed transaction sent from the Admin Wallet and have the ability to bump the fee."*
**Plan:** [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) Phase 5 (Transactions + fee-bump).
**Compliance:** [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md) §4.3.3 — **PASS** (this phase).
**Status:** Implemented — PR [#276](https://github.com/wakeuplabs-io/alpen-multisig/pull/276).

## Objective

Let the signer see every unconfirmed transaction **sent from** the Admin Wallet in the wallet slide-over and bump its fee, on regtest with the mnemonic login. This closes PRD §4.3.3.

Two bump methods, selected automatically by transaction kind:

- **RBF** for plain wallet sends that signal BIP-125 — replace the tx with a higher-fee version.
- **CPFP** for **governance commits** with a pending pre-signed reveal — the commit cannot be replaced (a new commit txid would invalidate the pre-signed reveal; the ephemeral envelope key is dropped after signing, R1.0.1), but the reveal pays change back to the Admin Wallet. A child transaction spending that change raises the effective fee rate of the whole `commit → reveal → child` package, which miners evaluate together.

Why now: Phase 4 delivered the fee-rate domain (`FeeRate`, `FeeEstimationService`, `FeeRateSelector` pattern) and R2 delivered Electrum sync with mempool visibility — both prerequisites for a correct unconfirmed-tx view and replacement broadcast.

## Scope

**In scope**

- List unconfirmed transactions where the Admin Wallet contributed inputs (sent), with txid, net amount, current fee / fee rate, vsize, RBF-signaling flag, and first-seen time. Governance commits additionally report the **package** fee / vsize / rate (commit + reveal).
- Fee-bump (RBF) of an unconfirmed, RBF-signaling wallet transaction: build via BDK `build_fee_bump`, sign via the session `PsbtSigner` (R1.1 port — mnemonic software signer; Ledger path reuses the same device flow as commit signing), broadcast Electrum-first with node-RPC fallback.
- Fee-bump (CPFP) of a pending **governance commit** (txid held in `PendingReveals`): build a child tx that spends the reveal's wallet-owned change output with an absolute fee chosen so the `commit+reveal+child` package reaches the requested rate; sign via the same `PsbtSigner` port; broadcast Electrum-first.
- Single-transaction broadcast capability on the existing `TxBroadcaster` port (today it only broadcasts commit+reveal pairs).
- Wallet panel UI: a "Pending transactions" accordion section (same conventions as Addresses) with per-row Bump action, inline fee-rate entry (0.1 sat/vB step, max 10,000 sat/vB, suggested default from `fee_rates_estimate`), explicit Confirm, success (new/child txid) and error surfaces. Governance rows show package fee/rate and bump via CPFP.
- Watch-only sessions: list is visible; Bump is visible but disabled ("Hardware wallet required to sign" pattern from 3.8 — actual HW Send signing is Phase 8; Ledger commit-style PSBT signing is reused where the session signer supports it).

**Not in scope**

- CPFP for non-governance transactions (RBF covers them; CPFP is used only where RBF is structurally impossible).
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
  │                                          + PendingReveals commit→reveal cross-check:
  │                                              is_governance_commit, bump_method = cpfp,
  │                                              package fee/vsize/rate (commit + reveal, from the graph)
  │
  └─ admin_wallet_bump_fee { txid, feeRateSatPerKvb }
       0. validate rate (FeeRate::new) → InvalidFeeRate; best-effort sync()
       └─► WalletService::bump_fee                  (no network I/O of its own except broadcast)
             1. signer present? else ReadOnly       (before any wallet lock or broadcast)
             2. signer.allowed_on(network)? else SignerNotAllowedOnNetwork
             3. parse txid → InvalidTxid; txid in PendingReveals (commit→reveal map)?
                ├─ yes → CPFP path:
                │    a. locate reveal tx in the graph; find its wallet-owned change output
                │       (missing / already spent → CpfpOutputUnavailable)
                │    b. package = commit fee+vsize + reveal fee+vsize
                │       child_fee = ceil(rate × (package_vsize + child_vsize_est)) − package_fee
                │       (child_vsize_est = 111 vB: 1 P2TR keypath input + 1 P2TR output)
                │       child_fee < child min-relay floor → FeeRateTooLow { required package rate }
                │    c. build child: add_utxo(reveal change) + drain_to(next internal address)
                │       + fee_absolute(child_fee)  (BDK adds more wallet inputs only if needed)
                │    └─ continue at 5
                └─ no → RBF path:
                     4. wallet.build_fee_bump(txid) + fee_rate(new) → PSBT
                        (BDK rejects confirmed / non-RBF / unknown; FeeTooLow/FeeRateTooLow if not above old)
             5. sign via PsbtSigner port (shared sign_and_finalize_psbt helper, same as commit)
             6. broadcast single tx: Electrum first → node RPC fallback (TxBroadcaster::broadcast_one)
             7. return { newTxid, targetTxid, feeSats, feeRateSatPerKvb, method }
                (CPFP: newTxid = child txid, feeSats = child fee, rate = resulting package rate)
       8. best-effort sync() so the panel reflects the replacement/child
```

**CPFP after CPFP:** once a child exists, the reveal change is spent, so a second bump of the commit returns `CpfpOutputUnavailable`. The child itself is a plain RBF-signaling wallet send — it appears in the list as a normal row and is bumped via RBF, which is the standard way to iterate a CPFP anchor.

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
    /// bumped via CPFP on the reveal's change output (RBF would invalidate the reveal).
    pub is_governance_commit: bool,
    /// How this tx can be bumped: governance commit → Cpfp, RBF-signaling → Rbf, else None.
    pub bump_method: Option<BumpMethod>,
    /// commit fee + reveal fee — Some only for governance commits whose reveal is in the graph.
    pub package_fee_sats: Option<u64>,
    pub package_vsize_vbytes: Option<u64>,
    /// Effective package rate in sat/kvB — what a CPFP bump must exceed.
    pub package_fee_rate_sat_per_kvb: Option<u64>,
    /// Mempool last-seen, unix seconds. None when the indexer gave no timestamp.
    pub last_seen_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BumpMethod { Rbf, Cpfp }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpFeeResultDto {
    pub new_txid: String,     // RBF: replacement txid · CPFP: child txid
    pub target_txid: String,  // the txid the user asked to bump
    pub fee_sats: u64,        // RBF: replacement fee · CPFP: child fee
    pub fee_rate_sat_per_kvb: u64, // RBF: replacement rate · CPFP: resulting package rate
    pub method: BumpMethod,
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
    /// CPFP anchor (reveal's wallet-owned change output) missing from the graph or already
    /// spent — sync first, or bump the existing child via RBF.
    CpfpOutputUnavailable { txid: String, message: String },
    /// BDK `CreateTxError::FeeTooLow` — replacement absolute fee not above the original.
    FeeTooLow { required_fee_sats: u64 },
    /// RBF: BDK `CreateTxError::FeeRateTooLow`. CPFP: requested rate does not exceed the
    /// current package rate (required carries the minimum viable package rate, sat/kvB).
    FeeRateTooLow { required_sat_per_kvb: u64 },
    InsufficientFunds { message: String },
    BuildFailed { message: String },
    SignFailed { message: String },
    InvalidFeeRate(#[from] crate::domain::fee_rate::FeeRateError),
    BroadcastFailed { message: String },
}
```

Functions (production):

- `WalletService::list_unconfirmed_sent_txs(&self, pending_commit_to_reveal: &HashMap<String, String>) -> Result<Vec<UnconfirmedTxDto>, AdminWalletError>`
  Pure read over the BDK graph: `wallet.transactions()` filtered to `ChainPosition::Unconfirmed` AND `sent > 0` (`sent_and_received`). Fee via `calculate_fee` (→ `None` on `CalculateFeeError`, never an error for the whole list). RBF flag: any input `sequence <= 0xFFFFFFFD`. For governance commits, looks up the reveal in the graph to compute package fee/vsize/rate. Sorted newest-first by `last_seen`.
- `WalletService::bump_fee(&self, txid: &str, new_rate: FeeRate, pending_commit_to_reveal: &HashMap<String, String>, broadcasters: &[Arc<dyn TxBroadcaster>]) -> Result<BumpFeeResultDto, BumpFeeError>`
  Steps 1–7 and 9 above, dispatching to the CPFP path when the txid is a pending governance commit; **does not sync** — the IPC command syncs best-effort before and after (decision: keeps the use-case hermetic and unit-testable without network; a stale view is ultimately caught by the node rejecting the replacement). `BuildFeeBumpError` maps: `TransactionNotFound|UnknownUtxo → TxNotFound`, `TransactionConfirmed → TxAlreadyConfirmed`, `IrreplaceableTransaction → TxNotReplaceable`, `FeeRateUnavailable → BuildFailed`. `CreateTxError` maps: `FeeTooLow → FeeTooLow` (sats), `FeeRateTooLow → FeeRateTooLow` (sat/kvB), `CoinSelection → InsufficientFunds`.
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

`PendingReveals` lives in Tauri managed state (R1.0.1) and is **persisted to disk** via
[`pending_reveals_store.rs`](../../desktop-app/src-tauri/src/infrastructure/pending_reveals_store.rs)
(`pending-reveals.json` under the app data dir). Commands read it through a read-only
`pending_commit_to_reveal()` accessor (commit txid → reveal txid map).

### Frontend — React

**`api/admin-wallet.ts`** (transport):

- `UnconfirmedTxDto`, `BumpFeeInput`, `BumpFeeResultDto` types (camelCase mirror).
- `listAdminWalletUnconfirmedTxs(): Promise<ApiResult<UnconfirmedTxDto[]>>`
- `bumpAdminWalletFee(input: BumpFeeInput): Promise<ApiResult<BumpFeeResultDto>>`
- Extend `AdminWalletError` union with the new tagged bump variants (`TxNotFound`, `TxAlreadyConfirmed`, `TxNotReplaceable`, `CpfpOutputUnavailable`, `FeeRateTooLow`, `BuildFailed`, `SignFailed`, `BroadcastFailed`, `InvalidFeeRate`).

**`domain/admin-wallet/model/`** (pure view-model, unit-tested):

- `compose-unconfirmed-tx-rows.ts` — DTO → `UnconfirmedTxView`: truncated txid (reuse `trunc-txid.ts`), signed net amount (reuse `format-signed-sats.ts`), fee-rate label in sat/vB (`satPerKvb / 1000`, one decimal), relative time (reuse `relative-time.ts`), `bumpMethod` from the DTO, `canBump` = `bumpMethod !== null`, `bumpDisabledReason` (`'not-rbf' | null`). Governance rows display the **package** fee/rate (fallback: commit's own) and use the package rate as the floor the new rate must exceed.
- `format-admin-wallet-error.ts` — add copy for the new error variants (high-signal, action-oriented).

**`domain/admin-wallet/hooks/`**:

- `use-unconfirmed-txs.ts` — fetch/refresh, `{ data, isLoading, error, refresh }`, same contract shape as `use-addresses-with-balance`. `refresh` is a stable `useCallback` (wallet-panel rule: on-open effect deps must be stable callbacks only).
- `use-bump-fee.ts` — `{ bump(txid, satPerKvb), isSubmitting, result, error, reset }`; on success triggers the panel's `syncAndRefresh`.
- `use-wallet-panel-data.ts` — wire the new hook; add `'transactions'` to `WalletPanelSection`; include the unconfirmed list refresh in `syncAndRefresh`.

**`domain/admin-wallet/components/`**:

- `unconfirmed-txs-list.tsx` — accordion section `Pending transactions · N` (same chrome as `AddressesWithBalanceList`: collapsed by default, loading skeleton, error row, empty state "No pending transactions").
- `unconfirmed-tx-row.tsx` — txid (truncated + `CopyButton`), signed amount, current fee rate (package fee/rate for governance rows), RBF/`Not replaceable`/`Governance` badge, relative time, `Bump fee` button (disabled with reason when `!canBump` or watch-only; tooltip explains CPFP on governance rows).
- `bump-fee-form.tsx` — inline expansion under the row (no modal; consistent with the slide-over): current rate → new-rate stepper input (0.1 sat/vB step, min = current + 0.1 — package rate for CPFP —, max 10,000 — reuse `fee-selection/model/fee-rate.ts` helpers and `useFeePresets` for the suggested default = Fast preset clamped to ≥ min), estimated cost (RBF: `feeSats(rate, vsize)`; CPFP: estimated **child fee** = `feeSats(rate, packageVsize + 111) − packageFee`), explicit **Confirm bump** button, busy state, success state with new/child txid + copy (CPFP copy explains a child tx was broadcast), error state with mapped message.
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
5. Tx whose txid is in the pending commit→reveal map → `is_governance_commit = true`, `bump_method = cpfp`, package fee/vsize/rate = commit + reveal sums.
6. Non-RBF tx (all sequences `0xFFFFFFFE+`) → listed with `is_rbf_signaling = false`, `bump_method = None`.
7. Fee unknown (foreign input) → `fee_sats = None`, row still listed.

**Rust — `bump_fee` (RBF path)**
8. Watch-only session → `ReadOnly` before any RPC/Electrum contact (no broadcasters touched).
9. Unknown txid → `TxNotFound`.
10. Confirmed txid → `TxAlreadyConfirmed`.
11. Non-RBF txid → `TxNotReplaceable`.
12. New rate ≤ current rate → `FeeRateTooLow` carrying the required rate (from BDK `FeeTooLow`).
13. Happy path (mnemonic signer, mock broadcaster): returns new txid ≠ target txid, `method = rbf`, fee strictly greater than original; replacement tx pays to the same recipient script; broadcaster received exactly one tx.
14. All broadcasters fail → `BroadcastFailed` aggregating both source errors; wallet state not corrupted (replacement not marked broadcast).
15. First broadcaster fails, second succeeds → Ok (fallback walk).
16. "already known" broadcaster response → Ok (idempotency, reuse `is_already_known`).

**Rust — `bump_fee` (CPFP path, governance commit)**
17. Happy path: child tx spends the reveal's change outpoint, drains to a wallet internal address, `method = cpfp`, child fee makes the resulting package rate ≥ requested; broadcaster received exactly the child.
18. Requested rate ≤ current package rate → `FeeRateTooLow` carrying the minimum viable package rate.
19. Reveal tx not in the wallet graph (not yet synced) → `CpfpOutputUnavailable`.
20. Reveal change already spent (prior CPFP) → `CpfpOutputUnavailable`; the child row itself is RBF-bumpable.

**Rust — `TxBroadcaster::broadcast_one`**
21. `NodeBroadcaster::broadcast_one` happy + error mapping (stub RPC, mirrors existing pair tests).
22. `broadcast_single_with_fallback` ordering + error aggregation.

**Rust — IPC layer**
23. Commands registered in both handler sets (extend the existing `invoke.rs` source-inclusion test pattern).
24. No-session → tagged `Disabled` error (same as other admin-wallet commands).
25. Invalid `fee_rate_sat_per_kvb` (0, > max) → tagged `InvalidFeeRate` without touching the wallet.

**TS — view-model / hooks / components**
26. `compose-unconfirmed-tx-rows`: net amount sign, fee-rate formatting (sat/kvB → sat/vB one decimal), `canBump`/`bumpMethod` matrix (rbf × governance × non-rbf), package fee/rate displayed for governance rows (with fallback to the commit's own), sort order preserved.
27. `format-admin-wallet-error`: copy for each new variant (incl. `CpfpOutputUnavailable`).
28. `use-unconfirmed-txs` contract test (load → data; error path) following `use-addresses-with-balance.test.ts`.
29. `bump-fee-form`: Confirm disabled until rate > current; min/max clamping; success shows new/child txid with method-specific copy; CPFP shows estimated child fee; error renders mapped message.
30. Watch-only: row renders Bump disabled with reason (component test).
31. Architecture test (`architecture.test.ts` rules): components import domain types from `model/types.ts`, new hook wired in panel data.

**Manual regtest acceptance (Done-when)**
32. Broadcast a governance proposal (commit+reveal land in mempool, no mining) → panel lists the commit with the GOVERNANCE badge and package fee/rate → bump with a higher rate → node accepts the CPFP child → mine → list empties, balance correct. This makes §4.3.3 manually verifiable **today**, without waiting for Phase 6 Send.

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

- **Why CPFP (not RBF) for governance commits:** the reveal is pre-signed with an ephemeral envelope key that is dropped right after signing (R1.0.1); replacing the commit changes the txid the reveal spends, permanently orphaning the action. The reveal's change output returns to the Admin Wallet, so a child spend is always constructible while the package is unconfirmed and unspent — no key custody change, no protocol deviation.
- **Child vsize estimate:** the child fee is computed against a 111 vB estimate (1 P2TR keypath input + 1 P2TR output). If BDK must add an extra wallet input to cover the fee, the realized package rate lands slightly below the requested one (~58 vB per extra input) — acceptable for a fee accelerator; the user can always bump the child via RBF.
- **Package relay:** regtest/modern Core evaluates ancestor packages for mining; the child confirms only with its ancestors, which is exactly the acceleration intent.
- **Replacement visibility:** between broadcast and next Electrum sync both txs may appear; step 8's immediate `sync()` plus the list's canonical filter (BDK `transactions()` is canonical-only) collapse this.
- **`FeeRate` unit:** all IPC uses sat/kvB (Phase 4 convention); UI converts to sat/vB for display only.

## Done when

- Regtest, mnemonic login: panel shows unconfirmed sent txs; bumping a plain send yields a node-accepted replacement (RBF) and bumping a pending governance commit yields a node-accepted child (CPFP); the panel converges after sync.
- PRD §4.3.3 → **PASS** in `admin-wallet-prd-compliance.md`; plan traceability table fixed (Phase 5 = Transactions + fee-bump, Phase 6 = Send) and Phase 5 marked ✅ with PR link.
- All CI green: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run format:check && npm run lint && npm run build`.
