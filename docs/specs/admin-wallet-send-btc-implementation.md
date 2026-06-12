# Spec: Admin Wallet — Send BTC — Implementation (Phase 6)

**PRD:** [`03-prd-update.md`](../0-prd/03-prd-update.md) §4.3.5 — Send BTC (destination, amount, fee rate, change routing, Confirm gate).
**Functional roadmap:** [`admin-wallet-send-btc.md`](./admin-wallet-send-btc.md) — slices **P6.1 → P6.2 → P6.3 → P6.4**, PRD traceability per slice.
**Plan:** [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) Phase 6.
**Compliance:** [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md) §4.3.5 sub-rows.
**Status:** Ready for implementation — this document is the technical design for all four slices.

This spec is written so that an engineer with no prior context on Phases 1–5 can implement Send end-to-end by following it together with the referenced source files. Every type, command, error variant, validation rule, UI state, and test obligation is specified. Where a slice boundary matters, the section is tagged **[P6.1]**–**[P6.4]**.

---

## 1. System context

### 1.1 What exists today (reused, not rebuilt)

| Capability | Where | Reused for Send as |
|---|---|---|
| Session wallet + signer | `application/wallet_service.rs` — `WalletService { wallet: Arc<Mutex<bdk_wallet::Wallet>>, signer: Option<Arc<dyn PsbtSigner>>, network }` | The send pipeline host: new methods are added to `WalletService` via an `impl` block in a new module (Phase 5 pattern) |
| PSBT signing port (R1.1) | `WalletService::sign_and_finalize_psbt` (`wallet_service.rs:525`) — routes to `MnemonicPsbtSigner` (software) or `HwPsbtSigner` (Ledger on-device), finalizes, extracts the tx | Signing the send PSBT, unchanged |
| Capability guards | `signer().ok_or(ReadOnly)` then `signer.allowed_on(network)` (`wallet_transactions.rs:323-325`) | Identical guard order, before any wallet lock or I/O |
| Fee domain | `domain/fee_rate.rs` — `FeeRate` (sat/kvB, `new(rate, min_relay)`, `to_bdk()`, `fee_sats(vbytes)`), constants `FEE_RATE_STEP_SAT_PER_KVB = 100`, `MAX_FEE_RATE_SAT_PER_KVB = 10_000_000`, `FALLBACK_MIN_RELAY_SAT_PER_KVB = 1_000` | Rate validation and BDK conversion, unchanged |
| Fee presets | `commands/fee_rates.rs` `fee_rates_estimate` IPC → `FeeRatesDto { fast, medium, slow, minRelaySatPerKvb, maxSatPerKvb, source, … }`; `ConfirmationTarget::Fast.blocks() == 1` | The Send default rate (PRD "next block" = **Fast** preset) and the custom-rate bounds |
| Broadcast chain | `application/tx_broadcaster.rs` — `TxBroadcaster` port, `broadcast_single_with_fallback(&[Arc<dyn TxBroadcaster>], tx_hex)`; concrete chain built in `commands/admin_wallet.rs:305-308` (`ElectrumBroadcaster` → `NodeBroadcaster`) | Broadcasting the signed send, identical chain construction |
| Change-index discipline (R1.3) | BDK `TxBuilder` change script — gap-aware internal keychain (see §4.4) | PRD §4.3.5.4 |
| Tagged IPC errors | `serialize_wallet_error` / `serialize_bump_error` → `{ "type": <code>, "message": <text> }` (`commands/admin_wallet.rs:166-168, 246-248`) | Same shape via a new `serialize_send_error` |
| Frontend error union | `api/admin-wallet.ts` `AdminWalletError` discriminated union + `parse-admin-wallet-error.ts` | Extended with the Send variants (§6.2) |
| Panel section routing | `hooks/use-wallet-panel-state.ts` — `WalletPanelSection` already includes `'send'` (URL param `walletSection=send`) | The Send view's open/close state — no new routing |
| Submit state machine pattern | `hooks/use-bump-fee.ts` — `idle → submitting → success | error` | Template for `use-send.ts` |
| Inline form pattern | `components/bump-fee-form.tsx` — stepper, `parseSatPerVb`, disabled Confirm, success card with txid + `CopyButton`, error line, `e2e-*` test ids | Template for the Send form's fee entry and result surfaces |

### 1.2 What is new

- `desktop-app/src-tauri/src/application/wallet_send.rs` — send use-case (validate / estimate / send) on `WalletService`.
- Three IPC commands in `commands/admin_wallet.rs`: `admin_wallet_validate_send_address`, `admin_wallet_estimate_send`, `admin_wallet_send`.
- Frontend: `domain/admin-wallet/components/send-form.tsx` (+ sub-components), `hooks/use-send-form.ts`, `hooks/use-send.ts`, `model/send-validation.ts`, `model/format-send-error.ts`, API adapters in `api/admin-wallet.ts`.
- Architecture **Rule 8** in `domain/admin-wallet/architecture.test.ts` (Send wiring guard).

### 1.3 End-to-end flow

```text
React SendForm (walletSection=send)
  ├─ [debounced 300 ms] admin_wallet_validate_send_address { address }
  │     └─► parse Address<NetworkUnchecked> + require_network(wallet.network)   (no I/O, no lock)
  │
  ├─ [on valid dest + amount + rate change, debounced] admin_wallet_estimate_send { input }
  │     └─► WalletService::estimate_send — TxBuilder dry-run (build only, never signed,
  │          never broadcast) → fee, vsize, change, max  (wallet lock only; no network I/O)
  │
  └─ [Confirm] admin_wallet_send { input }
        0. FeeRate::new(rate, FALLBACK_MIN_RELAY_SAT_PER_KVB)  → InvalidFeeRate
        1. best-effort pre-sync (warn on failure — broadcast layer is the authority;
           sync lives in the IPC command, mirroring admin_wallet_bump_fee, so the
           use-case stays free of network I/O except broadcast and its unit tests
           stay hermetic)
        └─► WalletService::send_to_address
              1. signer present?            else ReadOnly
              2. signer.allowed_on(network)? else SignerNotAllowedOnNetwork
              3. parse + network-check destination → InvalidAddress | WrongNetwork
              4. amount guards               → InvalidAmount (zero w/o drain)
              5. build PSBT (recipient or drain)  → InsufficientFunds | AmountBelowDust | BuildFailed
              6. sign_and_finalize_psbt (PsbtSigner port)  → SignFailed   [nothing broadcast]
              7. broadcast_single_with_fallback (Electrum → node) → BroadcastFailed
              8. return SendResultDto { txid, … }
        2. best-effort post-sync (panel converges: balance drops, pending list shows the send)
```

Secrets never cross IPC. React only ever sees strings/numbers and typed error codes.

---

## 2. Design decisions

| # | Decision | Rationale |
|---|---|---|
| **D1** | New module `application/wallet_send.rs` with `impl WalletService` (not new methods inside `wallet_service.rs`) | Exact precedent: Phase 5 `wallet_transactions.rs`. Keeps `wallet_service.rs` core small; send-specific errors and DTOs live with their use-case |
| **D2** | Three IPC commands: validate / estimate / send | The frontend has no Bitcoin address parser and must not grow one — the backend is the validation authority (roadmap §5). Validation is pure parse (cheap to call per keystroke, debounced); estimation is a local TxBuilder dry-run (provides exact PRD `amount ≤ balance − fee·size` semantics from real coin selection, and powers Max + Insufficient funds); send is the only mutating call |
| **D3** | **Max = drain build**: `drain_wallet()` + `drain_to(dest)`; the resulting recipient value *is* the max amount. Confirming a Max send re-runs the same drain build (`drainWallet: true`), so no change output exists | The PRD formula `amount ≤ balance − fee·size` has its boundary precisely at "spend everything minus fee", which is what a BDK drain computes — no manual vsize arithmetic, no rounding drift between estimate and send. §4.3.5.4 is trivially satisfied (no change) |
| **D4** | Default fee = **Fast** preset (`target_blocks == 1`) | PRD §4.3.5.3: default MUST be the node's "next block" rate. Phase 4's `useFeePresets` defaults to `medium` for governance — Send overrides the initial selection to `fast` |
| **D5** | Exact PRD copy lives in the **frontend** formatter, keyed by typed backend error codes; backend messages are diagnostic | Backend stays copy-free (same convention as `BumpFeeError` + `format-admin-wallet-error.ts`); PRD copy can be audited in one TS file with unit tests asserting the exact strings |
| **D6** | Amount entry in **sats** (integer input) with a BTC equivalent sub-label via existing `format-btc-from-sats.ts` | Every wallet surface (balance, addresses, bump) displays sats; integer sats avoid float parsing hazards. PRD says "amount of BTC" generically — denomination toggle can come with Phase 9 chrome |
| **D7** | Send gets its own compact fee control (`SendFeeRateControl`) reusing `fee-selection/model` helpers and `useFeePresets`; it does **not** mount `FeeRateSelector` | `FeeRateSelector` renders a governance commit/reveal/dust breakdown (irrelevant to Send). Full chrome sharing is explicitly Phase 9; duplicating the *presentational* grid now (≈80 lines) is cheaper than prematurely parameterizing it |
| **D8** | Pre-send sync is **best-effort** (warn-and-continue), mirroring `admin_wallet_bump_fee` | If Electrum is down the user may still broadcast via the node fallback; a stale UTXO view is ultimately rejected by the network (`missing-or-spent`) and surfaced as `BroadcastFailed`. A hard sync gate would brick Send exactly when the fallback path matters |
| **D9** | Sends use the BDK default input sequence (RBF-signaling) | Phase 5 test `list_returns_unconfirmed_spend_with_fee_and_rbf_flag` proves BDK defaults signal BIP-125 — every Phase 6 send is automatically listed in "Pending transactions" and fee-bumpable via RBF with zero extra code |
| **D10** | Network word in the wrong-network copy comes from the backend (`expectedNetwork` field): `bitcoin → "mainnet"`, `testnet → "testnet"`, `regtest → "regtest"`, `signet → "signet"` | PRD names mainnet/testnet; regtest/signet are dev networks the same copy template covers. Single source of truth for the mapping (backend), template in the frontend |
| **D11** | Both new mutating/read commands are registered in **both** handler sets (`attach_production` and `attach_with_dev_signing`) | Capability is enforced per-signer at runtime (`allowed_on(network)`), same as Phase 5 (`phase5_commands_registered_in_both_handler_sets` test). Watch-only sessions get `ReadOnly` from the guard, not from registration |

---

## 3. Backend — types and contracts

All code below is normative for names, variants, and serde shapes; bodies are illustrative.

### 3.1 `application/wallet_send.rs` — errors **[P6.1, extended P6.2/P6.3]**

```rust
/// Typed failure surface for the Send use-case (PRD §4.3.5).
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    // Capability (guard order 1–2; before any lock or I/O)
    #[error("admin wallet is read-only — a signing-capable session is required to send")]
    ReadOnly,
    #[error("the session signer is not allowed on this network")]
    SignerNotAllowedOnNetwork,

    // Destination (§4.3.5.1) [P6.2; P6.1 ships the variants, minimal mapping]
    #[error("'{address}' is not a bitcoin address")]
    InvalidAddress { address: String },
    #[error("'{address}' is not a {expected_network} bitcoin address")]
    WrongNetwork { address: String, expected_network: String },

    // Amount (§4.3.5.2) [P6.3 closes; P6.1 ships zero-guard]
    #[error("send amount must be greater than zero")]
    InvalidAmount,
    #[error("send amount is below the dust limit for the destination script")]
    AmountBelowDust,
    #[error("insufficient funds: {message}")]
    InsufficientFunds { message: String },

    // Fee (§4.3.5.3)
    #[error("invalid fee rate: {0}")]
    InvalidFeeRate(#[from] crate::domain::fee_rate::FeeRateError),

    // Pipeline
    #[error("failed to build the send transaction: {message}")]
    BuildFailed { message: String },
    #[error("failed to sign the send transaction: {message}")]
    SignFailed { message: String },
    #[error("broadcast failed: {message}")]
    BroadcastFailed { message: String },
}

/// Stable code for the tagged `{ "type", "message" }` IPC shape
/// (mirrors `wallet_transactions::bump_error_code`).
pub fn send_error_code(e: &SendError) -> &'static str {
    match e {
        SendError::ReadOnly => "ReadOnly",
        SendError::SignerNotAllowedOnNetwork => "SignerNotAllowedOnNetwork",
        SendError::InvalidAddress { .. } => "InvalidAddress",
        SendError::WrongNetwork { .. } => "WrongNetwork",
        SendError::InvalidAmount => "InvalidAmount",
        SendError::AmountBelowDust => "AmountBelowDust",
        SendError::InsufficientFunds { .. } => "InsufficientFunds",
        SendError::InvalidFeeRate(_) => "InvalidFeeRate",
        SendError::BuildFailed { .. } => "BuildFailed",
        SendError::SignFailed { .. } => "SignFailed",
        SendError::BroadcastFailed { .. } => "BroadcastFailed",
    }
}
```

`ReadOnly`, `SignerNotAllowedOnNetwork`, `InvalidFeeRate`, `InsufficientFunds`, `BuildFailed`, `SignFailed`, `BroadcastFailed` reuse the **same code strings** as `BumpFeeError`, so the existing frontend union and `parse-admin-wallet-error.ts` already understand them. New union members for the frontend: `InvalidAddress`, `WrongNetwork`, `InvalidAmount`, `AmountBelowDust` (§6.2).

### 3.2 Destination validation helper **[P6.2]**

```rust
/// Network display word for PRD §4.3.5.1 copy.
/// bitcoin → "mainnet", testnet/testnet4 → "testnet", regtest → "regtest", signet → "signet".
pub fn network_display_name(network: bdk_wallet::bitcoin::Network) -> &'static str { … }

/// Parses `address` and enforces it belongs to `network`.
/// Pure function — no wallet lock, no I/O. Returns the checked address.
pub fn parse_send_destination(
    address: &str,
    network: bdk_wallet::bitcoin::Network,
) -> Result<bdk_wallet::bitcoin::Address, SendError> {
    use bdk_wallet::bitcoin::address::{Address, NetworkUnchecked};
    let unchecked: Address<NetworkUnchecked> =
        address.trim().parse().map_err(|_| SendError::InvalidAddress {
            address: address.to_string(),
        })?;
    unchecked.require_network(network).map_err(|_| SendError::WrongNetwork {
        address: address.to_string(),
        expected_network: network_display_name(network).to_string(),
    })
}
```

Semantics and documented behavior:

- **Accepted types:** whatever rust-bitcoin parses as a standard address — P2PKH, P2SH, P2WPKH, P2WSH, P2TR, and future witness versions. The PRD lists P2PK; P2PK has **no address encoding** (it is an output script type only), so it cannot be entered in an address field — documented here as satisfied-by-construction.
- **Network check:** `require_network` follows rust-bitcoin `is_valid_for_network` semantics. Bech32 HRPs are disjoint (`bc1`/`tb1`/`bcrt1`), so segwit/taproot mismatches are always caught. Base58 (legacy) testnet and regtest share version bytes — a legacy testnet address **is** accepted on regtest by upstream semantics. Documented limitation, irrelevant for mainnet (whose base58 prefixes are distinct).
- Empty/whitespace input never reaches IPC (frontend gates it), but the parser handles it as `InvalidAddress` anyway.

### 3.3 DTOs and use-case methods

```rust
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendInput {
    pub address: String,
    /// Ignored when `drain_wallet` is true (Max flow).
    pub amount_sats: u64,
    pub fee_rate_sat_per_kvb: u64,
    /// Max flow: spend all spendable UTXOs to `address`, no change output.
    #[serde(default)]
    pub drain_wallet: bool,
}

/// Dry-run result (estimate) — never signed, never broadcast. [P6.3]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEstimateDto {
    /// Echo of the effective amount: input amount, or the computed max for drain.
    pub amount_sats: u64,
    pub fee_sats: u64,
    pub fee_rate_sat_per_kvb: u64,
    pub vsize_vbytes: u64,
    /// Change returned to the wallet's internal keychain. 0 for drain builds.
    pub change_sats: u64,
    /// Max spendable to this destination at this rate (drain dry-run).
    /// Always present so the UI can keep the Max button and the boundary in sync.
    pub max_amount_sats: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResultDto {
    pub txid: String,
    pub amount_sats: u64,
    pub fee_sats: u64,
    pub fee_rate_sat_per_kvb: u64,  // realized rate: ceil(fee·1000 / vsize)
    pub vsize_vbytes: u64,
    pub change_sats: u64,           // 0 for drain
    pub drained: bool,
}

impl WalletService {
    /// Dry-run build for fee preview / Max / insufficient-funds boundary. [P6.3]
    /// Wallet lock only; no signing, no network I/O, no chain mutation.
    pub async fn estimate_send(&self, input: &SendInput) -> Result<SendEstimateDto, SendError>;

    /// Build → sign (PsbtSigner port) → broadcast (Electrum→node) → SendResultDto. [P6.1]
    pub async fn send_to_address(
        &self,
        input: &SendInput,
        rate: crate::domain::fee_rate::FeeRate,
        broadcasters: &[std::sync::Arc<dyn crate::application::tx_broadcaster::TxBroadcaster>],
    ) -> Result<SendResultDto, SendError>;
}
```

### 3.4 Build semantics (BDK) **[P6.1]**

**Normal send** (`drain_wallet == false`):

```rust
let mut wallet = self.wallet.lock().await;
let mut builder = wallet.build_tx();
builder.add_recipient(dest.script_pubkey(), Amount::from_sat(input.amount_sats));
builder.fee_rate(rate.to_bdk());
let psbt = builder.finish().map_err(map_send_create_tx_error)?;
```

**Max send** (`drain_wallet == true`):

```rust
let mut builder = wallet.build_tx();
builder.drain_wallet();
builder.drain_to(dest.script_pubkey());
builder.fee_rate(rate.to_bdk());
let psbt = builder.finish().map_err(map_send_create_tx_error)?;
```

Normative properties (each backed by a test in §7):

1. **Change routing (§4.3.5.4):** BDK's `TxBuilder` sources the change script from the wallet's **internal keychain next-unused index** — the same gap-aware discipline as R1.3's `next_unused_address`. The change output of a normal send MUST satisfy `wallet.is_mine(script)` with `KeychainKind::Internal` at the first unused index. Repeated estimate dry-runs MUST NOT advance the internal index (BDK's unused-aware change selection makes consecutive builds reuse the same index until it is observed used).
2. **Drain builds have exactly one recipient output** and no wallet-owned output; the recipient value is `max_amount_sats`.
3. **RBF:** inputs use the BDK default sequence (signals BIP-125). Do **not** call `set_exact_sequence`.
4. **Coin selection:** BDK default (spendable = confirmed + unconfirmed wallet-owned). The "wallet balance" of the PRD formula is `BalanceDto.total_sats` — same convention as the broadcast funding gate (`admin_wallet_info` doc, `commands/admin_wallet.rs:131-138`).
5. **Estimate/send consistency:** `estimate_send` and `send_to_address` MUST share one private `build_send_psbt(&mut wallet, …)` helper so a successful estimate can only diverge from the send by mempool/UTXO drift, never by construction.

**Error mapping** (`map_send_create_tx_error`, mirrors Phase 5's `map_create_tx_error`):

| `bdk_wallet::error::CreateTxError` | `SendError` |
|---|---|
| `CoinSelection(_)` | `InsufficientFunds { message }` |
| `OutputBelowDustLimit(_)` | `AmountBelowDust` |
| `FeeTooLow { .. }` / `FeeRateTooLow { .. }` | `BuildFailed` (unreachable in practice — `FeeRate::new` enforces min-relay first) |
| anything else | `BuildFailed { message }` |

### 3.5 `send_to_address` — normative step order **[P6.1]**

```text
1. signer()            → None ⇒ Err(ReadOnly)            (before any lock/network)
2. allowed_on(network) → false ⇒ Err(SignerNotAllowedOnNetwork)
3. parse_send_destination(input.address, self.network())  ⇒ InvalidAddress | WrongNetwork
4. !drain && amount_sats == 0                              ⇒ Err(InvalidAmount)
5. build PSBT per §3.4                                     ⇒ InsufficientFunds | AmountBelowDust | BuildFailed
6. sign_and_finalize_psbt(psbt)                            ⇒ SignFailed (nothing broadcast yet)
7. fee = wallet.calculate_fee(&tx)  (all prevouts wallet-known ⇒ exact)
   tx_hex = consensus::encode::serialize_hex(&tx)
   broadcast_single_with_fallback(broadcasters, &tx_hex)   ⇒ BroadcastFailed (join all errors with "; ")
8. Ok(SendResultDto { txid: tx.compute_txid().to_string(), … })
```

The best-effort pre/post sync (D8) lives in the **IPC command**, not in
`send_to_address` — exactly like `admin_wallet_bump_fee` — so the use-case
performs no network I/O besides the broadcast and its unit tests stay hermetic
(a live local electrs cannot contaminate fixture wallet state).

Step 6 failure is the **reject path** (§4.3.5.5.1): a signer error — including a future Phase 8 on-device rejection — returns `SignFailed` *before* any network contact. The UI keeps the filled form (§6.5). The `MnemonicPsbtSigner` cannot physically reject; the typed path is exercised in tests via a watch-only/`ReadOnly` session and by the existing `HwPsbtSigner` timeout mapping.

### 3.6 IPC commands (`commands/admin_wallet.rs`)

```rust
fn serialize_send_error(e: &SendError) -> String {
    serde_json::json!({ "type": send_error_code(e), "message": e.to_string() }).to_string()
}
```

**`admin_wallet_validate_send_address`** **[P6.2]** — pure, read-only, no signer required (watch-only sessions can type an address):

```rust
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAddressValidationDto {
    pub is_valid: bool,
    /// "invalid-address" | "wrong-network" | null
    pub reason: Option<String>,
    /// network_display_name(wallet.network) — drives the PRD copy template.
    pub expected_network: String,
}

#[tauri::command]
pub async fn admin_wallet_validate_send_address(
    address: String,
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<SendAddressValidationDto, String>   // Err only for Disabled (tagged, serialize_wallet_error)
```

Resolution: `current_or_fallback()` for the session network; `parse_send_destination` outcome maps `Ok → {isValid:true, reason:null}`, `InvalidAddress → "invalid-address"`, `WrongNetwork → "wrong-network"`. Validation failures are a **successful** IPC result (they are form states, not faults).

**`admin_wallet_estimate_send`** **[P6.3]** — read-only dry-run, no signer required:

```rust
#[tauri::command]
pub async fn admin_wallet_estimate_send(
    input: SendInput,
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<SendEstimateDto, String>   // Err: tagged SendError (serialize_send_error)
```

Validates the rate via `FeeRate::new(input.fee_rate_sat_per_kvb, FALLBACK_MIN_RELAY_SAT_PER_KVB)` first; then `WalletService::estimate_send`. **No pre-sync** (it runs per keystroke/debounce; the panel's sync loop keeps state fresh). `InsufficientFunds` / `AmountBelowDust` from the dry-run are how the form learns the §4.3.5.2 boundary before Confirm.

**`admin_wallet_send`** **[P6.1]** — mutating:

```rust
#[tauri::command]
pub async fn admin_wallet_send(
    input: SendInput,
    wallet_session: tauri::State<'_, WalletSession>,
    node_config: tauri::State<'_, NodeConfigState>,
) -> Result<SendResultDto, String>   // Err: tagged SendError
```

Body mirrors `admin_wallet_bump_fee` (`commands/admin_wallet.rs:283-328`): validate rate → resolve session → build broadcaster chain from current `NodeConfig` (`ElectrumBroadcaster::new(cfg.electrum_url())` then `NodeBroadcaster::new(rpc)`) → `svc.send_to_address(…)`.

**Registration (D11):** all three commands appear in **both** `attach_production` and `attach_with_dev_signing` in `commands/invoke.rs`. Compile-time-ish guard test extends the Phase 5 pattern (§7.3).

---

## 4. Backend — slice mapping

| Backend deliverable | Slice |
|---|---|
| `wallet_send.rs` with `SendError` (all variants), `send_error_code`, `parse_send_destination`, `network_display_name`, `build_send_psbt`, `send_to_address` | **P6.1** (destination errors exist but the form only consumes them in P6.2) |
| `admin_wallet_send` IPC + registration + broadcaster wiring | **P6.1** |
| `admin_wallet_validate_send_address` IPC | **P6.2** |
| `estimate_send` + `SendEstimateDto` + `admin_wallet_estimate_send` IPC | **P6.3** |
| No new backend work | **P6.4** (frontend-only closure) |

---

## 5. Frontend — API boundary (`api/admin-wallet.ts`)

```ts
// Phase 6 (PRD §4.3.5)
export type SendInput = {
	address: string
	amountSats: number
	feeRateSatPerKvb: number
	drainWallet?: boolean
}

export type SendAddressValidationDto = {
	isValid: boolean
	reason: 'invalid-address' | 'wrong-network' | null
	expectedNetwork: string
}

export type SendEstimateDto = {
	amountSats: number
	feeSats: number
	feeRateSatPerKvb: number
	vsizeVbytes: number
	changeSats: number
	maxAmountSats: number
}

export type SendResultDto = {
	txid: string
	amountSats: number
	feeSats: number
	feeRateSatPerKvb: number
	vsizeVbytes: number
	changeSats: number
	drained: boolean
}

export function validateSendAddress(address: string): Promise<ApiResult<SendAddressValidationDto>>
export function estimateSend(input: SendInput): Promise<ApiResult<SendEstimateDto>>
export function sendFromAdminWallet(input: SendInput): Promise<ApiResult<SendResultDto>>
```

### 5.1 `AdminWalletError` union extension **[P6.1/P6.2]**

Add to the existing union (codes shared with `BumpFeeError` already exist):

```ts
	// Phase 6 — Send (PRD §4.3.5)
	| { type: 'InvalidAddress'; message: string }
	| { type: 'WrongNetwork'; message: string }
	| { type: 'InvalidAmount'; message: string }
	| { type: 'AmountBelowDust'; message: string }
```

`parse-admin-wallet-error.ts` requires no structural change (it parses the tagged shape generically); extend its known-codes test fixtures.

---

## 6. Frontend — components, hooks, models

Layout (per `react-frontend-patterns` rules — components presentational, hooks own state/effects, model pure):

```text
domain/admin-wallet/
  components/
    send-form.tsx              # composed form: destination, amount+Max, fee, summary, Confirm
    send-fee-rate-control.tsx  # presets (Slow/Medium/Fast/Custom) + 0.1 stepper  [P6.3; P6.1 hides it]
    send-result-card.tsx       # success card: txid + CopyButton + Done           [P6.1, polished P6.4]
  hooks/
    use-send-form.ts           # field state + debounced validate/estimate + canConfirm  [grows P6.1→P6.4]
    use-send.ts                # submit machine: idle → submitting → success | error     [P6.1]
  model/
    send-validation.ts         # pure: destination/amount field states, canConfirm predicate
    format-send-error.ts       # typed code → exact PRD copy (table below)
```

### 6.1 Entry point and placement **[P6.1]**

- `WalletPanelContent` gains a **Send** primary action (button row under `WalletBalance`, alongside the Receive row) that calls `onOpenSend()` → `setExpandedSection('send')`.
- When `expandedSection === 'send'`, the panel body renders `SendForm` in place of the accordion stack (full-height sub-view with a back affordance returning to `null`), keeping the slide-over single-column model. URL deep-link `?wallet=open&walletSection=send` already works via `use-wallet-panel-state.ts`.
- **Watch-only** (`canSign === false` from `useAdminWalletCapability`): the Send button renders disabled with the 3.8 copy **"Hardware wallet required to sign"** — visible, not hidden. The section, if deep-linked, shows the same notice instead of the form. **[P6.1]**

### 6.2 Exact PRD copy (single source: `format-send-error.ts`) **[P6.2/P6.3]**

| Trigger (typed code / state) | Exact user-facing copy | PRD |
|---|---|---|
| `InvalidAddress` / validation `reason: 'invalid-address'` | `Destination must be a bitcoin address.` | §4.3.5.1 |
| `WrongNetwork` / `reason: 'wrong-network'` | `Destination must be a {expectedNetwork} bitcoin address.` (e.g. *…a regtest bitcoin address.*) | §4.3.5.1 |
| `InsufficientFunds` (estimate or send) | `Insufficient funds` | §4.3.5.2 |
| `AmountBelowDust` | `Amount is below the dust limit.` | — (consensus constraint, high-signal) |
| `ReadOnly` | `Hardware wallet required to sign` | §3.2 / 3.8 pattern |
| `SignerNotAllowedOnNetwork` | `This signer cannot send on {network}.` | §3.2 |
| `SignFailed` | `Signing was rejected or failed — nothing was broadcast.` | §4.3.5.5.1 |
| `BroadcastFailed` | `Broadcast failed: {message}` (+ form retained) | — |

Unit tests assert these strings byte-for-byte (the two §4.3.5.1 strings are PRD MUSTs).

### 6.3 `use-send-form.ts` — state machine

```ts
type DestinationState =
	| { status: 'empty' }
	| { status: 'validating'; address: string }
	| { status: 'valid'; address: string }
	| { status: 'invalid'; address: string; reason: 'invalid-address' | 'wrong-network'; expectedNetwork: string }

type AmountState =
	| { status: 'empty' }
	| { status: 'set'; sats: number; isMax: boolean }   // isMax ⇒ drainWallet on submit

type EstimateState =
	| { status: 'idle' }
	| { status: 'loading' }
	| { status: 'ready'; estimate: SendEstimateDto }
	| { status: 'error'; error: AdminWalletError }      // InsufficientFunds lands here pre-Confirm
```

Behavior contract:

- **Destination** **[P6.2]**: debounce 300 ms after last keystroke → `validateSendAddress`. Stale responses discarded (request id / active-flag pattern as in `useFeePresets`). Inline error rendered from §6.2; Confirm gated on `status === 'valid'`.
- **Amount** **[P6.1 basic, P6.3 full]**: integer sats input (`inputMode="numeric"`, reject non-digits). `0`/empty keeps Confirm disabled (no error shown for empty; `Amount must be greater than zero` for explicit 0). BTC equivalent sub-label.
- **Max** **[P6.3]**: enabled only when destination is `valid` and presets are `ready` (vsize depends on the destination script type — D3). Click sets `{ sats: estimate.maxAmountSats, isMax: true }` from a drain dry-run. Any manual edit of amount, destination, or rate clears `isMax`.
- **Fee** **[P6.3]**: `useFeePresets` with initial selection `{ kind: 'preset', preset: 'fast' }` (D4); `SendFeeRateControl` renders presets + custom (step `FEE_RATE_STEP_SAT_PER_KVB`, bounds `minRelaySatPerKvb`/`maxSatPerKvb` from the presets DTO — never hardcoded). Rate changes re-run the estimate; if `isMax`, the Max amount is recomputed from the new drain dry-run (PRD: boundary recomputes on fee change).
- **Estimate** **[P6.3]**: debounced on (valid destination ∧ amount > 0 ∧ rate). `ready` feeds the summary row (`Network fee ~N sats · change M sats`); `error: InsufficientFunds` renders the §4.3.5.2 copy and disables Confirm.
- **`canConfirm`** **[P6.4 closes]** — pure predicate in `send-validation.ts`:

```ts
canConfirm =
	destination.status === 'valid' &&
	amount.status === 'set' && amount.sats > 0 &&
	feePresets.status === 'ready' &&
	estimate.status === 'ready' &&
	submit.status !== 'submitting'
```

In **P6.1** (no validate/estimate IPC yet) the interim gate is `address non-empty ∧ sats > 0 ∧ presets ready`; the backend remains the authority. P6.2/P6.3 tighten it; P6.4 asserts the final predicate.

### 6.4 `use-send.ts` **[P6.1]**

Carbon copy of `use-bump-fee.ts` semantics:

```ts
export type SendState =
	| { status: 'idle' }
	| { status: 'submitting' }
	| { status: 'success'; result: SendResultDto }
	| { status: 'error'; error: AdminWalletError }
// send(input): Promise<boolean>; reset(): void
```

On `success`, the form triggers the panel's `onRefreshSync` (same hook the bump flow uses) so balance and the Phase 5 pending list converge immediately.

### 6.5 Result / reject / retry surfaces **[P6.1 minimal, P6.4 complete]**

- **Success:** `SendResultCard` — green card per `bump-fee-form.tsx` success branch: amount + destination (truncated, `title` = full), `truncTxid(txid)` + `CopyButton`, fee paid, `Done` (resets form to empty and returns to the panel root). The send appears in **Pending transactions** (Phase 5) as an RBF row — no extra wiring (D9).
- **Error (`SignFailed`, `BroadcastFailed`, transient):** inline error line **below the Confirm button**; **all field values retained**; Confirm re-enabled (retry) — §4.3.5.5.1 "nothing happens in the UI; retry or back out". Back affordance always active except while `submitting`.
- **While `submitting`:** Confirm shows `Sending…`, all inputs disabled, back affordance disabled.

### 6.6 Test ids (e2e-webdriver convention)

`e2e-wallet-send-open`, `e2e-wallet-send-form`, `e2e-wallet-send-address-input`, `e2e-wallet-send-address-error`, `e2e-wallet-send-amount-input`, `e2e-wallet-send-max`, `e2e-wallet-send-amount-error`, `e2e-wallet-send-fee-control`, `e2e-wallet-send-estimate`, `e2e-wallet-send-confirm`, `e2e-wallet-send-error`, `e2e-wallet-send-success`, `e2e-wallet-send-txid`.

### 6.7 Architecture Rule 8 **[P6.4]**

Extend `domain/admin-wallet/architecture.test.ts`:

- `send-form.tsx` must not import `@/api/admin-wallet` or `@tauri-apps/api/core` (Rule 1 already covers components — Rule 8 asserts the **wiring**):
- `wallet-panel-content.tsx` (or panel composition) must reference `SendForm` and pass a capability-derived disabled state;
- `format-send-error.ts` must contain the two literal PRD §4.3.5.1 strings (`must be a bitcoin address.` and `bitcoin address."` template) so copy regressions fail CI.

---

## 7. Test plan

Naming, fixtures, and helpers follow `wallet_transactions.rs` tests (`funded_wallet`, `signing_service`, `MockBroadcaster`, `external_script`, `TEST_MNEMONIC`).

### 7.1 Rust unit — `wallet_send.rs`

**P6.1 — pipeline**

| Test | Asserts |
|---|---|
| `send_on_watch_only_returns_read_only_before_any_broadcast` | `Err(ReadOnly)`; `mock.sent_single().is_empty()` |
| `send_zero_amount_returns_invalid_amount` | guard 4 |
| `send_happy_path_signs_and_broadcasts` | `Ok`; broadcast hex decodes to a tx paying `external_script()` the exact amount; `result.txid == decoded.compute_txid()` |
| `send_change_goes_to_first_unused_internal_index` | decoded change output: `wallet.is_mine`, keychain `Internal`, derivation index == first unused before the send |
| `send_inputs_signal_rbf` | every input `sequence.is_rbf()` (D9) |
| `send_insufficient_funds_returns_typed_error` | amount > balance ⇒ `Err(InsufficientFunds)`; nothing broadcast |
| `send_below_dust_returns_amount_below_dust` | 100 sats to P2WSH ⇒ `Err(AmountBelowDust)` |
| `send_broadcast_fallback_to_node` / `send_all_broadcasters_failing_returns_broadcast_failed` | fallback order; aggregated `"; "`-joined message (mirrors Phase 5) |
| `send_fee_matches_calculate_fee` | `result.fee_sats == wallet.calculate_fee(&tx)` |

**P6.2 — destination**

| Test | Asserts |
|---|---|
| `parse_destination_accepts_each_standard_type_on_regtest` | P2PKH, P2SH, P2WPKH, P2WSH, P2TR vectors parse `Ok` |
| `parse_destination_garbage_returns_invalid_address` | `"not-an-address"`, `""`, `"bc1qqqqq"` |
| `parse_destination_mainnet_address_on_regtest_returns_wrong_network` | `expected_network == "regtest"` |
| `network_display_name_maps_all_networks` | bitcoin→mainnet, testnet→testnet, regtest→regtest, signet→signet |
| `send_invalid_destination_fails_before_sync_or_broadcast` | guard 3 ordering: no broadcaster contact |

**P6.3 — estimate / Max**

| Test | Asserts |
|---|---|
| `estimate_normal_send_returns_fee_change_and_max` | `change_sats == inputs − amount − fee`; `max_amount_sats > 0` |
| `estimate_is_side_effect_free` | two consecutive estimates: identical result; internal keychain index unchanged; balance unchanged |
| `estimate_drain_returns_max_and_zero_change` | `drain_wallet: true` ⇒ `change_sats == 0`, `amount_sats == max_amount_sats` |
| `max_then_send_drain_spends_everything` | estimate(max) → send(drain): broadcast tx has no wallet-owned output; `amount == estimate.max_amount_sats` |
| `estimate_amount_over_balance_returns_insufficient_funds` | §4.3.5.2 boundary pre-Confirm |
| `max_recomputes_when_rate_changes` | higher rate ⇒ strictly smaller `max_amount_sats` |
| `estimate_send_consistency` | for the same input, `send` realizes the estimated fee exactly (shared builder, same wallet state) |

### 7.2 Rust — DTO/serde contracts

`SendInput` deserializes camelCase with `drainWallet` defaulting false; `SendEstimateDto`/`SendResultDto` serialize camelCase (golden JSON asserts like `unconfirmed_tx_dto_serializes_camel_case`); `serialize_send_error` emits `{type, message}` for every variant (table-driven over `send_error_code`).

### 7.3 Rust — IPC registration

`phase6_commands_registered_in_both_handler_sets` — `include_str!("invoke.rs")`, each of the three commands appears exactly **2×** (clone of `phase5_commands_registered_in_both_handler_sets`).

### 7.4 Frontend (vitest)

- `send-validation.test.ts`: `canConfirm` truth table (every gating dimension toggled independently); amount parsing (rejects `-1`, `1.5`, `1e3`, non-digits; accepts `0`→ disabled).
- `format-send-error.test.ts`: **byte-exact** PRD strings for `InvalidAddress` and `WrongNetwork('regtest')`; `InsufficientFunds` → `Insufficient funds`.
- `use-send-form.test.ts`: debounce fires once per settle; stale validation response discarded; Max sets `isMax` and any edit clears it; rate change re-estimates and recomputes Max when `isMax`.
- `use-send.test.ts`: machine transitions (mirror `use-bump-fee` tests); error keeps previous form state untouched (hook owns no field state — assert no reset callback fired).
- `architecture.test.ts` Rule 8 (§6.7).

### 7.5 e2e-webdriver (optional, P6.4)

`send.spec` in `desktop-app/e2e-webdriver`: fund via `bitcoin-cli -rpcwallet=asm-runner`, open panel → Send, paste a regtest address, amount, Confirm, assert `e2e-wallet-send-success` txid; mine 1 block; assert balance decreased and tx confirmed. Wire as a separate `test:e2e:send` script (suite convention: one flow per script).

---

## 8. Slice execution summary

| Slice | Backend | Frontend | Done when (verifiable) |
|---|---|---|---|
| **P6.1** | `wallet_send.rs` (full error enum, `send_to_address`, builder, mappings); `admin_wallet_send` IPC; registration | Send button + watch-only disabled state; minimal `SendForm` (address text, sats amount, Fast-preset rate label, Confirm); `use-send.ts`; success card with txid; error line retains form | Regtest: dev-mnemonic login sends to a `bcrt1` address, txid shown, change at first unused internal index, send listed in Pending transactions as RBF row; §7.1-P6.1 + §7.2 + §7.3 green |
| **P6.2** | `admin_wallet_validate_send_address` | Debounced destination validation; inline PRD copy; Confirm gated on `valid` | Both §4.3.5.1 copies render byte-exact for bad input / mainnet-address-on-regtest; Confirm disabled while invalid; §7.1-P6.2 + copy tests green |
| **P6.3** | `estimate_send` + `admin_wallet_estimate_send` | `SendFeeRateControl` (presets default **Fast**, custom 0.1-step, bounds from DTO); Max button (drain dry-run); estimate summary; `Insufficient funds` pre-Confirm; Max/boundary recompute on rate change | Max + Confirm drains the wallet on regtest; over-balance amount shows `Insufficient funds` without submitting; rate bump shrinks Max; §7.1-P6.3 green |
| **P6.4** | — | Final `canConfirm` predicate; complete reject/retry surfaces; submitting lock; Rule 8; (optional) e2e spec; UI polish to panel conventions | Full §4.3.5 MUST checklist on the dev-mnemonic path passes manual playbook (§9); compliance matrix rows flipped to **PASS (regtest / dev mnemonic)** |

Each slice lands as its own PR (branch from `develop`), with the standard pre-commit CI gate (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run format:check`, `npm run lint`, `npm run build`).

---

## 9. Manual regtest playbook (P6.4 exit)

Stack: `scripts/bitcoind-asm-runner.sh` + electrs (R2.1 compose) + dev-mnemonic ("Palabras") login.

1. Fund: `bitcoin-cli -rpcwallet=asm-runner -regtest sendtoaddress <panel receive address> 0.001` + mine 1 block → panel shows 100 000 sats confirmed.
2. **Happy path:** Send → destination `bitcoin-cli getnewaddress` (bech32) → 30 000 sats → default rate shows Fast preset → Confirm → txid card; `getrawmempool` contains the txid; Pending transactions shows the send (RBF). Mine 1; balance = 100 000 − 30 000 − fee.
3. **Change discipline:** `bitcoin-cli getrawtransaction <txid> 2` — change output address equals the wallet's first unused internal address (`admin_wallet_list_addresses keychain=internal`).
4. **Validation copy:** paste `bc1q…` (mainnet) → *"Destination must be a regtest bitcoin address."*; paste garbage → *"Destination must be a bitcoin address."*; Confirm stays disabled in both.
5. **Insufficient funds:** amount 10 BTC → *"Insufficient funds"*, Confirm disabled.
6. **Max:** click Max → amount fills; raise rate by +5 sat/vB → amount shrinks; Confirm → wallet balance reaches 0 (no change row appears afterwards).
7. **Retry path:** stop electrs **and** bitcoind → Confirm → `Broadcast failed…`, form intact → restart services → Confirm again → success.
8. **Watch-only:** HW/xpub login → Send button visible, disabled, "Hardware wallet required to sign".
9. **Fee-bump interplay:** broadcast a send without mining → bump it via the Phase 5 row (RBF) → replacement confirms.

---

## 10. Risks and edge cases

| Risk | Mitigation |
|---|---|
| Estimate/send drift (UTXO changes between dry-run and Confirm) | Shared `build_send_psbt`; send re-runs coin selection at Confirm time; worst case is a typed `InsufficientFunds`/`BroadcastFailed` with the form retained |
| Estimate advancing the internal keychain index | `estimate_is_side_effect_free` test pins BDK's unused-aware change behavior; regression breaks CI |
| Legacy base58 testnet addresses accepted on regtest | Upstream rust-bitcoin semantics; documented in §3.2 — mainnet unaffected |
| `drain_wallet` with zero spendable UTXOs | BDK `CoinSelection` error → `InsufficientFunds`; Max button additionally hidden when `totalSats == 0` |
| Spending unconfirmed change immediately after a send | Allowed by design (BDK default; matches broadcast funding-gate convention). The chain enforces ancestry; failures surface as `BroadcastFailed` |
| Mnemonic signer on mainnet | Guard 2 (`SignerNotAllowedOnNetwork`, R1.1) — covered by existing signer-capability tests plus the §6.2 copy |
| Send during an in-flight background sync | `wallet` Mutex serializes; `sync()` collapses concurrent calls (`sync_in_flight`) — no new locking introduced |
| PRD copy drift | Byte-exact unit tests + architecture Rule 8 literal check |

## 11. Explicitly out of scope (unchanged from the roadmap spec)

HW on-device Send confirm (Phase 8 — drops in behind `PsbtSigner` with zero UI change), mainnet enablement (Phase 10), shared Send chrome with governance broadcast (Phase 9), QR (Phase 7), coin control, multi-recipient, address book, fiat display, BTC-denominated input (Phase 9 candidate).
