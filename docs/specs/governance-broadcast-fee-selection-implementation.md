# Spec: Governance Broadcast — Fee Rate Selection (Technical Implementation)

> **Status: ✅ Complete** — M1 (PR [#267](https://github.com/wakeuplabs-io/alpen-multisig/pull/267)), M2+M3 (PR [#273](https://github.com/wakeuplabs-io/alpen-multisig/pull/273)).

> Implements the functional contract in
> [`governance-broadcast-fee-selection.md`](./governance-broadcast-fee-selection.md).
> That document defines *what* the signer experiences; this one defines *how* it is built.
> If the two ever disagree, the functional document wins and this one must be updated.

## Objective

Let Strata/Alpen Administrator signers choose the fee rate (Slow / Medium / Fast presets or a
Custom sat/vB value) before broadcasting the commit/reveal pair of an approved governance
proposal, with:

- live estimates from the Bitcoin node (`estimatesmartfee`), Electrum fallback, and an honest
  static fallback when both are unavailable;
- a security margin per preset (+20% / +10% / +5%);
- 0.1 sat/vB granularity for Custom entry, bounded by the node's min relay fee and 10,000 sat/vB;
- RBF (BIP-125) signaling guaranteed by regression tests;
- an Electrum-first broadcast path with node-RPC fallback and a manual copy-hex escape hatch.

The fee-selection building blocks (domain types, estimation service, UI component) are designed
for reuse by wallet Send (Phase 6) and shared Send UX (Phase 9).

## Scope

### Included

1. **Fee rate representation** — a validated integer domain type shared by all flows
   (`sat/kvB`, see [Units](#1-units--arithmetic-no-floats-at-boundaries)).
2. **Fee estimation** — a `FeeEstimator` port with node and Electrum implementations, a
   `FeeEstimationService` that produces the three presets (margins, rounding, clamping), and a
   new Tauri command `fee_rates_estimate`.
3. **Fee selection UI** — a reusable `domain/fee-selection/` feature context (model, hook,
   presentational `FeeRateSelector`), integrated into:
   - `BroadcastDetailsCard` / `useBroadcastProposal` (covers the governance broadcast screen
     **and** the cancel-broadcast screen, which reuses the same hook), and
   - the manual broadcast flow (`proposals_prepare_broadcast_manual` / `proposals_broadcast_manual`).
4. **Plumbing the selected rate** — `BroadcastInput` / `BroadcastManualInput` gain a required
   `feeRateSatPerKvb`; `submit_commit_then_reveal` and `broadcast_manual` stop estimating
   internally and use the caller-provided rate for both commit (BDK) and reveal (fixed-vsize).
5. **RBF regression tests** — commit and reveal must signal BIP-125.
6. **Broadcast path reorder (M3)** — `TxBroadcaster` port: Electrum first, node RPC fallback,
   typed error carrying both raw tx hexes for manual clipboard broadcast when everything fails.

### NOT included

- Fee bumping of already-broadcast transactions (Phase 5 — Transactions + fee-bump).
- Wallet Send UI (Phase 6) and shared Send UX (Phase 9) — they will consume the modules built
  here but are specced separately.
- UTXO selection / coin control (BDK's default coin selection stays as is).
- Making RBF user-configurable (explicitly non-negotiable in the functional spec).
- Orchestrator (`orchestrator-be`) changes — fee selection is entirely desktop-side; the
  orchestrator never sees fee rates.
- Persisting the user's last fee choice across sessions.

### Delivery milestones (one PR each, individually shippable)

| Milestone | Contents |
|-----------|----------|
| **M1** | Domain `FeeRate` type, strict node estimator, `FeeEstimationService` (node + static fallback), `fee_rates_estimate` command, full UI (`fee-selection` context + integration), rate plumbed through broadcast commands, RBF regression tests |
| **M2** | Electrum fee estimation fallback + last-known-good in-memory cache |
| **M3** | `TxBroadcaster` port: Electrum-first broadcast, node fallback, manual copy-hex UI |

M1 alone satisfies the core PRD requirement (manual sat/vB rate, presets, default Medium).
M2/M3 complete the functional spec's §3.1 fallback and §6 broadcast path.

---

## Technical Design

### 1. Units & arithmetic (no floats at boundaries)

The PRD requires 0.1 sat/vB increments, so integer sat/vB (the current `u64` plumbing) cannot
represent custom rates. All fee rates cross every boundary (domain, IPC, commands) as:

> **`u64` satoshis per virtual kilobyte (sat/kvB)** — `1 sat/vB = 1_000 sat/kvB`,
> `0.1 sat/vB = 100 sat/kvB`.

This is exact for every 0.1 step, matches Bitcoin Core's own feerate denomination
(`BTC/kvB`, just scaled), and stays integer all the way.

Canonical constants (Rust `domain/fee_rate.rs`, mirrored in TS `domain/fee-selection/model/fee-rate.ts`):

```rust
/// 0.1 sat/vB — the UI increment.
pub const FEE_RATE_STEP_SAT_PER_KVB: u64 = 100;
/// 10,000 sat/vB — hard ceiling from the PRD.
pub const MAX_FEE_RATE_SAT_PER_KVB: u64 = 10_000_000;
/// 1 sat/vB — used when no relay-fee source is reachable.
pub const FALLBACK_MIN_RELAY_SAT_PER_KVB: u64 = 1_000;
```

Conversions (all integer, all rounding **up** so we never underpay):

| Conversion | Formula |
|------------|---------|
| Core `estimatesmartfee` feerate (BTC/kvB, f64) → sat/kvB | `(feerate * 1e8).ceil() as u64` (single float touchpoint, at the RPC parsing edge only) |
| Electrum `blockchain.estimatefee` (BTC/kB, f64) → sat/kvB | same formula; `-1` result = "no estimate" → error |
| sat/kvB → `bdk_wallet::bitcoin::FeeRate` | `FeeRate::from_sat_per_kwu(sat_per_kvb.div_ceil(4))` (1 vB = 4 WU) |
| absolute fee for `v` vbytes | `(sat_per_kvb * v).div_ceil(1000)` |
| margin `+p%` | `(rate * (100 + p)).div_ceil(100)` |
| round up to UI step | `rate.div_ceil(FEE_RATE_STEP_SAT_PER_KVB) * FEE_RATE_STEP_SAT_PER_KVB` |
| UI display (sat/vB, 1 decimal) | `(satPerKvb / 1000).toFixed(1)` — display only, never parsed back |

**Golden vectors** (must pass identically in Rust and TS tests — keep both test files in sync):

| Input | Operation | Expected |
|-------|-----------|----------|
| `1_000` sat/kvB, 350 vB | absolute fee | `350` sats |
| `1_100` sat/kvB, 350 vB | absolute fee | `385` sats |
| `1_001` sat/kvB, 350 vB | absolute fee | `351` sats (ceil) |
| `1_000` sat/kvB, +20% | margin | `1_200` |
| `1_000` sat/kvB, +5%, round to step | margin+round | `1_100` (1_050 → step-up) |
| `2_530` sat/kvB | → FeeRate sat/kwu | `633` (`2530/4=632.5` → ceil) |
| `10_000_001` sat/kvB | validate | `Err(AboveMax)` |
| `900` sat/kvB, min relay `1_000` | validate | `Err(BelowMinRelay)` |

### 2. Rust domain — `desktop-app/src-tauri/src/domain/fee_rate.rs` (new)

Single responsibility: *validated fee-rate value type and its pure arithmetic*.

```rust
/// A validated fee rate in satoshis per virtual kilobyte (sat/kvB).
/// 1 sat/vB == 1_000 sat/kvB; the UI step of 0.1 sat/vB == 100 sat/kvB.
/// Invariant: 0 < value <= MAX_FEE_RATE_SAT_PER_KVB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeeRate(u64);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FeeRateError {
    #[error("fee rate {given} sat/kvB is below the minimum relay fee {min_relay} sat/kvB")]
    BelowMinRelay { given: u64, min_relay: u64 },
    #[error("fee rate {given} sat/kvB exceeds the maximum {max} sat/kvB (10,000 sat/vB)")]
    AboveMax { given: u64, max: u64 },
    #[error("fee rate must be greater than zero")]
    Zero,
}

impl FeeRate {
    /// Validate a raw sat/kvB value against [min_relay, MAX].
    pub fn new(sat_per_kvb: u64, min_relay_sat_per_kvb: u64) -> Result<Self, FeeRateError>;

    pub fn sat_per_kvb(self) -> u64;

    /// Absolute fee in sats for a transaction of `vbytes` virtual bytes (ceil).
    pub fn fee_sats(self, vbytes: u64) -> u64;

    /// Add a percentage margin, rounding up (used by preset derivation).
    /// Saturates at MAX_FEE_RATE_SAT_PER_KVB instead of overflowing.
    pub fn with_margin_pct(self, pct: u64) -> Self;

    /// Round up to the next 0.1 sat/vB UI step.
    pub fn round_up_to_step(self) -> Self;

    /// Convert to BDK's FeeRate (sat/kwu, ceil) for TxBuilder::fee_rate.
    pub fn to_bdk(self) -> bdk_wallet::bitcoin::FeeRate;
}
```

Notes:

- **Name clash**: `bdk_wallet::bitcoin::FeeRate` exists. Convention: import the domain type
  plainly (`use crate::domain::fee_rate::FeeRate;`) and always refer to the BDK type by full
  path (`bdk_wallet::bitcoin::FeeRate`) — the codebase already writes the BDK type fully
  qualified (`wallet_service.rs:494`), so only the new domain import is ever bare.
- `new` enforces both bounds; `with_margin_pct`/`round_up_to_step` are infallible (clamp at MAX).
- No `Default`: a fee rate must always be chosen or estimated, never silently defaulted in domain
  code. (The current `FeeRate::from_sat_per_vb(..).unwrap_or(BROADCAST_MIN)` silent fallback in
  `wallet_service.rs` is removed by this spec — see §6.)
- `domain/fee_constants.rs` gains one constant (display-only, see §5.4):

```rust
/// Conservative vsize estimate for the BDK commit tx (1 P2TR keyspend input + commit
/// P2TR output + P2TR change). Display-only — BDK computes the real fee when building.
pub const COMMIT_TX_VBYTES_ESTIMATE: u64 = 160;
```

### 3. Fee estimation port & service

#### 3.1 Port — `desktop-app/src-tauri/src/application/fee_estimation.rs` (new)

Single responsibility: *fee-estimation contract and preset derivation policy*.
The trait and the types it speaks live together in this module; infrastructure implements it
(dependency direction: infrastructure → application abstractions, same as `CommitFunding`).

```rust
/// Confirmation targets fixed by the functional spec §3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationTarget {
    /// Next block.
    Fast,   // -> 1 block
    /// ~6 blocks (default preset).
    Medium, // -> 6 blocks
    /// ~12 blocks.
    Slow,   // -> 12 blocks
}

impl ConfirmationTarget {
    pub fn blocks(self) -> u16 { /* 1 | 6 | 12 */ }
    /// Security margin (functional spec §3.2): Fast +20%, Medium +10%, Slow +5%.
    pub fn margin_pct(self) -> u64 { /* 20 | 10 | 5 */ }
}

#[derive(Debug, thiserror::Error)]
pub enum FeeEstimateError {
    #[error("fee source unavailable: {0}")]
    Unavailable(String),
    #[error("fee source returned no estimate for target {target} blocks: {reason}")]
    NoEstimate { target: u16, reason: String },
}

/// One fee source (node RPC or Electrum). Returns RAW rates — no margin applied.
#[async_trait]
pub trait FeeEstimator: Send + Sync {
    /// Raw estimate in sat/kvB for the given target.
    async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError>;
    /// Minimum relay fee in sat/kvB.
    async fn min_relay_sat_per_kvb(&self) -> Result<u64, FeeEstimateError>;
}
```

#### 3.2 Service — same module

```rust
/// Where the presets came from — surfaced verbatim to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeSource {
    /// Bitcoin node estimatesmartfee.
    Node,
    /// Electrum blockchain.estimatefee (M2).
    Electrum,
    /// Previous successful estimate, re-served because live sources failed (M2).
    Cached,
    /// Static fallback: presets derived from the min relay fee. Signer must review.
    Fallback,
}

#[derive(Debug, Clone, Copy)]
pub struct FeePreset {
    pub rate: FeeRate,        // margin applied, rounded up to step, clamped
    pub target_blocks: u16,
    pub margin_pct: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct FeePresets {
    pub fast: FeePreset,
    pub medium: FeePreset,
    pub slow: FeePreset,
    pub min_relay_sat_per_kvb: u64,
    pub source: FeeSource,
    /// Unix ms when the underlying estimate was taken (cache age display, M2).
    pub estimated_at_ms: u64,
}

/// Orchestrates sources in priority order and derives presets.
pub struct FeeEstimationService { /* estimators in priority order + cache (M2) */ }

impl FeeEstimationService {
    pub fn new(estimators: Vec<Arc<dyn FeeEstimator>>) -> Self;
    pub async fn presets(&self) -> FeePresets;  // infallible — see fallback policy
}
```

**Preset derivation algorithm** (pure function, unit-tested in isolation):

```
for each source in [node, electrum (M2)]:
    min_relay = source.min_relay_sat_per_kvb()  (on error → FALLBACK_MIN_RELAY_SAT_PER_KVB)
    rates = source.estimate(1), source.estimate(6), source.estimate(12)
    if all three succeed:
        preset(t) = clamp(round_up_to_step(rate(t) * (1 + margin(t))), min_relay, MAX)
        enforce monotonicity: slow <= medium <= fast (raise lower ones to match if the
            source returns inverted estimates — estimatesmartfee can plateau)
        cache and return (source = Node | Electrum)
if cache has an entry younger than CACHE_MAX_AGE (10 min)  (M2):
    return it with source = Cached
else:
    # static fallback: derive presets from the min relay fee so broadcast stays possible
    base = last known min_relay or FALLBACK_MIN_RELAY_SAT_PER_KVB
    preset(t) = round_up_to_step(base * (1 + margin(t)))
    return with source = Fallback
```

Why infallible: blocking governance broadcast because fee estimation is down would violate the
project's offline-survivability rule. Instead the UI labels `Fallback` rates loudly (§7) and the
signer can always enter a Custom rate. On regtest `estimatesmartfee` always fails (insufficient
data), so regtest flows deterministically get `Fallback` presets of 1.2 / 1.1 / 1.1 sat/vB —
equivalent to today's hardcoded 1 sat/vB behavior, keeping existing e2e flows green.

**All-targets-or-nothing:** a source is used only if all three targets succeed. Mixing sources
per-target would produce incoherent preset ladders.

#### 3.3 Node estimator — `desktop-app/src-tauri/src/infrastructure/node_fee_estimator.rs` (new)

Single responsibility: *`FeeEstimator` over the Bitcoin node JSON-RPC*.

Wraps `&dyn BitcoinRpcClient`. Requires two trait changes in
`infrastructure/bitcoin_rpc.rs`:

1. **New method** `estimate_smart_fee_sat_per_kvb(&self, target_blocks: u16) -> Result<u64, String>`
   — strict version: if the `estimatesmartfee` result has an `errors` array or no `feerate`
   field, return `Err` with the node's message (do **not** swallow into a default).
2. **New method** `min_relay_sat_per_kvb(&self) -> Result<u64, String>` — returns
   `max(getnetworkinfo.relayfee, getmempoolinfo.mempoolminfee)` converted to sat/kvB (ceil).
   `mempoolminfee` matters during congestion, `relayfee` is the static floor; Core enforces
   the max of both.
3. **Remove** the legacy `estimate_fee_rate_sats_per_vb` (and its silent
   `unwrap_or(0.00001)`) once all call sites are migrated (§6). No production caller may keep
   depending on the swallow-errors behavior.

#### 3.4 Electrum estimator (M2) — `desktop-app/src-tauri/src/infrastructure/electrum_fee_estimator.rs` (new)

Single responsibility: *`FeeEstimator` over the Electrum protocol*.

- Uses `bdk_electrum::electrum_client::{Client, ElectrumApi}` — already a dependency
  (`bdk_electrum = "0.21"` workspace pin); URL from `NodeConfig::electrum_url()` (same source
  as wallet sync, including the R2.3 custom URL).
- `ElectrumApi::estimate_fee(target) -> Result<f64>` returns BTC/kB; `-1.0` means
  "no estimate available" → `FeeEstimateError::NoEstimate`.
- `ElectrumApi::relay_fee() -> Result<f64>` (BTC/kB) for `min_relay_sat_per_kvb`.
- The electrum client is blocking I/O — every call goes through `tokio::task::spawn_blocking`,
  mirroring the established pattern in `wallet_service.rs::sync` (`wallet_service.rs:442`).
- Connection per call (the sync path does the same); no shared client state.

### 4. Tauri command — `desktop-app/src-tauri/src/commands/fee_rates.rs` (new)

Single responsibility: *IPC boundary for fee estimation (DTO mapping only — no policy)*.

```rust
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeePresetDto {
    pub sat_per_kvb: u64,
    pub target_blocks: u16,
    pub margin_pct: u64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeRatesDto {
    pub fast: FeePresetDto,
    pub medium: FeePresetDto,
    pub slow: FeePresetDto,
    pub min_relay_sat_per_kvb: u64,
    pub max_sat_per_kvb: u64,          // MAX_FEE_RATE_SAT_PER_KVB — UI never hardcodes it
    pub source: String,                // "node" | "electrum" | "cached" | "fallback"
    pub estimated_at_ms: u64,
    // vsize/dust facts so the UI can recompute amounts locally with zero round-trips:
    pub reveal_vbytes: u64,            // REVEAL_TX_VBYTES (350)
    pub commit_vbytes_estimate: u64,   // COMMIT_TX_VBYTES_ESTIMATE (160)
    pub commit_dust_sats: u64,         // COMMIT_DUST_SATS (1500)
}

/// Estimate fee presets for governance broadcast (and, later, wallet Send).
/// Read-only: requires neither an active wallet session nor signing capability.
#[tauri::command]
pub async fn fee_rates_estimate(
    node_config: tauri::State<'_, NodeConfigState>,
) -> Result<FeeRatesDto, String>;
```

- Builds `HttpBitcoinRpcClient` from `NodeConfig` (same pattern as
  `proposals_resolve_broadcast_status`), wraps it in `NodeFeeEstimator`, adds
  `ElectrumFeeEstimator` in M2, calls `FeeEstimationService::presets()`.
- Registered in `commands/invoke.rs` alongside the existing proposal commands.
- Deliberately session-free so the UI can show estimates while the signer reviews, before any
  signing-capability gate.

### 5. Plumbing the selected rate through broadcast

#### 5.1 IPC inputs

`commands/proposals.rs`:

```rust
pub struct BroadcastInput {
    pub base_url: String,
    pub action_id: String,
    /// Selected fee rate in sat/kvB. Required for `proposals_broadcast`;
    /// optional for `proposals_prepare_broadcast` (None -> Medium preset).
    pub fee_rate_sat_per_kvb: Option<u64>,
}
```

`BroadcastManualInput` gains the same field. Backward compatibility is not needed — frontend and
Tauri backend ship together — but `Option` on prepare keeps the initial screen load (which
happens before presets resolve) a single round-trip.

**Validation at the command boundary** (`proposals_broadcast`, `proposals_broadcast_manual`):

```rust
let min_relay = /* node min relay; on error fall back to FALLBACK_MIN_RELAY_SAT_PER_KVB */;
let fee_rate = FeeRate::new(input.fee_rate_sat_per_kvb.ok_or("fee rate is required")?, min_relay)
    .map_err(|e| /* map to BroadcastError::FeeRateOutOfRange */)?;
```

`BroadcastError` (application/proposals.rs) gains:

```rust
#[error("fee rate out of range: {0}")]
FeeRateOutOfRange(#[from] crate::domain::fee_rate::FeeRateError),
```

and `map_broadcast_error` maps it to a string the frontend classifies (see §7.3).

#### 5.2 Application layer signature changes

```rust
// application/proposals.rs — fee estimation REMOVED from inside these functions:

/// Replaces the current `(String, u64, u64)` tuple return of the prepare functions.
pub struct PreparedBroadcast {
    pub commit_address: String,
    pub commit_amount_sats: u64,
    /// Total estimated network fee: reveal fee + commit-vsize estimate (display).
    pub estimated_fee_sats: u64,
    pub fee_rate: FeeRate,
}

pub async fn prepare_broadcast_local(..., fee_rate: FeeRate) -> Result<PreparedBroadcast, BroadcastError>;
pub async fn prepare_broadcast_manual(..., fee_rate: FeeRate) -> Result<PreparedBroadcast, BroadcastError>;
pub async fn submit_commit_then_reveal(..., fee_rate: FeeRate, ...) -> ...;
pub async fn broadcast_manual(..., fee_rate: FeeRate, ...) -> ...;
```

- `reveal_fee_sats = fee_rate.fee_sats(REVEAL_TX_VBYTES)`
- `commit_amount_sats = COMMIT_DUST_SATS + reveal_fee_sats`
- `commit_funding.build_signed_commit(addr, amount, fee_rate)` — trait signature changes from
  `fee_rate: u64` (sat/vB) to `fee_rate: FeeRate`.

This removes today's **double estimation** (prepare estimates once for display, broadcast
re-estimates and may silently use a different rate than the one shown). After this spec, the rate
shown is by construction the rate used: prepare/broadcast never read a fee source — only commands
do, and only via the user's selection.

#### 5.3 Wallet service

`wallet_service.rs::build_signed_commit` takes `FeeRate` and calls
`tx_builder.fee_rate(fee_rate.to_bdk())`. The
`FeeRate::from_sat_per_vb(..).unwrap_or(BROADCAST_MIN)` silent fallback is deleted — invalid
rates can no longer reach this layer (validated at the command boundary).

#### 5.4 Prepare DTO

```rust
#[serde(rename_all = "camelCase")]
pub struct PrepareBroadcastDto {
    pub action_id: String,
    pub commit_address: String,
    pub commit_amount_sats: u64,
    pub estimated_fee_sats: u64,      // reveal fee + commit fee estimate (display)
    pub fee_rate_sat_per_kvb: u64,    // the rate these numbers were computed with
}
```

`estimated_fee_sats` changes meaning from "reveal fee only" (today) to "total estimated network
fee" = `fee_rate.fee_sats(REVEAL_TX_VBYTES) + fee_rate.fee_sats(COMMIT_TX_VBYTES_ESTIMATE)`,
matching functional spec §4 ("estimated total network fee"). The commit component is an estimate
(BDK picks real inputs at signing time); the UI labels it "Estimated".

When the signer changes preset/custom **after** prepare, the UI recomputes locally using the
vsize facts from `FeeRatesDto` (same golden-vector formulas) — no prepare re-fetch. The
authoritative numbers are recomputed by the backend from `fee_rate_sat_per_kvb` at broadcast
time; the local recompute is display-only and uses identical integer math, so they agree.

### 6. RBF (BIP-125)

Already satisfied by the implementation; this spec locks it with regression tests:

- **Reveal**: `broadcast_tx.rs::build_reveal_tx` sets `Sequence::ENABLE_RBF_NO_LOCKTIME`
  (`broadcast_tx.rs:205`). Add an explicit assertion test.
- **Commit**: `bdk_wallet` 1.x defaults `TxBuilder` input sequences to
  `0xFFFFFFFD` (RBF-signaling). Add a regression test asserting every input of a built commit
  has `sequence.is_rbf()`, so a future BDK upgrade or a stray `set_exact_sequence` cannot
  silently disable RBF. Test mechanics: `WalletService::build_signed_commit` calls `sync()`
  (live Electrum) and cannot run in unit tests — make `build_and_sign_tx` `pub(crate)` and test
  it directly against a regtest wallet funded with a synthetic confirmed UTXO (insert a fake
  funding tx via BDK's test utilities / `Wallet::apply_update`-style insertion in a
  `#[cfg(test)]` helper).

No production code change expected; if the commit test fails, fix by setting
`tx_builder.set_exact_sequence(Sequence::ENABLE_RBF_NO_LOCKTIME)`.

### 7. Frontend

#### 7.1 New feature context `desktop-app/src/domain/fee-selection/`

```
domain/fee-selection/
├── model/fee-rate.ts          # pure: types, conversion/validation/format helpers
├── hooks/use-fee-presets.ts   # stateful: load presets via IPC, expose selection state
└── components/
    └── fee-rate-selector.tsx  # presentational: segmented control + custom entry
```

**`model/fee-rate.ts`** — single responsibility: *pure fee-rate math and validation for the UI*.

```ts
export type FeePresetId = 'slow' | 'medium' | 'fast' | 'custom'

export type FeePreset = { satPerKvb: number; targetBlocks: number; marginPct: number }

export type FeeRates = {
	fast: FeePreset
	medium: FeePreset
	slow: FeePreset
	minRelaySatPerKvb: number
	maxSatPerKvb: number
	source: 'node' | 'electrum' | 'cached' | 'fallback'
	estimatedAtMs: number
	revealVbytes: number
	commitVbytesEstimate: number
	commitDustSats: number
}

/** Selected rate state. `custom` keeps the raw input string for controlled editing. */
export type FeeSelection =
	| { kind: 'preset'; preset: Exclude<FeePresetId, 'custom'> }
	| { kind: 'custom'; inputSatPerVb: string }

export function selectedRateSatPerKvb(selection: FeeSelection, rates: FeeRates): number | null
// preset -> rates[preset].satPerKvb; custom -> parseCustomRate(...) ?? null

/** Strict parse of the custom field: /^\d{1,5}(\.\d)?$/, bounds-checked. */
export function parseCustomRate(
	input: string,
	minRelaySatPerKvb: number,
	maxSatPerKvb: number,
): { ok: true; satPerKvb: number } | { ok: false; reason: 'invalid' | 'below-min' | 'above-max' }

/** Integer math mirroring Rust (golden vectors): ceil(satPerKvb * vbytes / 1000). */
export function feeSatsFor(satPerKvb: number, vbytes: number): number

export function estimatedTotalFeeSats(satPerKvb: number, rates: FeeRates): number
// feeSatsFor(rate, revealVbytes) + feeSatsFor(rate, commitVbytesEstimate)

export function commitAmountSats(satPerKvb: number, rates: FeeRates): number
// commitDustSats + feeSatsFor(rate, revealVbytes)

export function formatSatPerVb(satPerKvb: number): string  // '1.1' — display only
```

`parseCustomRate` parses the decimal **textually** (split on `.`, scale to integers) — never
`parseFloat` into money math. JS integer math is safe here: max values
(`10_000_000 sat/kvB × 350`) are far below `Number.MAX_SAFE_INTEGER`.

**`hooks/use-fee-presets.ts`** — single responsibility: *load/refresh presets and own the
selection state machine*.

```ts
export type UseFeePresetsReturn = {
	rates: FeeRates | null              // null while loading or on hard failure
	isLoading: boolean
	loadError: string | null            // IPC-level failure (rare: command itself errored)
	selection: FeeSelection             // default { kind: 'preset', preset: 'medium' }
	select: (selection: FeeSelection) => void
	/** Resolved rate for the current selection; null when custom input is invalid. */
	rateSatPerKvb: number | null
	refresh: () => Promise<void>
}

export function useFeePresets(): UseFeePresetsReturn
```

- Fetches once on mount (the command is infallible by design; `loadError` only covers IPC
  transport failures, where the UI falls back to custom-only entry with min-relay default).
- Switching to `custom` seeds `inputSatPerVb` with the **current Medium preset** formatted to one
  decimal (functional spec §4 "default starting value").
- Switching back to a preset preserves the custom string (so the user can toggle to compare).

**`components/fee-rate-selector.tsx`** — single responsibility: *render selection UI from props
and emit intent*. No IPC, no business rules (per react-frontend-patterns).

```ts
type Props = {
	rates: FeeRates | null
	isLoading: boolean
	selection: FeeSelection
	onSelect: (selection: FeeSelection) => void
	rateSatPerKvb: number | null
	disabled?: boolean        // true while broadcasting / awaiting device
}
```

Layout (per functional spec §7, Alta `FeeRateInput` pattern):

- Segmented control `Slow / Medium / Fast` (left) + gear/settings toggle for Custom (right).
  `role="radiogroup"`, each option `role="radio"` + `aria-checked`; gear is `aria-pressed`.
- Description line under the control:
  - preset: `"~{targetBlocks} blocks · {formatSatPerVb(rate)} sat/vB"`
    (Fast: `"Next block · … sat/vB"`)
  - custom: numeric input (`inputMode="decimal"`, `step 0.1` semantics enforced by
    `parseCustomRate`), suffix `sat/vB`, helper line
    `"Min {formatSatPerVb(minRelaySatPerKvb)} — Max 10,000 sat/vB"`, inline error from
    `parseCustomRate.reason`.
- Estimated fee row: `"Estimated network fee: {estimatedTotalFeeSats(...).toLocaleString()} sats"`.
- Source banner when `source !== 'node'`:
  - `electrum`: muted info — `"Fee estimates from Electrum (node unavailable)"`.
  - `cached`: muted info with age — `"Using last known estimates ({n}m old)"`.
  - `fallback`: amber warning — `"Live fee estimates unavailable — using minimum-relay-based
    defaults. Verify the rate before broadcasting."`
- `data-testid`s: `e2e-fee-preset-slow|medium|fast`, `e2e-fee-custom-toggle`,
  `e2e-fee-custom-input`, `e2e-fee-estimated-total`, `e2e-fee-source-banner`.

#### 7.2 Integration — governance broadcast (and cancel, for free)

- `broadcast-proposal-screen.tsx` calls `useFeePresets()` and passes `{rates, isLoading,
  selection, onSelect, rateSatPerKvb}` into `BroadcastDetailsCard`, which renders
  `<FeeRateSelector/>` between the "Reveal TX" section and the (now removed) static
  "Estimated fee" row — the selector's estimated-fee row replaces it. The displayed
  commit amount switches to the local recompute `commitAmountSats(rate, rates)` once a rate
  is resolved (falls back to `bundle.commitAmountSats` until then).
- `useBroadcastProposal` accepts the rate at call time:

```ts
broadcast: (feeRateSatPerKvb: number) => Promise<void>
```

  and threads it into `broadcastProposal({ baseUrl, actionId, feeRateSatPerKvb })`.
  `prepare()` keeps sending no rate (backend defaults to Medium).
- The Broadcast button is additionally disabled when `rateSatPerKvb === null`
  (invalid/empty custom input) — invalid rates cannot even be submitted.
- `cancel-proposal-broadcast` reuses `useBroadcastProposal` + `BroadcastDetailsCard`, so it
  inherits the selector with **zero cancel-specific changes** beyond passing the new props
  through `cancel-details-card.tsx` if it duplicates the card (verify at implementation time;
  follow whichever composition exists).
- Manual flow: `manual-sign-collect.tsx` renders the same `<FeeRateSelector/>` and passes
  `feeRateSatPerKvb` into `prepareBroadcastManual`/`broadcastManual` inputs.

#### 7.3 API adapters & schemas

- `api/fee-rates.ts` (new): `estimateFeeRates(): Promise<ApiResult<FeeRates>>` via
  `tauriCall('fee_rates_estimate', {}, feeRatesSchema)`.
- `api/ipc-schemas.ts`: `feeRatesSchema` (zod) validating the full `FeeRatesDto` shape,
  including `source` as a literal union; `prepareBroadcastResultSchema` gains
  `feeRateSatPerKvb`.
- `api/proposals.ts`: `BroadcastInput`/`BroadcastManualInput` gain `feeRateSatPerKvb`.
- `domain/broadcast-proposal/model/broadcast-proposal.ts`: new error code
  `fee_rate_out_of_range` (matched on the `"fee rate out of range"` message prefix) with
  recovery `'retry'`; `deriveBroadcastError` keeps its existing prefix-matching style.

### 8. Broadcast path — Electrum first, node fallback, manual escape (M3)

#### 8.1 Port — `desktop-app/src-tauri/src/application/tx_broadcaster.rs` (new)

Single responsibility: *contract for submitting the signed commit+reveal pair to the network*.

```rust
#[derive(Debug, thiserror::Error)]
pub enum TxBroadcastError {
    #[error("{source_name} rejected the transaction: {message}")]
    Rejected { source_name: &'static str, message: String },
    #[error("{source_name} unavailable: {message}")]
    Unavailable { source_name: &'static str, message: String },
}

#[async_trait]
pub trait TxBroadcaster: Send + Sync {
    fn name(&self) -> &'static str;
    /// Submit commit then reveal. Implementations MUST treat "already in mempool/known"
    /// responses as success (idempotent re-submission after a partial earlier attempt).
    async fn broadcast_pair(&self, commit_hex: &str, reveal_hex: &str)
        -> Result<(), TxBroadcastError>;
}
```

Implementations:

- `infrastructure/electrum_broadcaster.rs` (new) — `ElectrumApi::transaction_broadcast` for
  commit, then reveal, via `spawn_blocking`. The reveal spends the unconfirmed commit; electrs
  forwards to bitcoind, which accepts in-mempool chained spends, so sequential submission is
  correct. Idempotency: treat error strings containing `"already"`/`"duplicate"` (electrs relays
  bitcoind's `txn-already-in-mempool` / `txn-already-known`) as success.
- `infrastructure/bitcoin_rpc.rs` — existing node path extracted into `NodeBroadcaster`
  (new struct in the same file or `node_broadcaster.rs`): `submit_package` first,
  sequential `send_raw_transaction` fallback when the method is unknown (preserving
  `is_unknown_method`, which moves here from `application/proposals.rs`). Same idempotency rule.

#### 8.2 Orchestration

`submit_commit_then_reveal` Step 6 becomes:

```
for broadcaster in [electrum (M3), node]:
    match broadcaster.broadcast_pair(...):
        Ok -> proceed to step 7 (report progress)
        Err(e) -> log with tracing::warn!(broadcaster = name, %e); continue
all failed -> Err(BroadcastError::AllBroadcastersFailed {
    commit_tx_hex, reveal_tx_hex, errors: Vec<(name, message)>
})
```

- A **rejection** (consensus/policy error) from a source still falls through to the next source:
  electrs policy can differ from the node's, and the node is authoritative. Only when all
  sources have failed does the user see the aggregate error.
- `PendingReveals` is already inserted before any broadcast attempt (Step 5) — unchanged, so
  resubmit/reconcile keep working regardless of which source succeeded.
- The orchestrator `failed` report on genuine submission errors is unchanged.

#### 8.3 Manual escape hatch (UI)

`AllBroadcastersFailed` serializes (via `map_broadcast_error`) to a structured message the
frontend parses into a new `BroadcastError` code `broadcast_unavailable` with recovery
`'manual-broadcast'`. To keep hexes out of the human-readable message, `proposals_broadcast`
returns them in the typed error path: change the command's error type from `String` to a
serde-serializable `BroadcastFailureDto { code, message, commitTxHex?, revealTxHex? }` (Tauri
supports any `Serialize` error type). **Implementation note:** verify how
`api/tauri-bridge.ts::tauriCall` materializes command errors today — if it assumes string
errors, extend it (backward-compatibly) to pass structured payloads through to
`deriveBroadcastError`; do this as the first step of M3.

UI: when `recovery === 'manual-broadcast'`, `BroadcastPhaseProgress` renders a
"Broadcast manually" panel: explanation line, commit hex + `CopyButton`, reveal hex +
`CopyButton`, note "Broadcast the commit first, then the reveal, via any Bitcoin node
(`sendrawtransaction`)." plus the existing Retry button. The signed hexes are
not secrets (they are destined for the public mempool) — exposing them is safe and is exactly
the manual-fallback escape the project conventions require.

### 9. Logging

Per rust-specialist standards, new modules use `tracing` (`warn!` for source fallbacks,
`info!` for chosen source) — not `eprintln!`. Existing `eprintln!` call sites in touched
functions are left as-is (separate cleanup), but no new ones are added. Never log mnemonics,
xprvs, or PSBTs; fee rates and txids are fine.

### Production code vs. test helpers

**Production (exposed):**

- Tauri commands: `fee_rates_estimate` (new), `proposals_prepare_broadcast`,
  `proposals_broadcast`, `proposals_prepare_broadcast_manual`, `proposals_broadcast_manual`
  (modified inputs/outputs).
- Library API (src-tauri crate): `domain/fee_rate.rs`, `application/fee_estimation.rs`,
  `application/tx_broadcaster.rs` and their infrastructure implementations.
- Frontend: everything under `domain/fee-selection/`, `api/fee-rates.ts`.

**Test helpers (never registered as commands, never in production paths):**

- Mock `FeeEstimator`s (scripted success/failure sequences) — `#[cfg(test)]` in
  `application/fee_estimation.rs`.
- Mock `TxBroadcaster` — `#[cfg(test)]` in `application/tx_broadcaster.rs`.
- The existing mock `BitcoinRpcClient`/`OrchestratorClient` test doubles in
  `application/proposals.rs` tests gain the new methods.
- Funded-regtest-wallet fixture for the commit RBF test (reuse the existing
  `make_wallet_service`-style helpers in `commit_funding.rs` tests).
- Frontend: `FeeRates` fixture builders in `domain/fee-selection/model/__tests__/fixtures.ts`.

---

## Test Cases

Tests target production functions only. Naming: `test_<unit>_<behavior>`.

### Rust — `domain/fee_rate.rs` (pure unit)

1. **Golden vectors** — every row of the table in §1 (conversions, ceil rounding, margins,
   step round-up, BDK kwu conversion).
2. `new` happy path: `1_000` with min relay `1_000` → `Ok`.
3. `new` rejects: `0` → `Zero`; `999` vs min relay `1_000` → `BelowMinRelay`;
   `10_000_100` → `AboveMax`; boundary `10_000_000` → `Ok`.
4. `with_margin_pct` saturates at MAX instead of overflowing (`u64::MAX`-adjacent input).
5. `fee_sats` exactness: rate `100` (0.1 sat/vB) × 350 vB → `35` sats.
6. `to_bdk` round-trip: resulting `FeeRate::to_sat_per_kwu` ≥ `sat_per_kvb / 4` (never
   underpays).

### Rust — `application/fee_estimation.rs` (service with mock estimators)

7. Node succeeds on all targets → presets = margin-applied, step-rounded, clamped; `source ==
   Node`; margins are exactly +20/+10/+5.
8. Monotonicity enforcement: node returns slow > medium (plateau inversion) → derived presets
   satisfy `slow <= medium <= fast`.
9. Preset below min relay after margin → clamped up to min relay.
10. Preset above MAX → clamped to MAX.
11. Node fails one target (of three) → node skipped entirely → next source (all-or-nothing).
12. Node fails, Electrum succeeds → `source == Electrum` (M2).
13. Both fail, fresh cache exists → cached presets, `source == Cached` (M2).
14. Both fail, cache stale/empty → static fallback from min relay; `source == Fallback`;
    with min relay 1_000: fast `1_200`, medium `1_100`, slow `1_100` (after step round-up).
15. Min relay endpoint fails but estimates succeed → presets still produced, min relay =
    `FALLBACK_MIN_RELAY_SAT_PER_KVB`.
16. `presets()` never returns `Err` (type-level: signature returns `FeePresets`, test asserts
    fallback path completes).

### Rust — `infrastructure/bitcoin_rpc.rs` / `node_fee_estimator.rs`

17. `estimate_smart_fee_sat_per_kvb` parses `{"feerate": 0.00001}` → `1_000`.
18. `estimatesmartfee` response with `errors: [...]` → `Err` containing the node message
    (regression against the old swallow-into-default behavior).
19. Response missing `feerate` → `Err`.
20. `min_relay_sat_per_kvb` = max(relayfee, mempoolminfee), ceil conversion.
21. (M2) Electrum `estimate_fee` returning `-1` → `NoEstimate`; BTC/kB conversion vector.

### Rust — `application/proposals.rs` (mocked orchestrator/RPC/funding)

22. `submit_commit_then_reveal` uses the **provided** `FeeRate` — mock `CommitFunding` records
    the rate; mock RPC asserts `estimatesmartfee` is **never called** during broadcast.
23. Reveal fee math: rate `1_100` → `commit_amount_sats == 1_500 + 385`.
24. `prepare_broadcast_local` with explicit rate returns
    `estimated_fee_sats == fee(350) + fee(160)` and echoes `fee_rate_sat_per_kvb`.
25. (M3) Electrum broadcaster succeeds → node never called.
26. (M3) Electrum unavailable → node fallback succeeds → `Ok`, progress reported.
27. (M3) Electrum broadcasts commit then fails reveal; node retry gets
    "txn-already-in-mempool" for commit → treated as success, reveal proceeds.
28. (M3) All broadcasters fail → `AllBroadcastersFailed` carries both tx hexes and per-source
    errors; orchestrator received a `failed` report; `PendingReveals` entry retained.

### Rust — commands (`commands/proposals.rs`, `commands/fee_rates.rs`)

29. `proposals_broadcast` with `fee_rate_sat_per_kvb: None` → error mentioning fee rate
    required.
30. `proposals_broadcast` with out-of-range rate → `FeeRateOutOfRange` mapped error; no
    broadcast attempted (mock funding never called).
31. `fee_rates_estimate` DTO shape: camelCase fields, `max_sat_per_kvb == 10_000_000`,
    vsize/dust facts present (serde serialization test).

### Rust — RBF regression

32. `build_reveal_tx` output input[0] `sequence.is_rbf()` (explicit BIP-125 assertion).
33. `build_signed_commit` on a funded regtest wallet: every input `sequence.is_rbf()`.

### Frontend — `model/fee-rate.ts` (vitest, pure)

34. Golden vectors (same table as Rust test 1 — keep in sync).
35. `parseCustomRate`: `'1'` → 1_000; `'1.1'` → 1_100; `'0.1'` with minRelay 100 → ok;
    `'1.15'` → invalid (one decimal max); `'abc'`, `''`, `'-1'`, `'1.'` → invalid;
    `'0.9'` vs minRelay 1_000 → `below-min`; `'10000.1'` → `above-max`; `'10000'` → ok.
36. `selectedRateSatPerKvb`: preset resolves from rates; custom resolves via parse; invalid
    custom → null.
37. `estimatedTotalFeeSats` / `commitAmountSats` match backend formulas (vectors).
38. `formatSatPerVb(1_100)` → `'1.1'`; `formatSatPerVb(1_000)` → `'1.0'`.

### Frontend — `use-fee-presets.ts` (vitest, mocked IPC)

39. Loads presets on mount; default selection is Medium; `rateSatPerKvb` = medium rate.
40. Selecting custom seeds input with Medium formatted (`'1.1'` for 1_100).
41. Invalid custom input → `rateSatPerKvb === null`.
42. IPC transport failure → `loadError` set, `rates === null`.

### Frontend — `fee-rate-selector.tsx` (vitest + testing-library)

43. Renders three presets + custom toggle; Medium checked by default (`aria-checked`).
44. Fallback source → warning banner rendered (`e2e-fee-source-banner`).
45. Custom invalid input shows the bound-specific error message and no estimated-fee total.
46. `disabled` prop disables all controls.

### Frontend — integration (`use-broadcast-proposal`, `broadcast-details-card`)

47. `broadcast(rate)` passes `feeRateSatPerKvb` through to the IPC input (mock `tauriCall`).
48. Broadcast button disabled when `rateSatPerKvb === null`.
49. `fee_rate_out_of_range` backend error derives recovery `'retry'`.
50. (M3) `broadcast_unavailable` error renders manual panel with both hex `CopyButton`s.

### Authority isolation

Fee estimation is authority-agnostic (network-level data) and the broadcast authority gates are
untouched (`load_broadcast_env` session/signing gates, orchestrator claim). Covered by existing
tests; test 30 additionally proves fee validation happens **before** any funding/signing access.

### Offline fallback

- Estimation: tests 11–16 (node down → Electrum → cache → static fallback; UI never blocked).
- Broadcast: tests 25–28 + 50 (Electrum down → node; all down → manual copy-hex).
- Orchestrator down: unchanged — fee selection adds no orchestrator dependency.

### e2e (WebDriver) — additive, existing specs stay green

The default Medium preset means existing broadcast smoke specs require **no changes** (regtest
resolves to Fallback presets ≈ today's 1 sat/vB). Add one optional spec:
select Fast → assert description updates → broadcast → done banner; and one custom-entry spec:
toggle custom, enter `2.5`, assert estimated total updates.

---

## Module structure

### New files (single responsibility each)

| File | Responsibility (one sentence) |
|------|-------------------------------|
| `src-tauri/src/domain/fee_rate.rs` | Validated fee-rate value type (sat/kvB) and its pure arithmetic. |
| `src-tauri/src/application/fee_estimation.rs` | Fee-estimation port (`FeeEstimator`) and the preset-derivation policy (`FeeEstimationService`). |
| `src-tauri/src/infrastructure/node_fee_estimator.rs` | `FeeEstimator` implementation over the Bitcoin node JSON-RPC. |
| `src-tauri/src/infrastructure/electrum_fee_estimator.rs` (M2) | `FeeEstimator` implementation over the Electrum protocol. |
| `src-tauri/src/commands/fee_rates.rs` | IPC boundary mapping `FeePresets` to camelCase DTOs. |
| `src-tauri/src/application/tx_broadcaster.rs` (M3) | Port for submitting the signed commit+reveal pair to the network. |
| `src-tauri/src/infrastructure/electrum_broadcaster.rs` (M3) | `TxBroadcaster` implementation over Electrum `transaction_broadcast`. |
| `src/domain/fee-selection/model/fee-rate.ts` | Pure fee-rate math, parsing, and formatting for the UI. |
| `src/domain/fee-selection/hooks/use-fee-presets.ts` | Preset loading and fee-selection state machine. |
| `src/domain/fee-selection/components/fee-rate-selector.tsx` | Presentational fee-rate selection control. |
| `src/api/fee-rates.ts` | Typed IPC adapter for `fee_rates_estimate`. |

### Modified files

| File | Change |
|------|--------|
| `src-tauri/src/domain/fee_constants.rs` | + `COMMIT_TX_VBYTES_ESTIMATE` |
| `src-tauri/src/infrastructure/bitcoin_rpc.rs` | strict `estimate_smart_fee_sat_per_kvb`, `min_relay_sat_per_kvb`; remove legacy lenient estimator; (M3) extract `NodeBroadcaster` |
| `src-tauri/src/application/proposals.rs` | `FeeRate` parameter on prepare/broadcast fns; remove internal estimation; (M3) broadcaster chain + `AllBroadcastersFailed` |
| `src-tauri/src/application/commit_funding.rs` | trait takes `FeeRate` |
| `src-tauri/src/application/wallet_service.rs` | `build_signed_commit(FeeRate)`; delete silent `BROADCAST_MIN` fallback |
| `src-tauri/src/commands/proposals.rs` | inputs gain `fee_rate_sat_per_kvb`; boundary validation; DTO `fee_rate_sat_per_kvb`; (M3) structured error DTO |
| `src-tauri/src/commands/invoke.rs` + `commands/mod.rs` | register `fee_rates_estimate` |
| `src/api/proposals.ts`, `src/api/ipc-schemas.ts` | new input field, `feeRatesSchema`, prepare-result field |
| `src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts` | `broadcast(feeRateSatPerKvb)` |
| `src/domain/broadcast-proposal/components/broadcast-details-card.tsx` | embed `FeeRateSelector`; local amount recompute |
| `src/domain/broadcast-proposal/model/broadcast-proposal.ts` | new error codes/recoveries |
| `src/screens/broadcast-proposal-screen.tsx` | wire `useFeePresets` |
| `src/domain/manual-proposal/components/manual-sign-collect.tsx` (+ its hook) | selector + rate in manual inputs |

### Dependency direction (verified)

- `domain/fee_rate.rs` depends only on `bdk_wallet::bitcoin` types (for `to_bdk`) — no
  application/infrastructure imports.
- `application/fee_estimation.rs` and `application/tx_broadcaster.rs` define the traits **and**
  the types those traits speak (`FeePresets`, `TxBroadcastError`); infrastructure modules import
  from application, never the reverse — same direction as `CommitFunding`/`WalletService`.
- Frontend: `model/` is import-free (pure), `hooks/` import model + api adapter, `components/`
  import model types only; screens compose all three (react-frontend-patterns).
- The TS model duplicates four integer formulas from Rust by design (zero-round-trip recompute);
  the shared golden-vector tables in both test suites are the drift guard.

## PRD / functional-spec traceability

| Functional spec § | This spec |
|-------------------|-----------|
| §3 presets, Medium default | `ConfirmationTarget`, `FeePresets`, hook default selection (§3.2, §7.1) |
| §3.1 estimate source + Electrum fallback | `NodeFeeEstimator` → `ElectrumFeeEstimator` priority (§3.3–3.4, M2) |
| §3.2 security margin | `margin_pct` 20/10/5 in `ConfirmationTarget` (§3.1) |
| §4 custom: unit, 0.1 step, min relay, 10k max, Medium seed, total fee | `FeeRate` bounds, `parseCustomRate`, custom seed, `estimatedTotalFeeSats` (§1, §2, §7.1) |
| §5 RBF always on | regression tests 32–33 (§6) |
| §6 broadcast path: Electrum → node → manual hex | `TxBroadcaster` chain + manual panel (§8, M3) |
| §8 constraints table | constants in §1–§2; UTXO source unchanged (Electrum-synced BDK wallet) |

## Resolved design decisions (free decisions taken by this spec)

1. **sat/kvB integers** over floats or deci-sat/vB — exact for 0.1 steps, Core-aligned, headroom.
2. **Estimation never blocks broadcast** — static fallback presets + loud source banner instead
   of disabled UI (offline survivability > strictness; Custom always available).
3. **All-targets-or-nothing per source** — no mixed-source preset ladders.
4. **Rate is chosen client-side and validated server-side** — commands never silently substitute
   a different rate than displayed (removes today's double-estimation drift).
5. **`estimated_fee_sats` becomes commit-estimate + reveal fee** — matches "total network fee"
   in the functional spec; commit component labeled as estimate.
6. **Three milestones** — M1 ships the PRD-critical selection UX without waiting for the
   Electrum broadcast work.
7. **TS/Rust formula duplication with golden vectors** instead of a shared WASM/codegen layer —
   four one-line integer formulas do not justify build complexity.
