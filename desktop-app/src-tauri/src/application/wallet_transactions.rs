//! Phase 5 (PRD §4.3.3) — unconfirmed sent-transaction listing and fee-bump
//! use-cases over [`WalletService`].
//!
//! Two bump methods, dispatched by transaction kind:
//! - **RBF** for plain RBF-signaling wallet sends (BDK `build_fee_bump`).
//! - **CPFP** for governance commits with a pending pre-signed reveal — the commit
//!   cannot be replaced (a new txid would orphan the reveal, whose ephemeral key is
//!   dropped after signing, R1.0.1), so a child spends the reveal's wallet-owned
//!   change output and raises the effective rate of the whole package.
//!
//! Business logic only: depends on the [`TxBroadcaster`] port and the session
//! [`PsbtSigner`](crate::application::psbt_signer::PsbtSigner) port (via
//! `WalletService::sign_and_finalize_psbt`), never on concrete transports.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

use crate::application::tx_broadcaster::{broadcast_single_with_fallback, TxBroadcaster};
use crate::application::wallet_service::WalletService;
use crate::domain::fee_rate::{FeeRate, MAX_BROADCAST_SAT_PER_KVB};
use crate::infrastructure::admin_wallet::AdminWalletError;

// ── DTOs ────────────────────────────────────────────────────────────────────

/// How an unconfirmed transaction's fee can be bumped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BumpMethod {
    /// Replace the transaction with a higher-fee version (BIP-125).
    Rbf,
    /// Spend the reveal's change with a high-fee child so miners take the package.
    Cpfp,
}

/// One unconfirmed transaction **sent from** the Admin Wallet (wallet-owned inputs).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnconfirmedTxDto {
    pub txid: String,
    /// Sats spent from wallet-owned inputs.
    pub sent_sats: u64,
    /// Sats received back to the wallet (change + self-transfers).
    pub received_sats: u64,
    /// `received - sent`. Negative for sends.
    pub net_sats: i64,
    /// Absolute fee. `None` when a prevout is unknown to the wallet (foreign input).
    pub fee_sats: Option<u64>,
    /// Current fee rate in sat/kvB (Phase 4 unit convention). `None` when fee is unknown.
    pub fee_rate_sat_per_kvb: Option<u64>,
    pub vsize_vbytes: u64,
    /// True when at least one input signals BIP-125 replaceability.
    pub is_rbf_signaling: bool,
    /// True when this txid is a governance commit with a pending pre-signed reveal —
    /// bumped via CPFP on the reveal's change (RBF would orphan the reveal, R1.0.1).
    pub is_governance_commit: bool,
    /// Governance commit → `Cpfp`, RBF-signaling send → `Rbf`, otherwise `None`.
    pub bump_method: Option<BumpMethod>,
    /// commit fee + reveal fee — `Some` only for governance commits whose reveal
    /// (and both fees) are known to the wallet graph.
    pub package_fee_sats: Option<u64>,
    pub package_vsize_vbytes: Option<u64>,
    /// Effective package rate in sat/kvB — the floor a CPFP bump must exceed.
    pub package_fee_rate_sat_per_kvb: Option<u64>,
    /// Highest package rate this row can be bumped to, in sat/kvB. `Some` only for CPFP
    /// rows: the child pays the package's shortfall out of its own vsize, so the package
    /// rate tops out well below the 10,000 sat/vB an operator may ask for (#431). `None`
    /// for RBF rows, where the general ceiling is the only limit.
    pub max_bump_rate_sat_per_kvb: Option<u64>,
    /// Mempool last-seen, unix seconds. `None` when the indexer gave no timestamp.
    pub last_seen_secs: Option<u64>,
}

/// Outcome of a successful fee-bump: the replacement (RBF) or child (CPFP) is
/// signed and broadcast.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpFeeResultDto {
    /// RBF: replacement txid. CPFP: child txid.
    pub new_txid: String,
    /// The txid the user asked to bump.
    pub target_txid: String,
    /// RBF: replacement fee. CPFP: child fee.
    pub fee_sats: u64,
    /// RBF: replacement rate. CPFP: resulting package rate.
    pub fee_rate_sat_per_kvb: u64,
    pub method: BumpMethod,
    /// Warning message when pre-bump sync failed but bump proceeded with stale state.
    /// UI should display this to alert the user that the wallet view may be outdated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_warning: Option<String>,
}

// ── Errors ──────────────────────────────────────────────────────────────────

/// Typed failure surface for the fee-bump use-case (PRD §4.3.3, RBF-first).
#[derive(Debug, thiserror::Error)]
pub enum BumpFeeError {
    #[error("admin wallet is read-only — a signing-capable session is required to bump fees")]
    ReadOnly,
    #[error("the session signer is not allowed on this network")]
    SignerNotAllowedOnNetwork,
    #[error("'{txid}' is not a valid transaction id")]
    InvalidTxid { txid: String },
    #[error("transaction {txid} was not found in the wallet")]
    TxNotFound { txid: String },
    #[error("transaction {txid} is already confirmed and can no longer be replaced")]
    TxAlreadyConfirmed { txid: String },
    #[error("transaction {txid} does not signal RBF and cannot be replaced")]
    TxNotReplaceable { txid: String },
    #[error("cannot accelerate {txid}: {message}")]
    CpfpOutputUnavailable { txid: String, message: String },
    #[error("replacement fee too low: at least {required_fee_sats} sats are required")]
    FeeTooLow { required_fee_sats: u64 },
    #[error("replacement fee rate too low: at least {required_sat_per_kvb} sat/kvB is required")]
    FeeRateTooLow { required_sat_per_kvb: u64 },
    /// The requested package rate would price the CPFP child above what a node accepts
    /// for a single transaction. Distinct from `InvalidFeeRate`, which guards the rate
    /// the operator typed: this one is about what that rate costs the child (#431).
    #[error(
        "this rate needs {child_sat_per_kvb} sat/kvB on the child transaction alone, over the \
         {MAX_BROADCAST_SAT_PER_KVB} sat/kvB a node accepts — this package tops out at \
         {max_sat_per_kvb} sat/kvB"
    )]
    FeeRateTooHigh {
        max_sat_per_kvb: u64,
        child_sat_per_kvb: u64,
    },
    #[error("insufficient funds to pay the increased fee: {message}")]
    InsufficientFunds { message: String },
    /// The wallet holds funds, but none the CPFP child is allowed to spend: immature
    /// coinbase, unconfirmed coins from outside this package, and other pending
    /// packages' anchors are all excluded. Distinct from `InsufficientFunds` because
    /// telling the operator their balance is too small would be false.
    #[error("no eligible coins to fund the acceleration: {message}")]
    CpfpFundingUnavailable { message: String },
    #[error("failed to build the replacement transaction: {message}")]
    BuildFailed { message: String },
    #[error("failed to sign the replacement transaction: {message}")]
    SignFailed { message: String },
    #[error("invalid fee rate: {0}")]
    InvalidFeeRate(#[from] crate::domain::fee_rate::FeeRateError),
    #[error("broadcast failed: {message}")]
    BroadcastFailed { message: String },
}

/// Stable error code for the tagged `{ "type", "message" }` IPC error shape
/// (mirrors `wallet_service::error_code`).
pub fn bump_error_code(e: &BumpFeeError) -> &'static str {
    match e {
        BumpFeeError::ReadOnly => "ReadOnly",
        BumpFeeError::SignerNotAllowedOnNetwork => "SignerNotAllowedOnNetwork",
        BumpFeeError::InvalidTxid { .. } => "InvalidTxid",
        BumpFeeError::TxNotFound { .. } => "TxNotFound",
        BumpFeeError::TxAlreadyConfirmed { .. } => "TxAlreadyConfirmed",
        BumpFeeError::TxNotReplaceable { .. } => "TxNotReplaceable",
        BumpFeeError::CpfpOutputUnavailable { .. } => "CpfpOutputUnavailable",
        BumpFeeError::FeeTooLow { .. } => "FeeTooLow",
        BumpFeeError::FeeRateTooLow { .. } => "FeeRateTooLow",
        BumpFeeError::FeeRateTooHigh { .. } => "FeeRateTooHigh",
        BumpFeeError::InsufficientFunds { .. } => "InsufficientFunds",
        BumpFeeError::CpfpFundingUnavailable { .. } => "CpfpFundingUnavailable",
        BumpFeeError::InvalidFeeRate(_) => "InvalidFeeRate",
        BumpFeeError::BuildFailed { .. } => "BuildFailed",
        BumpFeeError::SignFailed { .. } => "SignFailed",
        BumpFeeError::BroadcastFailed { .. } => "BroadcastFailed",
    }
}

// ── Fee arithmetic ──────────────────────────────────────────────────────────

/// Fee rate in sat/kvB from an absolute fee and a vsize (ceiling, never underreports).
/// Shared with the Phase 6 send path (`wallet_send.rs`).
pub(crate) fn fee_rate_sat_per_kvb(fee_sats: u64, vsize_vbytes: u64) -> u64 {
    fee_sats.saturating_mul(1_000).div_ceil(vsize_vbytes.max(1))
}

/// Non-witness skeleton weight of a transaction: version (4) + locktime (4) +
/// the input and output count varints (1 each — the child never gets near 253).
const TX_HEADER_WU: u64 = 4 * (4 + 4 + 1 + 1);
/// Segwit marker + flag, paid once as soon as any input carries a witness.
const SEGWIT_MARKER_WU: u64 = 2;
/// Per-input skeleton weight: outpoint (36) + empty scriptSig varint (1) + sequence (4).
const TX_INPUT_BASE_WU: u64 = 4 * (36 + 1 + 4);

/// vsize of the *signed* CPFP child: `input_count` inputs whose combined
/// satisfaction (witness) weight is `satisfaction_wu`, plus the single drain output.
///
/// The satisfaction weight comes from the wallet descriptor, so the estimate follows
/// the actual signing path instead of a magic constant (#431/F-006): for the BIP-86
/// `tr()` admin descriptor this yields the historical 111 vB at one input, and grows
/// by a full input whenever the fee forces another coin in.
fn cpfp_child_vsize(
    input_count: u64,
    drain_script: &bdk_wallet::bitcoin::Script,
    satisfaction_wu: u64,
) -> u64 {
    let output_wu = 4 * (8 + 1 + drain_script.len() as u64);
    bdk_wallet::bitcoin::Weight::from_wu(
        TX_HEADER_WU
            + SEGWIT_MARKER_WU
            + input_count * TX_INPUT_BASE_WU
            + output_wu
            + satisfaction_wu,
    )
    .to_vbytes_ceil()
}

/// Known fee + vsize of the unconfirmed `commit → reveal` pair.
#[derive(Debug, Clone, Copy)]
struct PackageStats {
    fee_sats: u64,
    vsize_vbytes: u64,
}

impl PackageStats {
    /// Absolute child fee needed so `(package_fee + child_fee) / (package_vsize +
    /// child_vsize)` reaches `rate`, for a child of `child_vsize_vbytes`. Errors when
    /// the result would not even pay the child's own 1 sat/vB relay floor — i.e. the
    /// package already meets the rate — or when it would push the child past what a
    /// node accepts for a single transaction.
    fn required_child_fee(
        self,
        rate: FeeRate,
        child_vsize_vbytes: u64,
    ) -> Result<u64, BumpFeeError> {
        let total_vsize = self.vsize_vbytes + child_vsize_vbytes;
        let child_fee = rate.fee_sats(total_vsize).saturating_sub(self.fee_sats);
        // The child must pay at least its own min-relay share (1 sat/vB).
        let child_floor = child_vsize_vbytes;
        if child_fee < child_floor {
            return Err(BumpFeeError::FeeRateTooLow {
                required_sat_per_kvb: fee_rate_sat_per_kvb(
                    self.fee_sats + child_floor,
                    total_vsize,
                ),
            });
        }
        // …and no more than a node will take for one transaction. The child pays the
        // whole package's shortfall out of its own vsize, so its individual rate runs
        // `(package + child) / child` times the requested package rate — roughly 3x in
        // the ordinary commit+reveal case. Left unchecked the operator gets rust-bitcoin's
        // `AbsurdFeeRate` at signing time, quoted in sat/kwu (#431).
        let child_sat_per_kvb = fee_rate_sat_per_kvb(child_fee, child_vsize_vbytes);
        if child_sat_per_kvb > MAX_BROADCAST_SAT_PER_KVB {
            return Err(BumpFeeError::FeeRateTooHigh {
                max_sat_per_kvb: self.max_package_rate_sat_per_kvb(child_vsize_vbytes),
                child_sat_per_kvb,
            });
        }
        Ok(child_fee)
    }

    /// Highest package rate a child of `child_vsize_vbytes` can carry without exceeding
    /// [`MAX_BROADCAST_SAT_PER_KVB`] on its own.
    ///
    /// Inverts `required_child_fee`: with `child_fee = CAP · C / 1000` the package rate is
    /// `(child_fee + package_fee) · 1000 / (P + C)`, i.e. `(CAP · C + F · 1000) / (P + C)`.
    /// Floored, so the figure handed to the UI is always one the bump can actually honour.
    fn max_package_rate_sat_per_kvb(self, child_vsize_vbytes: u64) -> u64 {
        let total_vsize = (self.vsize_vbytes + child_vsize_vbytes).max(1);
        MAX_BROADCAST_SAT_PER_KVB
            .saturating_mul(child_vsize_vbytes)
            .saturating_add(self.fee_sats.saturating_mul(1_000))
            / total_vsize
    }
}

/// F-006: does the package `commit + reveal + child` fall short of `requested` once the
/// child is measured as it will be broadcast? Returns the realized rate when it does.
///
/// A 10% tolerance is allowed: the child is itself RBF-bumpable, so landing slightly under
/// is recoverable, while refusing outright would strand a bump over a rounding sliver.
fn package_rate_shortfall(
    package: PackageStats,
    child_fee_sats: u64,
    child_vsize_vbytes: u64,
    requested: FeeRate,
) -> Option<u64> {
    let realized = fee_rate_sat_per_kvb(
        package.fee_sats + child_fee_sats,
        package.vsize_vbytes + child_vsize_vbytes,
    );
    let min_acceptable = requested.sat_per_kvb().saturating_mul(90) / 100;
    (realized < min_acceptable).then_some(realized)
}

/// vsize of a built-but-unsigned child once its witnesses are attached.
fn signed_child_vsize(unsigned: &bdk_wallet::bitcoin::Transaction, satisfaction_wu: u64) -> u64 {
    (unsigned.weight() + bdk_wallet::bitcoin::Weight::from_wu(SEGWIT_MARKER_WU + satisfaction_wu))
        .to_vbytes_ceil()
}

/// The coins a CPFP child spends and the absolute fee they must carry.
struct CpfpChildInputs {
    utxos: Vec<bdk_wallet::LocalOutput>,
    fee_sats: u64,
}

impl CpfpChildInputs {
    /// Combined witness weight of the selection, from the wallet descriptors.
    fn satisfaction_wu(&self, witness_wu: impl Fn(bdk_wallet::KeychainKind) -> u64) -> u64 {
        self.utxos
            .iter()
            .map(|utxo| witness_wu(utxo.keychain))
            .sum()
    }
}

/// Chooses what the CPFP child spends: the `anchor` is mandatory, then as few `spare`
/// coins as it takes to cover the child fee **plus** a non-dust drain output.
///
/// Selecting explicitly is the fix for #431: with `fee_absolute` and no recipient the
/// effective rate is zero, so BDK's branch-and-bound stops as soon as the mandatory
/// input covers the fee alone. When the leftover then falls under the dust limit the
/// change is dropped, the child is left with no output, and coin selection reports
/// insufficient funds on a wallet that is not short of funds at all.
///
/// **Smallest coin that closes the gap**, not largest-first: every bump parks its inputs
/// behind an unconfirmed child, and an evicted child is not noticed by the sync path, so
/// a bump that swept the biggest coin would strand most of the balance. When no single
/// coin closes the gap the largest is taken to advance as far as possible, and the pass
/// repeats.
///
/// Fee and size chase each other — a coin added to pay the fee makes the child bigger,
/// which raises the fee — so sizing and selection alternate. Every pass that does not
/// settle consumes at least one spare coin, so the loop is bounded by `spare.len()`:
/// once the spares run out the pass reports insufficient funds.
fn select_cpfp_child_inputs(
    anchor: bdk_wallet::LocalOutput,
    mut spare: Vec<bdk_wallet::LocalOutput>,
    drain_script: &bdk_wallet::bitcoin::Script,
    witness_wu: impl Fn(bdk_wallet::KeychainKind) -> u64,
    package: PackageStats,
    rate: FeeRate,
) -> Result<CpfpChildInputs, BumpFeeError> {
    let dust_threshold = drain_script.minimal_non_dust().to_sat();
    // Ascending, ties broken by outpoint so the selection is deterministic.
    spare.sort_by(|a, b| {
        a.txout
            .value
            .cmp(&b.txout.value)
            .then_with(|| a.outpoint.txid.cmp(&b.outpoint.txid))
            .then_with(|| a.outpoint.vout.cmp(&b.outpoint.vout))
    });

    let fee_for = |utxos: &[bdk_wallet::LocalOutput]| -> Result<u64, BumpFeeError> {
        let satisfaction: u64 = utxos.iter().map(|utxo| witness_wu(utxo.keychain)).sum();
        let child_vsize = cpfp_child_vsize(utxos.len() as u64, drain_script, satisfaction);
        package.required_child_fee(rate, child_vsize)
    };

    let available: u64 = sats(&anchor) + spare.iter().map(sats).sum::<u64>();
    let mut utxos = vec![anchor];
    let mut total = sats(&utxos[0]);
    let fee_sats = loop {
        let fee = fee_for(&utxos)?;
        let needed = fee + dust_threshold;
        if total >= needed {
            break fee;
        }
        if spare.is_empty() {
            return Err(BumpFeeError::InsufficientFunds {
                message: format!(
                    "the CPFP child needs {needed} sats (fee plus a non-dust output) but the \
                     wallet holds only {available} sats in coins it can spend here"
                ),
            });
        }
        // The smallest coin that closes the gap on its own; failing that, the largest.
        let gap = needed - total;
        let pick = spare
            .iter()
            .position(|utxo| sats(utxo) >= gap)
            .unwrap_or(spare.len() - 1);
        let next = spare.remove(pick);
        total += sats(&next);
        utxos.push(next);
    };
    Ok(CpfpChildInputs { utxos, fee_sats })
}

fn sats(utxo: &bdk_wallet::LocalOutput) -> u64 {
    utxo.txout.value.to_sat()
}

/// The reveal output a CPFP child anchors on: the wallet-owned output of largest value
/// (F-007 — deterministic, and never a dust output while a bigger one is mine).
///
/// Shared by the listing (which prices the bump ceiling) and the build (which spends it),
/// so the figure the operator is offered is measured against the coin the bump will use.
fn reveal_anchor_vout(
    wallet: &bdk_wallet::Wallet,
    reveal_tx: &bdk_wallet::bitcoin::Transaction,
) -> Option<usize> {
    reveal_tx
        .output
        .iter()
        .enumerate()
        .filter(|(_, out)| wallet.is_mine(out.script_pubkey.clone()))
        .max_by_key(|(_, out)| out.value)
        .map(|(idx, _)| idx)
}

/// Highest package rate this governance package can be bumped to (#431), or `None` when
/// the reveal or the descriptor cannot be read.
///
/// Sized for the ordinary child — the anchor alone funds it — which is the shape the UI
/// quotes. When the fee forces a second coin in, the child grows and its ceiling rises
/// with it, so the figure below is the conservative one. The anchor's script stands in
/// for the child's change script: both are wallet outputs of the same type, so they
/// weigh the same.
fn max_cpfp_package_rate_sat_per_kvb(
    wallet: &bdk_wallet::Wallet,
    reveal_txid: &str,
    package: PackageStats,
) -> Option<u64> {
    let reveal_txid: bdk_wallet::bitcoin::Txid = reveal_txid.parse().ok()?;
    let reveal = wallet.get_tx(reveal_txid)?;
    let reveal_tx = reveal.tx_node.tx.as_ref();
    let anchor_script = &reveal_tx.output[reveal_anchor_vout(wallet, reveal_tx)?].script_pubkey;
    let satisfaction_wu = wallet
        .public_descriptor(bdk_wallet::KeychainKind::Internal)
        .max_weight_to_satisfy()
        .ok()?
        .to_wu();
    let child_vsize = cpfp_child_vsize(1, anchor_script, satisfaction_wu);
    Some(package.max_package_rate_sat_per_kvb(child_vsize))
}

/// Package fee/vsize for a governance commit, from the wallet graph. `None` when
/// the reveal (or either fee) is not yet known — the row then falls back to the
/// commit's own numbers and the bump errors with a clear message.
fn governance_package_stats(
    wallet: &bdk_wallet::Wallet,
    commit_tx: &bdk_wallet::bitcoin::Transaction,
    reveal_txid: &str,
) -> Option<PackageStats> {
    let reveal_txid: bdk_wallet::bitcoin::Txid = reveal_txid.parse().ok()?;
    let reveal = wallet.get_tx(reveal_txid)?;
    let reveal_tx = reveal.tx_node.tx.as_ref();
    let commit_fee = wallet.calculate_fee(commit_tx).ok()?.to_sat();
    let reveal_fee = wallet.calculate_fee(reveal_tx).ok()?.to_sat();
    Some(PackageStats {
        fee_sats: commit_fee + reveal_fee,
        vsize_vbytes: commit_tx.vsize() as u64 + reveal_tx.vsize() as u64,
    })
}

// ── BDK error mapping ───────────────────────────────────────────────────────

fn map_build_fee_bump_error(e: bdk_wallet::error::BuildFeeBumpError, txid: &str) -> BumpFeeError {
    use bdk_wallet::error::BuildFeeBumpError as E;
    let txid = txid.to_string();
    match e {
        E::TransactionNotFound(_) | E::UnknownUtxo(_) => BumpFeeError::TxNotFound { txid },
        E::TransactionConfirmed(_) => BumpFeeError::TxAlreadyConfirmed { txid },
        E::IrreplaceableTransaction(_) => BumpFeeError::TxNotReplaceable { txid },
        E::FeeRateUnavailable => BumpFeeError::BuildFailed {
            message: e.to_string(),
        },
    }
}

fn map_create_tx_error(e: bdk_wallet::error::CreateTxError) -> BumpFeeError {
    use bdk_wallet::error::CreateTxError as E;
    match e {
        E::FeeTooLow { required } => BumpFeeError::FeeTooLow {
            required_fee_sats: required.to_sat(),
        },
        E::FeeRateTooLow { required } => BumpFeeError::FeeRateTooLow {
            // sat/kwu → sat/kvB (1 vB = 4 WU)
            required_sat_per_kvb: required.to_sat_per_kwu().saturating_mul(4),
        },
        E::CoinSelection(inner) => BumpFeeError::InsufficientFunds {
            message: inner.to_string(),
        },
        other => BumpFeeError::BuildFailed {
            message: other.to_string(),
        },
    }
}

// ── Use-cases ───────────────────────────────────────────────────────────────

impl WalletService {
    /// Lists unconfirmed transactions that spend wallet-owned inputs ("sent from the
    /// Admin Wallet", PRD §4.3.3), newest-first by mempool last-seen.
    ///
    /// `pending_commit_to_reveal` maps governance commits to their pending pre-signed
    /// reveal (see [`crate::application::pending_reveals::pending_commit_to_reveal`]);
    /// those rows are flagged, offered CPFP as the bump method, and carry the
    /// commit+reveal **package** fee/vsize/rate when the reveal is in the graph.
    ///
    /// Pure read over the last-synced wallet state — no network I/O.
    pub async fn list_unconfirmed_sent_txs(
        &self,
        pending_commit_to_reveal: &HashMap<String, String>,
    ) -> Result<Vec<UnconfirmedTxDto>, AdminWalletError> {
        use bdk_wallet::chain::ChainPosition;

        let wallet = self.wallet.lock().await;
        let mut rows: Vec<UnconfirmedTxDto> = wallet
            .transactions()
            .filter_map(|wtx| {
                let last_seen_secs = match wtx.chain_position {
                    ChainPosition::Confirmed { .. } => return None,
                    ChainPosition::Unconfirmed { last_seen } => last_seen,
                };
                let tx = wtx.tx_node.tx.as_ref();
                let (sent, received) = wallet.sent_and_received(tx);
                if sent.to_sat() == 0 {
                    return None; // incoming-only — surfaced by balance/addresses (R1.5/R1.6)
                }
                let txid = wtx.tx_node.txid.to_string();
                let vsize_vbytes = tx.vsize() as u64;
                let fee_sats = wallet.calculate_fee(tx).ok().map(|fee| fee.to_sat());
                let is_rbf_signaling = tx.input.iter().any(|input| input.sequence.is_rbf());
                let pending_reveal = pending_commit_to_reveal.get(&txid);
                let package = pending_reveal
                    .and_then(|reveal_txid| governance_package_stats(&wallet, tx, reveal_txid));
                let bump_method = match (pending_reveal, is_rbf_signaling) {
                    (Some(_), _) => Some(BumpMethod::Cpfp),
                    (None, true) => Some(BumpMethod::Rbf),
                    (None, false) => None,
                };
                Some(UnconfirmedTxDto {
                    is_governance_commit: pending_reveal.is_some(),
                    bump_method,
                    package_fee_sats: package.map(|p| p.fee_sats),
                    package_vsize_vbytes: package.map(|p| p.vsize_vbytes),
                    package_fee_rate_sat_per_kvb: package
                        .map(|p| fee_rate_sat_per_kvb(p.fee_sats, p.vsize_vbytes)),
                    max_bump_rate_sat_per_kvb: package.zip(pending_reveal).and_then(
                        |(p, reveal_txid)| {
                            max_cpfp_package_rate_sat_per_kvb(&wallet, reveal_txid, p)
                        },
                    ),
                    txid,
                    sent_sats: sent.to_sat(),
                    received_sats: received.to_sat(),
                    net_sats: received.to_sat() as i64 - sent.to_sat() as i64,
                    fee_rate_sat_per_kvb: fee_sats
                        .map(|fee| fee_rate_sat_per_kvb(fee, vsize_vbytes)),
                    fee_sats,
                    vsize_vbytes,
                    is_rbf_signaling,
                    last_seen_secs,
                })
            })
            .collect();
        // F-012: Sort newest-first by last_seen, with stable fallback to txid when
        // last_seen is None (indexer didn't provide a timestamp). This ensures
        // deterministic ordering even when some txs lack mempool timestamps.
        rows.sort_by(|a, b| {
            b.last_seen_secs
                .cmp(&a.last_seen_secs)
                .then_with(|| a.txid.cmp(&b.txid))
        });
        Ok(rows)
    }

    /// Bumps the fee of an unconfirmed wallet transaction (PRD §4.3.3).
    ///
    /// Dispatch: a pending governance commit (key of `pending_commit_to_reveal`)
    /// is accelerated via **CPFP** on its reveal's change output; anything else is
    /// replaced via **RBF** (BDK `build_fee_bump`). Both paths sign through the
    /// session [`PsbtSigner`](crate::application::psbt_signer::PsbtSigner) port and
    /// broadcast Electrum-first with node fallback.
    ///
    /// Capability guards run **before** any wallet or network I/O. The caller is
    /// responsible for syncing the wallet beforehand (the IPC command does so
    /// best-effort); a stale view is ultimately caught by the node, which rejects
    /// replacements of confirmed or already-replaced transactions.
    pub async fn bump_fee(
        &self,
        txid: &str,
        new_rate: FeeRate,
        pending_commit_to_reveal: &HashMap<String, String>,
        broadcasters: &[Arc<dyn TxBroadcaster>],
    ) -> Result<BumpFeeResultDto, BumpFeeError> {
        // 1–2. Capability guards — before any wallet lock or network contact.
        let signer = self.signer().ok_or(BumpFeeError::ReadOnly)?;
        if !signer.allowed_on(self.network()) {
            return Err(BumpFeeError::SignerNotAllowedOnNetwork);
        }
        let parsed_txid: bdk_wallet::bitcoin::Txid =
            txid.parse().map_err(|_| BumpFeeError::InvalidTxid {
                txid: txid.to_string(),
            })?;

        // 3. Dispatch: governance commits are accelerated (CPFP), the rest replaced (RBF).
        let (psbt, package) = match pending_commit_to_reveal.get(txid) {
            Some(reveal_txid) => {
                let (psbt, package) = self
                    .build_cpfp_child_psbt(
                        txid,
                        parsed_txid,
                        reveal_txid,
                        new_rate,
                        pending_commit_to_reveal,
                    )
                    .await?;
                (psbt, Some(package))
            }
            None => (
                self.build_rbf_psbt(txid, parsed_txid, new_rate).await?,
                None,
            ),
        };

        // 4. Sign through the session signer port (same flow as commit funding, R1.1).
        let tx = self
            .sign_and_finalize_psbt(psbt)
            .await
            .map_err(|e| BumpFeeError::SignFailed {
                message: e.to_string(),
            })?;

        // 5. Result metadata — all prevouts are wallet-known, so the fee is exact.
        let new_txid = tx.compute_txid().to_string();
        let fee_sats = {
            let wallet = self.wallet.lock().await;
            wallet
                .calculate_fee(&tx)
                .map(|fee| fee.to_sat())
                .map_err(|e| BumpFeeError::BuildFailed {
                    message: format!("bump fee unknown: {e}"),
                })?
        };
        let vsize_vbytes = tx.vsize() as u64;

        // F-006: the realized package rate, measured on the *signed* child — the only size
        // the network will ever see. Checked here rather than against the estimate, so the
        // guard compares against reality instead of re-deriving the arithmetic that priced
        // the fee, and it runs before the broadcast so a short package is never published.
        if let Some(p) = package {
            if let Some(realized) = package_rate_shortfall(p, fee_sats, vsize_vbytes, new_rate) {
                return Err(BumpFeeError::FeeRateTooLow {
                    required_sat_per_kvb: realized,
                });
            }
        }

        // 6. Broadcast: Electrum first, node RPC fallback.
        let tx_hex = bdk_wallet::bitcoin::consensus::encode::serialize_hex(&tx);
        broadcast_single_with_fallback(broadcasters, &tx_hex)
            .await
            .map_err(|errors| BumpFeeError::BroadcastFailed {
                message: errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            })?;

        // CPFP reports the resulting package rate (what miners evaluate); RBF the
        // replacement's own rate.
        let (fee_rate, method) = match package {
            Some(p) => (
                fee_rate_sat_per_kvb(p.fee_sats + fee_sats, p.vsize_vbytes + vsize_vbytes),
                BumpMethod::Cpfp,
            ),
            None => (
                fee_rate_sat_per_kvb(fee_sats, vsize_vbytes),
                BumpMethod::Rbf,
            ),
        };
        Ok(BumpFeeResultDto {
            new_txid,
            target_txid: txid.to_string(),
            fee_rate_sat_per_kvb: fee_rate,
            fee_sats,
            method,
            sync_warning: None,
        })
    }

    /// RBF path: replacement PSBT via BDK `build_fee_bump` (rejects confirmed /
    /// non-RBF / unknown txids).
    async fn build_rbf_psbt(
        &self,
        txid: &str,
        parsed_txid: bdk_wallet::bitcoin::Txid,
        new_rate: FeeRate,
    ) -> Result<bdk_wallet::bitcoin::Psbt, BumpFeeError> {
        let mut wallet = self.wallet.lock().await;
        let mut builder = wallet
            .build_fee_bump(parsed_txid)
            .map_err(|e| map_build_fee_bump_error(e, txid))?;
        builder.fee_rate(new_rate.to_bdk());
        builder.finish().map_err(map_create_tx_error)
    }

    /// CPFP path: child PSBT spending the reveal's wallet-owned change output with
    /// an absolute fee that lifts the `commit+reveal+child` package to `new_rate`.
    ///
    /// `pending_commit_to_reveal` is read to keep other pending packages' anchors out of
    /// the funding pool — spending one would leave that bundle impossible to accelerate.
    async fn build_cpfp_child_psbt(
        &self,
        txid: &str,
        commit_txid: bdk_wallet::bitcoin::Txid,
        reveal_txid: &str,
        new_rate: FeeRate,
        pending_commit_to_reveal: &HashMap<String, String>,
    ) -> Result<(bdk_wallet::bitcoin::Psbt, PackageStats), BumpFeeError> {
        use bdk_wallet::bitcoin::{Amount, OutPoint};
        use bdk_wallet::chain::ChainPosition;

        let unavailable = |message: String| BumpFeeError::CpfpOutputUnavailable {
            txid: txid.to_string(),
            message,
        };

        let mut wallet = self.wallet.lock().await;
        let commit = wallet
            .get_tx(commit_txid)
            .ok_or_else(|| BumpFeeError::TxNotFound {
                txid: txid.to_string(),
            })?;
        if matches!(commit.chain_position, ChainPosition::Confirmed { .. }) {
            return Err(BumpFeeError::TxAlreadyConfirmed {
                txid: txid.to_string(),
            });
        }
        let commit_tx = bdk_wallet::bitcoin::Transaction::clone(&commit.tx_node.tx);

        let reveal_parsed: bdk_wallet::bitcoin::Txid = reveal_txid
            .parse()
            .map_err(|_| unavailable(format!("'{reveal_txid}' is not a valid reveal txid")))?;
        let reveal = wallet.get_tx(reveal_parsed).ok_or_else(|| {
            unavailable(
                "the reveal transaction is not in the wallet yet — sync and retry".to_string(),
            )
        })?;
        let reveal_tx = bdk_wallet::bitcoin::Transaction::clone(&reveal.tx_node.tx);

        // F-007: the largest wallet-owned output, same criterion the listing prices against.
        let vout = reveal_anchor_vout(&wallet, &reveal_tx).ok_or_else(|| {
            unavailable("the reveal pays no change back to the admin wallet".to_string())
        })?;
        let anchor = OutPoint {
            txid: reveal_parsed,
            vout: vout as u32,
        };
        let anchor_utxo = wallet.get_utxo(anchor).ok_or_else(|| {
            unavailable(
                "the reveal change is already spent — bump the existing child transaction instead"
                    .to_string(),
            )
        })?;

        let package = governance_package_stats(&wallet, &commit_tx, reveal_txid)
            .ok_or_else(|| unavailable("the package fee is not known to the wallet".to_string()))?;

        let drain_script = wallet
            .reveal_next_address(bdk_wallet::KeychainKind::Internal)
            .address
            .script_pubkey();

        // Witness cost per keychain, straight from the descriptor.
        let satisfaction_wu = |keychain| -> Result<u64, BumpFeeError> {
            wallet
                .public_descriptor(keychain)
                .max_weight_to_satisfy()
                .map(|weight| weight.to_wu())
                .map_err(|e| BumpFeeError::BuildFailed {
                    message: format!("cannot size the CPFP child: {e}"),
                })
        };
        let external_wu = satisfaction_wu(bdk_wallet::KeychainKind::External)?;
        let internal_wu = satisfaction_wu(bdk_wallet::KeychainKind::Internal)?;
        let witness_wu = |keychain| match keychain {
            bdk_wallet::KeychainKind::External => external_wu,
            bdk_wallet::KeychainKind::Internal => internal_wu,
        };

        // What the child may spend besides the anchor. Three exclusions, each load-bearing:
        //
        // - **Immature coinbase.** `manually_selected_only` skips BDK's `filter_utxos`, and
        //   with it the maturity check, so nothing downstream would stop the child from
        //   spending a coinbase the node will reject as `premature-spend-of-coinbase`. The
        //   wallet's own balance already excludes these, so spending them would also mean
        //   spending money the UI says the operator does not have.
        // - **Unconfirmed coins from outside this package.** Their parents would join the
        //   child's mempool ancestor set, which `governance_package_stats` does not account
        //   for, so the package rate reported to the user would be higher than the one a
        //   miner computes. The commit's and reveal's own outputs are exempt: those two are
        //   already inside the accounted package, which is what keeps the #431 case working
        //   on a fully unconfirmed wallet.
        // - **Other pending packages' anchors** would leave those bundles impossible to
        //   accelerate, reporting "the reveal change is already spent" with no child to
        //   bump. Redundant while the rule above holds — a pending package's anchor is by
        //   definition unconfirmed and outside this package, so it is already excluded —
        //   and kept as the safety net for the day that rule is relaxed.
        let tip_height = wallet.latest_checkpoint().height();
        let other_anchors: std::collections::HashSet<bdk_wallet::bitcoin::Txid> =
            pending_commit_to_reveal
                .values()
                .filter_map(|reveal| reveal.parse::<bdk_wallet::bitcoin::Txid>().ok())
                .filter(|txid| *txid != reveal_parsed)
                .collect();
        let spare: Vec<bdk_wallet::LocalOutput> = wallet
            .list_unspent()
            .filter(|utxo| utxo.outpoint != anchor)
            .filter(|utxo| !other_anchors.contains(&utxo.outpoint.txid))
            .filter(|utxo| match &utxo.chain_position {
                ChainPosition::Confirmed { anchor, .. } => {
                    let is_coinbase = wallet
                        .get_tx(utxo.outpoint.txid)
                        .is_some_and(|tx| tx.tx_node.tx.is_coinbase());
                    !is_coinbase
                        || tip_height.saturating_sub(anchor.block_id.height) + 1
                            >= bdk_wallet::bitcoin::constants::COINBASE_MATURITY
                }
                ChainPosition::Unconfirmed { .. } => {
                    utxo.outpoint.txid == commit_txid || utxo.outpoint.txid == reveal_parsed
                }
            })
            .collect();
        // Everything the child may spend, against everything the wallet holds. When they
        // differ the operator is looking at a balance the acceleration cannot touch, and
        // saying "your balance is too small" would be false — that is a separate error.
        let eligible_sats: u64 = sats(&anchor_utxo) + spare.iter().map(sats).sum::<u64>();
        let wallet_total_sats = wallet.balance().total().to_sat();

        let child = select_cpfp_child_inputs(
            anchor_utxo,
            spare,
            &drain_script,
            witness_wu,
            package,
            new_rate,
        )
        .map_err(|e| match e {
            BumpFeeError::InsufficientFunds { message } if wallet_total_sats > eligible_sats => {
                BumpFeeError::CpfpFundingUnavailable {
                    message: format!(
                        "{message}. The rest of the balance cannot fund this acceleration: \
                         newly mined coins, unconfirmed coins from other transactions, and \
                         coins reserved by other pending proposals are all excluded"
                    ),
                }
            }
            other => other,
        })?;
        let child_fee = child.fee_sats;

        // Selecting explicitly keeps BDK's coin selection out of the picture, so the
        // child is exactly the transaction that was sized and priced above.
        let mut builder = wallet.build_tx();
        builder.manually_selected_only();
        for utxo in &child.utxos {
            builder
                .add_utxo(utxo.outpoint)
                .map_err(|e| unavailable(e.to_string()))?;
        }
        builder.drain_to(drain_script);
        builder.fee_absolute(Amount::from_sat(child_fee));
        let psbt = builder.finish().map_err(map_create_tx_error)?;

        // Sanity check on the arithmetic above: the child BDK built must be the one that
        // was priced. This is not F-006 — it cannot fail unless the builder starts adding
        // or dropping inputs behind our back, and F-006 itself is enforced on the *signed*
        // child in `bump_fee`, which is the size the network charges for.
        let modelled = cpfp_child_vsize(
            child.utxos.len() as u64,
            &psbt.unsigned_tx.output[0].script_pubkey,
            child.satisfaction_wu(witness_wu),
        );
        let built = signed_child_vsize(&psbt.unsigned_tx, child.satisfaction_wu(witness_wu));
        if built > modelled {
            return Err(BumpFeeError::BuildFailed {
                message: format!("the child was priced for {modelled} vB but built to {built} vB"),
            });
        }

        Ok((psbt, package))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::admin_wallet::load_admin_wallet;
    use crate::infrastructure::node_config_store::NodeConfig;
    use bdk_wallet::bitcoin::hashes::Hash;
    use bdk_wallet::bitcoin::{
        absolute, transaction, Amount, BlockHash, Network, OutPoint, ScriptBuf, Sequence,
        Transaction, TxIn, TxOut, Txid,
    };
    use bdk_wallet::chain::BlockId;
    use bdk_wallet::test_utils::{
        insert_anchor, insert_checkpoint, insert_seen_at, insert_tx, receive_output,
        receive_output_in_latest_block, ReceiveTo,
    };
    use std::sync::RwLock as StdRwLock;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn test_node_config() -> Arc<StdRwLock<NodeConfig>> {
        Arc::new(StdRwLock::new(NodeConfig::default()))
    }

    /// An arbitrary P2WPKH script that does not belong to the test wallet.
    fn external_script() -> ScriptBuf {
        ScriptBuf::new_p2wpkh(&bdk_wallet::bitcoin::WPubkeyHash::from_byte_array(
            [0x42; 20],
        ))
    }

    /// Admin wallet with one confirmed UTXO per requested amount.
    fn funded_wallet_with(amounts: &[u64]) -> bdk_wallet::Wallet {
        let mut wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        insert_checkpoint(
            &mut wallet,
            BlockId {
                height: 1_000,
                hash: BlockHash::all_zeros(),
            },
        );
        for amount in amounts {
            receive_output_in_latest_block(&mut wallet, *amount);
        }
        wallet
    }

    /// Admin wallet with one confirmed 100_000-sat UTXO at external index 0.
    fn funded_wallet() -> bdk_wallet::Wallet {
        funded_wallet_with(&[100_000])
    }

    /// Spends every wallet UTXO except `keep` to an external script, leaving the
    /// CPFP anchor as the only spendable coin.
    fn spend_all_except(wallet: &mut bdk_wallet::Wallet, keep: OutPoint) {
        spend_all_except_any(wallet, &[keep]);
    }

    /// `spend_all_except` for scenarios that must preserve more than one coin.
    fn spend_all_except_any(wallet: &mut bdk_wallet::Wallet, keep: &[OutPoint]) {
        let others: Vec<OutPoint> = wallet
            .list_unspent()
            .map(|utxo| utxo.outpoint)
            .filter(|outpoint| !keep.contains(outpoint))
            .collect();
        assert!(!others.is_empty(), "nothing to spend away");
        let mut builder = wallet.build_tx();
        for outpoint in others {
            builder.add_utxo(outpoint).expect("utxo must be unspent");
        }
        builder.manually_selected_only();
        builder.drain_to(external_script());
        let mut psbt = builder.finish().expect("build sweep");
        assert!(
            wallet
                .sign(&mut psbt, bdk_wallet::SignOptions::default())
                .expect("sign sweep"),
            "sweep must finalize"
        );
        let tx = psbt.extract_tx().expect("extract sweep");
        insert_tx(wallet, tx.clone());
        insert_seen_at(wallet, tx.compute_txid(), 4_000_000_500);
    }

    /// Builds, signs, and inserts an unconfirmed spend of the wallet's funds.
    fn insert_unconfirmed_spend(
        wallet: &mut bdk_wallet::Wallet,
        rbf: bool,
        seen_at: u64,
    ) -> Transaction {
        let mut builder = wallet.build_tx();
        builder.add_recipient(external_script(), Amount::from_sat(40_000));
        if !rbf {
            builder.set_exact_sequence(Sequence::MAX);
        }
        let mut psbt = builder.finish().expect("build spend");
        let finalized = wallet
            .sign(&mut psbt, bdk_wallet::SignOptions::default())
            .expect("sign spend");
        assert!(finalized, "test spend must finalize");
        let tx = psbt.extract_tx().expect("extract spend");
        insert_tx(wallet, tx.clone());
        insert_seen_at(wallet, tx.compute_txid(), seen_at);
        tx
    }

    /// Inserts an unconfirmed governance `commit → reveal` pair: the commit funds a
    /// 20_000-sat envelope output; the reveal spends it into an OP_RETURN action tag
    /// plus a 19_700-sat change output back to the wallet's internal keychain
    /// (mirrors `broadcast_tx.rs::build_reveal_tx`). Returns `(commit, reveal)`.
    fn insert_governance_package(
        wallet: &mut bdk_wallet::Wallet,
        seen_at: u64,
    ) -> (Transaction, Transaction) {
        insert_governance_package_with_change(wallet, seen_at, 19_700)
    }

    /// As [`insert_governance_package`], with the reveal paying `reveal_change_sats`
    /// back to the wallet — the amount that decides whether the CPFP child's leftover
    /// falls inside the dust window (#431).
    fn insert_governance_package_with_change(
        wallet: &mut bdk_wallet::Wallet,
        seen_at: u64,
        reveal_change_sats: u64,
    ) -> (Transaction, Transaction) {
        insert_governance_package_with_changes(wallet, seen_at, &[reveal_change_sats])
    }

    /// As [`insert_governance_package`], with one wallet-owned reveal output per
    /// requested amount (F-007 needs a reveal with several own outputs).
    ///
    /// The envelope is sized as `changes + REVEAL_FEE_SATS`, mirroring production
    /// (`proposals.rs`: `commit_amount = COMMIT_DUST_SATS + reveal_fee`), so shrinking
    /// the reveal change does not silently inflate the reveal fee.
    fn insert_governance_package_with_changes(
        wallet: &mut bdk_wallet::Wallet,
        seen_at: u64,
        reveal_change_sats: &[u64],
    ) -> (Transaction, Transaction) {
        /// Reveal fee baked into the fixture's envelope output.
        const REVEAL_FEE_SATS: u64 = 300;

        let envelope_sats = reveal_change_sats.iter().sum::<u64>() + REVEAL_FEE_SATS;
        let envelope_script = external_script();
        let mut builder = wallet.build_tx();
        builder.add_recipient(envelope_script.clone(), Amount::from_sat(envelope_sats));
        let mut psbt = builder.finish().expect("build commit");
        let finalized = wallet
            .sign(&mut psbt, bdk_wallet::SignOptions::default())
            .expect("sign commit");
        assert!(finalized, "commit must finalize");
        let commit = psbt.extract_tx().expect("extract commit");
        insert_tx(wallet, commit.clone());
        insert_seen_at(wallet, commit.compute_txid(), seen_at);

        let envelope_vout = commit
            .output
            .iter()
            .position(|out| out.script_pubkey == envelope_script)
            .expect("envelope output present") as u32;
        let mut output = vec![TxOut {
            value: Amount::ZERO,
            script_pubkey: ScriptBuf::new_op_return(b"sps50-action"),
        }];
        for change_sats in reveal_change_sats {
            output.push(TxOut {
                value: Amount::from_sat(*change_sats),
                script_pubkey: wallet
                    .reveal_next_address(bdk_wallet::KeychainKind::Internal)
                    .address
                    .script_pubkey(),
            });
        }
        let reveal = Transaction {
            version: transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: commit.compute_txid(),
                    vout: envelope_vout,
                },
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                ..Default::default()
            }],
            output,
        };
        insert_tx(wallet, reveal.clone());
        insert_seen_at(wallet, reveal.compute_txid(), seen_at + 1);
        (commit, reveal)
    }

    /// `pending_map` when the commit txid is already a string in hand.
    fn pending_map_from(commit_txid: &str, reveal: &Transaction) -> HashMap<String, String> {
        [(commit_txid.to_string(), reveal.compute_txid().to_string())].into()
    }

    fn pending_map(commit: &Transaction, reveal: &Transaction) -> HashMap<String, String> {
        [(
            commit.compute_txid().to_string(),
            reveal.compute_txid().to_string(),
        )]
        .into()
    }

    // ── list_unconfirmed_sent_txs ───────────────────────────────────────────

    #[tokio::test]
    async fn list_returns_empty_on_fresh_wallet() {
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashMap::new())
            .await
            .expect("list ok");

        assert!(rows.is_empty(), "fresh wallet must list no transactions");
    }

    #[tokio::test]
    async fn list_excludes_confirmed_transactions() {
        let svc = WalletService::new(funded_wallet(), test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashMap::new())
            .await
            .expect("list ok");

        assert!(
            rows.is_empty(),
            "a wallet with only a confirmed funding tx must list nothing"
        );
    }

    #[tokio::test]
    async fn list_returns_unconfirmed_spend_with_fee_and_rbf_flag() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashMap::new())
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.txid, tx.compute_txid().to_string());
        assert!(
            row.sent_sats > 0,
            "spend must report wallet-owned input sats"
        );
        assert!(row.net_sats < 0, "a send must have negative net");
        let fee = row
            .fee_sats
            .expect("fee must be known for a wallet-built spend");
        assert!(fee > 0);
        assert_eq!(
            row.fee_rate_sat_per_kvb.expect("rate known"),
            fee_rate_sat_per_kvb(fee, row.vsize_vbytes)
        );
        assert!(row.is_rbf_signaling, "BDK default sequence must signal RBF");
        assert!(!row.is_governance_commit);
        assert_eq!(row.bump_method, Some(BumpMethod::Rbf));
        assert_eq!(row.package_fee_sats, None);
        assert_eq!(
            row.max_bump_rate_sat_per_kvb, None,
            "an RBF row pays its own rate — only the general ceiling applies"
        );
        assert_eq!(row.last_seen_secs, Some(4_000_000_100));
    }

    #[tokio::test]
    async fn list_excludes_incoming_only_unconfirmed_tx() {
        let mut wallet = funded_wallet();
        receive_output(&mut wallet, 5_000, ReceiveTo::Mempool(4_000_000_200));
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashMap::new())
            .await
            .expect("list ok");

        assert!(
            rows.is_empty(),
            "incoming-only unconfirmed txs are not 'sent from the Admin Wallet'"
        );
    }

    #[tokio::test]
    async fn list_flags_pending_governance_commit_with_cpfp_and_package_stats() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package(&mut wallet, 4_000_000_100);
        let commit_fee = wallet.calculate_fee(&commit).expect("commit fee").to_sat();
        let reveal_fee = wallet.calculate_fee(&reveal).expect("reveal fee").to_sat();
        let pending = pending_map(&commit, &reveal);
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&pending)
            .await
            .expect("list ok");

        assert_eq!(
            rows.len(),
            1,
            "the reveal (incoming-only) must not be listed"
        );
        let row = &rows[0];
        assert!(row.is_governance_commit);
        assert_eq!(row.bump_method, Some(BumpMethod::Cpfp));
        assert_eq!(row.package_fee_sats, Some(commit_fee + reveal_fee));
        let package_vsize = commit.vsize() as u64 + reveal.vsize() as u64;
        assert_eq!(row.package_vsize_vbytes, Some(package_vsize));
        assert_eq!(
            row.package_fee_rate_sat_per_kvb,
            Some(fee_rate_sat_per_kvb(commit_fee + reveal_fee, package_vsize))
        );
        // #431: the row carries the ceiling its own child can honour, priced for the
        // ordinary 111 vB child.
        assert_eq!(
            row.max_bump_rate_sat_per_kvb,
            Some(
                PackageStats {
                    fee_sats: commit_fee + reveal_fee,
                    vsize_vbytes: package_vsize,
                }
                .max_package_rate_sat_per_kvb(111)
            )
        );
    }

    #[tokio::test]
    async fn list_governance_commit_without_reveal_in_graph_has_no_package_stats() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let svc = WalletService::new(wallet, test_node_config());
        // Reveal txid valid in shape, but never inserted into the wallet graph.
        let pending: HashMap<String, String> = [(
            tx.compute_txid().to_string(),
            Txid::from_byte_array([0xCD; 32]).to_string(),
        )]
        .into();

        let rows = svc
            .list_unconfirmed_sent_txs(&pending)
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!(row.is_governance_commit);
        assert_eq!(
            row.bump_method,
            Some(BumpMethod::Cpfp),
            "CPFP is still offered; the bump itself reports the missing reveal"
        );
        assert_eq!(row.package_fee_sats, None);
        assert_eq!(row.package_vsize_vbytes, None);
        assert_eq!(row.package_fee_rate_sat_per_kvb, None);
        assert_eq!(
            row.max_bump_rate_sat_per_kvb, None,
            "without package stats there is no ceiling to quote"
        );
    }

    #[tokio::test]
    async fn list_reports_non_rbf_spend_as_not_signaling() {
        let mut wallet = funded_wallet();
        insert_unconfirmed_spend(&mut wallet, false, 4_000_000_100);
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashMap::new())
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 1);
        assert!(
            !rows[0].is_rbf_signaling,
            "MAX sequence on every input must report is_rbf_signaling=false"
        );
        assert_eq!(
            rows[0].bump_method, None,
            "non-RBF non-governance txs offer no bump method"
        );
    }

    #[tokio::test]
    async fn list_reports_unknown_fee_as_none_for_foreign_input() {
        let mut wallet = funded_wallet();
        let wallet_outpoint = receive_output_in_latest_block(&mut wallet, 20_000);
        let foreign_outpoint = OutPoint {
            txid: Txid::from_byte_array([0xAB; 32]),
            vout: 0,
        };
        let tx = Transaction {
            version: transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![
                TxIn {
                    previous_output: wallet_outpoint,
                    sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                    ..Default::default()
                },
                TxIn {
                    previous_output: foreign_outpoint,
                    sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                    ..Default::default()
                },
            ],
            output: vec![TxOut {
                script_pubkey: external_script(),
                value: Amount::from_sat(10_000),
            }],
        };
        let txid = tx.compute_txid();
        insert_tx(&mut wallet, tx);
        insert_seen_at(&mut wallet, txid, 4_000_000_300);
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashMap::new())
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 1, "tx spending a wallet input must be listed");
        assert!(
            rows[0].fee_sats.is_none(),
            "foreign prevout makes the fee unknown"
        );
        assert!(rows[0].fee_rate_sat_per_kvb.is_none());
    }

    #[tokio::test]
    async fn list_sorts_newest_first_by_last_seen() {
        // Build from scratch to capture confirmed outpoints for explicit UTXO selection.
        // Without it, BDK's coin-selection sometimes picks the change from `older` as an
        // input for `newer`, making `newer` a child of `older`. The canonical iterator then
        // assigns `older` the `last_seen` of `newer` (4_000_000_900) transitively, causing
        // both rows to tie on `last_seen` and the txid fallback to produce a wrong order.
        let mut wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        insert_checkpoint(
            &mut wallet,
            BlockId {
                height: 1_000,
                hash: BlockHash::all_zeros(),
            },
        );
        let utxo_older = receive_output_in_latest_block(&mut wallet, 100_000);
        let utxo_newer = receive_output_in_latest_block(&mut wallet, 50_000);

        let older = {
            let mut builder = wallet.build_tx();
            builder.add_recipient(external_script(), Amount::from_sat(40_000));
            builder
                .add_utxo(utxo_older)
                .expect("utxo_older must be unspent");
            builder.manually_selected_only();
            let mut psbt = builder.finish().expect("build older spend");
            assert!(
                wallet
                    .sign(&mut psbt, bdk_wallet::SignOptions::default())
                    .expect("sign older"),
                "older must finalize"
            );
            let tx = psbt.extract_tx().expect("extract older");
            insert_tx(&mut wallet, tx.clone());
            insert_seen_at(&mut wallet, tx.compute_txid(), 4_000_000_100);
            tx
        };

        let newer = {
            let mut builder = wallet.build_tx();
            builder.add_recipient(external_script(), Amount::from_sat(40_000));
            builder
                .add_utxo(utxo_newer)
                .expect("utxo_newer must be unspent");
            builder.manually_selected_only();
            let mut psbt = builder.finish().expect("build newer spend");
            assert!(
                wallet
                    .sign(&mut psbt, bdk_wallet::SignOptions::default())
                    .expect("sign newer"),
                "newer must finalize"
            );
            let tx = psbt.extract_tx().expect("extract newer");
            insert_tx(&mut wallet, tx.clone());
            insert_seen_at(&mut wallet, tx.compute_txid(), 4_000_000_900);
            tx
        };

        let svc = WalletService::new(wallet, test_node_config());
        let rows = svc
            .list_unconfirmed_sent_txs(&HashMap::new())
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].last_seen_secs,
            Some(4_000_000_900),
            "newer must be first (highest last_seen)"
        );
        assert_eq!(
            rows[1].last_seen_secs,
            Some(4_000_000_100),
            "older must be second"
        );
        assert_eq!(rows[0].txid, newer.compute_txid().to_string());
        assert_eq!(rows[1].txid, older.compute_txid().to_string());
    }

    // ── bump_fee ────────────────────────────────────────────────────────────

    use crate::application::psbt_signer::MnemonicPsbtSigner;
    use crate::application::tx_broadcaster::tests::MockBroadcaster;

    /// 5 sat/vB — strictly above the 1 sat/vB default the fixture spends use.
    fn higher_rate() -> FeeRate {
        FeeRate::new(5_000, 1_000).expect("valid rate")
    }

    fn signing_service(wallet: bdk_wallet::Wallet) -> WalletService {
        WalletService::with_signer(
            wallet,
            Arc::new(MnemonicPsbtSigner::new()),
            test_node_config(),
        )
    }

    fn mock_chain(mocks: &[Arc<MockBroadcaster>]) -> Vec<Arc<dyn TxBroadcaster>> {
        mocks.iter().map(|m| Arc::clone(m) as _).collect()
    }

    #[tokio::test]
    async fn bump_fee_on_watch_only_returns_read_only_before_any_broadcast() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let svc = WalletService::new_watch_only(wallet, test_node_config());
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &tx.compute_txid().to_string(),
                higher_rate(),
                &HashMap::new(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::ReadOnly)),
            "got: {result:?}"
        );
        assert!(
            mock.sent_single().is_empty(),
            "no broadcaster may be contacted"
        );
    }

    #[tokio::test]
    async fn bump_fee_unknown_txid_returns_tx_not_found() {
        let svc = signing_service(funded_wallet());

        let result = svc
            .bump_fee(
                &Txid::from_byte_array([0x11; 32]).to_string(),
                higher_rate(),
                &HashMap::new(),
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::TxNotFound { .. })),
            "got: {result:?}"
        );
    }

    #[tokio::test]
    async fn bump_fee_invalid_txid_returns_invalid_txid() {
        let svc = signing_service(funded_wallet());

        let result = svc
            .bump_fee(
                "not-a-txid",
                higher_rate(),
                &HashMap::new(),
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::InvalidTxid { .. })),
            "got: {result:?}"
        );
    }

    #[tokio::test]
    async fn bump_fee_confirmed_tx_returns_already_confirmed() {
        let wallet = funded_wallet();
        // The confirmed funding tx is in the graph — bumping it must be rejected.
        let funding_txid = wallet
            .transactions()
            .next()
            .expect("funding tx present")
            .tx_node
            .txid
            .to_string();
        let svc = signing_service(wallet);

        let result = svc
            .bump_fee(
                &funding_txid,
                higher_rate(),
                &HashMap::new(),
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::TxAlreadyConfirmed { .. })),
            "got: {result:?}"
        );
    }

    #[tokio::test]
    async fn bump_fee_non_rbf_tx_returns_not_replaceable() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, false, 4_000_000_100);
        let svc = signing_service(wallet);

        let result = svc
            .bump_fee(
                &tx.compute_txid().to_string(),
                higher_rate(),
                &HashMap::new(),
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::TxNotReplaceable { .. })),
            "got: {result:?}"
        );
    }

    // ── bump_fee — CPFP path (governance commits) ───────────────────────────

    #[tokio::test]
    async fn bump_fee_governance_commit_broadcasts_cpfp_child() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package(&mut wallet, 4_000_000_100);
        let commit_txid = commit.compute_txid().to_string();
        let reveal_txid = reveal.compute_txid();
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &commit_txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await
            .expect("cpfp bump must succeed");

        assert_eq!(result.method, BumpMethod::Cpfp);
        assert_eq!(result.target_txid, commit_txid);
        assert_ne!(result.new_txid, commit_txid, "child must be a new tx");
        assert!(result.fee_sats > 0, "child must pay a fee");
        assert!(
            result.fee_rate_sat_per_kvb >= 5_000 - 500,
            "resulting package rate {} must approach the requested 5 sat/vB",
            result.fee_rate_sat_per_kvb
        );

        let sent = mock.sent_single();
        assert_eq!(sent.len(), 1, "exactly the child must be broadcast");
        let child: Transaction = bdk_wallet::bitcoin::consensus::encode::deserialize_hex(&sent[0])
            .expect("broadcast hex decodes");
        assert_eq!(child.compute_txid().to_string(), result.new_txid);
        assert!(
            child
                .input
                .iter()
                .any(|input| input.previous_output.txid == reveal_txid),
            "child must anchor on the reveal's change output"
        );
        // F-006: the size the fee was computed against is the size the network sees.
        assert_eq!(
            child.vsize() as u64,
            111,
            "a signed 1-in/1-out taproot child measures 111 vB"
        );
    }

    #[tokio::test]
    async fn bump_fee_governance_rate_not_above_package_returns_fee_rate_too_low() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package(&mut wallet, 4_000_000_100);
        let commit_txid = commit.compute_txid().to_string();
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        // 1 sat/vB — at or below the package's current effective rate.
        let same_rate = FeeRate::new(1_000, 1_000).expect("valid rate");

        let result = svc
            .bump_fee(
                &commit_txid,
                same_rate,
                &pending,
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        match result {
            Err(BumpFeeError::FeeRateTooLow {
                required_sat_per_kvb,
            }) => {
                assert!(
                    required_sat_per_kvb > 1_000,
                    "required package rate {required_sat_per_kvb} must exceed the requested one"
                );
            }
            other => panic!("expected FeeRateTooLow, got: {other:?}"),
        }
    }

    /// #431: the PRD ceiling is not reachable on a CPFP row — the child would have to pay
    /// three times the requested rate. The operator must be told so before anything is
    /// signed, instead of meeting rust-bitcoin's `AbsurdFeeRate` at the signing step.
    #[tokio::test]
    async fn bump_fee_governance_at_the_prd_ceiling_returns_fee_rate_too_high_without_broadcasting()
    {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package(&mut wallet, 4_000_000_100);
        let commit_txid = commit.compute_txid().to_string();
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));
        let prd_ceiling = FeeRate::new(MAX_BROADCAST_SAT_PER_KVB, 1_000).expect("valid rate");

        let result = svc
            .bump_fee(
                &commit_txid,
                prd_ceiling,
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        match result {
            Err(BumpFeeError::FeeRateTooHigh {
                max_sat_per_kvb,
                child_sat_per_kvb,
            }) => {
                assert!(
                    child_sat_per_kvb > MAX_BROADCAST_SAT_PER_KVB,
                    "the rejection must quote a child rate over the ceiling, got {child_sat_per_kvb}"
                );
                assert!(
                    max_sat_per_kvb < MAX_BROADCAST_SAT_PER_KVB,
                    "the package ceiling {max_sat_per_kvb} must sit below the requested rate"
                );
            }
            other => panic!("expected FeeRateTooHigh, got: {other:?}"),
        }
        assert!(
            mock.sent_single().is_empty(),
            "nothing may reach the network once the rate is refused"
        );
    }

    /// The ceiling the listing advertises must be one the bump honours: the two size the
    /// child from the same model, so a bump at exactly `maxBumpRateSatPerKvb` succeeds.
    ///
    /// Funded far above the usual fixture on purpose — at the ceiling the child pays
    /// 10,000 sat/vB of its own, so the anchor has to carry over a million sats. A wallet
    /// that cannot afford it hits `InsufficientFunds` first, which is a different (and
    /// correct) refusal and would say nothing about the ceiling.
    #[tokio::test]
    async fn bump_fee_at_the_advertised_ceiling_succeeds() {
        let mut wallet = funded_wallet_with(&[3_000_000]);
        let (commit, reveal) =
            insert_governance_package_with_change(&mut wallet, 4_000_000_100, 2_000_000);
        let commit_txid = commit.compute_txid().to_string();
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let rows = svc
            .list_unconfirmed_sent_txs(&pending)
            .await
            .expect("list ok");
        let row = rows
            .iter()
            .find(|row| row.txid == commit_txid)
            .expect("the commit must be listed");
        let advertised = row
            .max_bump_rate_sat_per_kvb
            .expect("a CPFP row must carry its ceiling");
        assert!(
            advertised < MAX_BROADCAST_SAT_PER_KVB,
            "the ceiling {advertised} must be below the general one to be worth advertising"
        );

        let result = svc
            .bump_fee(
                &commit_txid,
                FeeRate::new(advertised, 1_000).expect("valid rate"),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await
            .expect("a bump at the advertised ceiling must succeed");

        assert_eq!(result.method, BumpMethod::Cpfp);
        assert_eq!(
            mock.sent_single().len(),
            1,
            "the child must reach the network"
        );
    }

    #[tokio::test]
    async fn bump_fee_governance_reveal_missing_from_graph_returns_cpfp_unavailable() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let txid = tx.compute_txid().to_string();
        let svc = signing_service(wallet);
        let pending: HashMap<String, String> =
            [(txid.clone(), Txid::from_byte_array([0xCD; 32]).to_string())].into();
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::CpfpOutputUnavailable { .. })),
            "got: {result:?}"
        );
        assert!(mock.sent_single().is_empty(), "nothing may be broadcast");
    }

    #[tokio::test]
    async fn bump_fee_governance_reveal_change_already_spent_returns_cpfp_unavailable() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package(&mut wallet, 4_000_000_100);
        let commit_txid = commit.compute_txid().to_string();
        let pending = pending_map(&commit, &reveal);
        // A prior child already consumed the reveal's change output (vout 1).
        let prior_child = Transaction {
            version: transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: reveal.compute_txid(),
                    vout: 1,
                },
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                ..Default::default()
            }],
            output: vec![TxOut {
                value: Amount::from_sat(18_000),
                script_pubkey: external_script(),
            }],
        };
        let prior_child_txid = prior_child.compute_txid();
        insert_tx(&mut wallet, prior_child);
        insert_seen_at(&mut wallet, prior_child_txid, 4_000_000_200);
        let svc = signing_service(wallet);

        let result = svc
            .bump_fee(
                &commit_txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::CpfpOutputUnavailable { .. })),
            "got: {result:?}"
        );
    }

    // ── bump_fee — CPFP dust window (#431) ──────────────────────────────────

    /// Decodes the single transaction a mock broadcaster received.
    fn only_broadcast_tx(mock: &MockBroadcaster) -> Transaction {
        let sent = mock.sent_single();
        assert_eq!(sent.len(), 1, "exactly one transaction must be broadcast");
        bdk_wallet::bitcoin::consensus::encode::deserialize_hex(&sent[0])
            .expect("broadcast hex decodes")
    }

    /// #431: a reveal whose change is the protocol's `COMMIT_DUST_SATS` leaves
    /// `anchor - child_fee` below the P2TR dust limit, so the child cannot pay for a
    /// valid output out of the anchor alone. Coin selection must reach for another
    /// wallet UTXO instead of reporting insufficient funds on a funded wallet.
    #[tokio::test]
    async fn bump_fee_governance_cpfp_funds_dust_window_child_from_another_utxo() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package_with_change(
            &mut wallet,
            4_000_000_100,
            crate::domain::fee_constants::COMMIT_DUST_SATS,
        );
        let commit_txid = commit.compute_txid().to_string();
        let anchor = OutPoint {
            txid: reveal.compute_txid(),
            vout: 1,
        };
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &commit_txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await
            .expect("a funded wallet must be able to accelerate a dust-window reveal");

        assert_eq!(result.method, BumpMethod::Cpfp);
        let child = only_broadcast_tx(&mock);
        assert!(
            child.input.len() >= 2,
            "the anchor alone cannot fund fee + a non-dust output: {} input(s)",
            child.input.len()
        );
        assert!(
            child
                .input
                .iter()
                .any(|input| input.previous_output == anchor),
            "the reveal change must stay a mandatory input"
        );
        assert!(
            result.fee_rate_sat_per_kvb >= 4_500,
            "realized package rate {} must approach the requested 5 sat/vB",
            result.fee_rate_sat_per_kvb
        );
    }

    /// #431: the same dust window with a *confirmed* spare coin — the defect never
    /// depended on the extra funds being unconfirmed.
    #[tokio::test]
    async fn bump_fee_governance_cpfp_funds_dust_window_child_from_confirmed_utxo() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package_with_change(
            &mut wallet,
            4_000_000_100,
            crate::domain::fee_constants::COMMIT_DUST_SATS,
        );
        let commit_txid = commit.compute_txid().to_string();
        let anchor = OutPoint {
            txid: reveal.compute_txid(),
            vout: 1,
        };
        // Leave the anchor plus a single confirmed coin as the whole balance.
        spend_all_except(&mut wallet, anchor);
        receive_output_in_latest_block(&mut wallet, 50_000);
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &commit_txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await
            .expect("confirmed spare funds must accelerate a dust-window reveal");

        let child = only_broadcast_tx(&mock);
        assert!(
            child.input.len() >= 2,
            "the confirmed coin must be spent too"
        );
        assert!(
            child
                .input
                .iter()
                .any(|input| input.previous_output == anchor),
            "the reveal change must stay a mandatory input"
        );
        assert!(result.fee_rate_sat_per_kvb >= 4_500);
    }

    /// #431: with no coin left besides the dust-window anchor the bump still fails —
    /// but as a typed `InsufficientFunds`, never a panic or an opaque build error.
    #[tokio::test]
    async fn bump_fee_governance_cpfp_dust_window_without_spare_funds_returns_insufficient_funds() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package_with_change(
            &mut wallet,
            4_000_000_100,
            crate::domain::fee_constants::COMMIT_DUST_SATS,
        );
        let commit_txid = commit.compute_txid().to_string();
        spend_all_except(
            &mut wallet,
            OutPoint {
                txid: reveal.compute_txid(),
                vout: 1,
            },
        );
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &commit_txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::InsufficientFunds { .. })),
            "got: {result:?}"
        );
        assert!(mock.sent_single().is_empty(), "nothing may be broadcast");
    }

    /// Builds a dust-window package on a wallet stripped of every coin but the anchor,
    /// then hands the caller the wallet to plant exactly one funding candidate on.
    /// Returns `(wallet, commit_txid, reveal)`.
    fn dust_window_wallet_with_no_spare_funds() -> (bdk_wallet::Wallet, String, Transaction) {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package_with_change(
            &mut wallet,
            4_000_000_100,
            crate::domain::fee_constants::COMMIT_DUST_SATS,
        );
        spend_all_except(
            &mut wallet,
            OutPoint {
                txid: reveal.compute_txid(),
                vout: 1,
            },
        );
        (wallet, commit.compute_txid().to_string(), reveal)
    }

    /// An unconfirmed transaction paying the wallet, standing in for another pending
    /// package's reveal. Planted rather than built, so the fixture does not depend on
    /// BDK's randomised coin selection.
    fn insert_foreign_reveal_paying_wallet(
        wallet: &mut bdk_wallet::Wallet,
        value: u64,
    ) -> Transaction {
        let change_script = wallet
            .reveal_next_address(bdk_wallet::KeychainKind::Internal)
            .address
            .script_pubkey();
        let reveal = Transaction {
            version: transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([0x77; 32]),
                    vout: 0,
                },
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                ..Default::default()
            }],
            output: vec![
                TxOut {
                    value: Amount::ZERO,
                    script_pubkey: ScriptBuf::new_op_return(b"sps50-action"),
                },
                TxOut {
                    value: Amount::from_sat(value),
                    script_pubkey: change_script,
                },
            ],
        };
        insert_tx(wallet, reveal.clone());
        insert_seen_at(wallet, reveal.compute_txid(), 4_000_000_600);
        reveal
    }

    /// A coinbase paying the wallet, confirmed at the tip and therefore immature.
    fn insert_immature_coinbase(wallet: &mut bdk_wallet::Wallet, value: u64) {
        let script = wallet
            .reveal_next_address(bdk_wallet::KeychainKind::External)
            .address
            .script_pubkey();
        let coinbase = Transaction {
            version: transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: ScriptBuf::from_bytes(vec![0x51; 8]),
                sequence: Sequence::MAX,
                ..Default::default()
            }],
            output: vec![TxOut {
                value: Amount::from_sat(value),
                script_pubkey: script,
            }],
        };
        assert!(coinbase.is_coinbase(), "fixture must be a coinbase");
        let height = wallet.latest_checkpoint().height();
        insert_tx(wallet, coinbase.clone());
        insert_anchor(
            wallet,
            coinbase.compute_txid(),
            bdk_wallet::chain::ConfirmationBlockTime {
                block_id: BlockId {
                    height,
                    hash: BlockHash::all_zeros(),
                },
                confirmation_time: 0,
            },
        );
    }

    /// S1 (audit): `manually_selected_only` bypasses BDK's maturity filter, so nothing
    /// downstream would stop the child from spending an immature coinbase. The node
    /// rejects such a transaction as `premature-spend-of-coinbase` — after the user has
    /// already signed it — and the wallet's own balance does not even count the money.
    #[tokio::test]
    async fn cpfp_child_never_funds_itself_from_an_immature_coinbase() {
        let (mut wallet, commit_txid, reveal) = dust_window_wallet_with_no_spare_funds();
        insert_immature_coinbase(&mut wallet, 5_000_000);
        assert_eq!(
            wallet.balance().immature.to_sat(),
            5_000_000,
            "fixture must leave the coinbase immature"
        );
        let pending = pending_map_from(&commit_txid, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &commit_txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::CpfpFundingUnavailable { .. })),
            "an immature coinbase is not spendable funding, got: {result:?}"
        );
        assert!(mock.sent_single().is_empty(), "nothing may be broadcast");
    }

    /// S3 (audit): an unconfirmed coin from outside the package would drag its own parent
    /// into the child's mempool ancestor set, which `governance_package_stats` does not
    /// account for — so the package rate shown to the user would be higher than the one a
    /// miner computes. In-package unconfirmed coins stay eligible; that is what keeps the
    /// #431 case working on a fully unconfirmed wallet.
    #[tokio::test]
    async fn cpfp_child_never_funds_itself_from_an_unrelated_unconfirmed_coin() {
        let (mut wallet, commit_txid, reveal) = dust_window_wallet_with_no_spare_funds();
        receive_output(&mut wallet, 5_000_000, ReceiveTo::Mempool(4_000_000_500));
        let pending = pending_map_from(&commit_txid, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &commit_txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::CpfpFundingUnavailable { .. })),
            "unaccounted unconfirmed ancestors must not fund the child, got: {result:?}"
        );
        assert!(mock.sent_single().is_empty(), "nothing may be broadcast");
    }

    /// S4 (audit): spending another pending package's anchor would leave that bundle
    /// impossible to accelerate — `get_utxo` would return `None` and the user would be
    /// told "the reveal change is already spent", pointing at a child that does not exist.
    #[tokio::test]
    async fn cpfp_child_never_funds_itself_from_another_pending_packages_anchor() {
        let (mut wallet, commit_txid, reveal) = dust_window_wallet_with_no_spare_funds();
        // The other bundle's reveal, planted directly rather than built through
        // `build_tx`: BDK's coin selection is randomised, so letting it pick the funding
        // for a second package makes the fixture flaky.
        let other_reveal = insert_foreign_reveal_paying_wallet(&mut wallet, 20_000);
        let mut pending = pending_map_from(&commit_txid, &reveal);
        pending.insert(
            "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            other_reveal.compute_txid().to_string(),
        );
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &commit_txid,
                higher_rate(),
                &pending,
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::CpfpFundingUnavailable { .. })),
            "another bundle's anchor is not funding, got: {result:?}"
        );
        assert!(mock.sent_single().is_empty(), "nothing may be broadcast");
    }

    /// The funding coin is the smallest one that closes the gap, not the largest one
    /// available. Every bump parks its inputs behind an unconfirmed child, and an evicted
    /// child is not noticed by the sync path, so sweeping the big coin would put most of
    /// the balance behind a transaction that may quietly vanish.
    #[tokio::test]
    async fn cpfp_child_funds_itself_from_the_smallest_sufficient_coin() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package_with_change(
            &mut wallet,
            4_000_000_100,
            crate::domain::fee_constants::COMMIT_DUST_SATS,
        );
        let commit_txid = commit.compute_txid().to_string();
        // Added after the package, so the commit cannot have consumed them.
        let small = receive_output_in_latest_block(&mut wallet, 5_000);
        let big = receive_output_in_latest_block(&mut wallet, 5_000_000);
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        svc.bump_fee(
            &commit_txid,
            higher_rate(),
            &pending,
            &mock_chain(&[Arc::clone(&mock)]),
        )
        .await
        .expect("cpfp bump must succeed");

        let child = only_broadcast_tx(&mock);
        let spends = |outpoint: OutPoint| {
            child
                .input
                .iter()
                .any(|input| input.previous_output == outpoint)
        };
        assert!(
            spends(small),
            "the 5_000-sat coin is the smallest that closes the gap"
        );
        assert!(
            !spends(big),
            "the 5_000_000-sat coin must be left alone, not swept behind the child"
        );
    }

    /// F-007: when the reveal pays several wallet-owned outputs the anchor is the
    /// largest one. Sized so the anchor covers fee and output on its own, which makes
    /// the input count the tell: picking the small output instead would force a second
    /// input to make up the difference.
    #[tokio::test]
    async fn f007_cpfp_anchors_on_the_largest_reveal_change_output() {
        let mut wallet = funded_wallet();
        let (commit, reveal) =
            insert_governance_package_with_changes(&mut wallet, 4_000_000_100, &[900, 20_000]);
        let commit_txid = commit.compute_txid().to_string();
        let reveal_txid = reveal.compute_txid();
        let pending = pending_map(&commit, &reveal);
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        svc.bump_fee(
            &commit_txid,
            higher_rate(),
            &pending,
            &mock_chain(&[Arc::clone(&mock)]),
        )
        .await
        .expect("cpfp bump must succeed");

        let child = only_broadcast_tx(&mock);
        let spends = |vout: u32| {
            child.input.iter().any(|input| {
                input.previous_output
                    == OutPoint {
                        txid: reveal_txid,
                        vout,
                    }
            })
        };
        assert!(spends(2), "F-007: the 20_000-sat output is the anchor");
        assert_eq!(
            child.input.len(),
            1,
            "F-007: the largest output funds the child on its own; a second input means \
             the smaller one was anchored instead"
        );
    }

    /// F-006: the realized package rate must reach the requested one whatever the
    /// child ends up looking like — the child fee is sized against its *signed*
    /// vsize, so extra funding inputs cannot silently dilute the package.
    #[tokio::test]
    async fn f006_realized_package_rate_reaches_requested_rate_with_extra_inputs() {
        for requested_sat_per_kvb in [5_000u64, 10_000, 20_000] {
            let mut wallet = funded_wallet();
            let (commit, reveal) = insert_governance_package_with_change(
                &mut wallet,
                4_000_000_100,
                crate::domain::fee_constants::COMMIT_DUST_SATS,
            );
            let commit_txid = commit.compute_txid().to_string();
            let pending = pending_map(&commit, &reveal);
            let svc = signing_service(wallet);
            let mock = Arc::new(MockBroadcaster::ok("Electrum"));

            let result = svc
                .bump_fee(
                    &commit_txid,
                    FeeRate::new(requested_sat_per_kvb, 1_000).expect("valid rate"),
                    &pending,
                    &mock_chain(&[Arc::clone(&mock)]),
                )
                .await
                .unwrap_or_else(|e| panic!("bump at {requested_sat_per_kvb} sat/kvB failed: {e}"));

            let child = only_broadcast_tx(&mock);
            assert!(child.input.len() >= 2, "dust window needs extra inputs");
            assert!(
                result.fee_rate_sat_per_kvb >= requested_sat_per_kvb * 90 / 100,
                "realized package rate {} must reach 90% of the requested {}",
                result.fee_rate_sat_per_kvb,
                requested_sat_per_kvb
            );
        }
    }

    #[tokio::test]
    async fn bump_fee_with_rate_not_above_current_returns_fee_rate_too_low() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let svc = signing_service(wallet);
        let same_rate = FeeRate::new(1_000, 1_000).expect("valid rate"); // 1 sat/vB == original

        let result = svc
            .bump_fee(
                &tx.compute_txid().to_string(),
                same_rate,
                &HashMap::new(),
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        assert!(
            matches!(
                result,
                Err(BumpFeeError::FeeRateTooLow { .. }) | Err(BumpFeeError::FeeTooLow { .. })
            ),
            "got: {result:?}"
        );
    }

    #[tokio::test]
    async fn bump_fee_happy_path_signs_and_broadcasts_replacement() {
        let mut wallet = funded_wallet();
        let original = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let original_txid = original.compute_txid().to_string();
        let original_fee = {
            // Fee of the original spend, for the strictly-greater assertion below.
            wallet.calculate_fee(&original).expect("fee known").to_sat()
        };
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .bump_fee(
                &original_txid,
                higher_rate(),
                &HashMap::new(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await
            .expect("bump must succeed");

        assert_ne!(
            result.new_txid, original_txid,
            "replacement must have a new txid"
        );
        assert_eq!(result.target_txid, original_txid);
        assert_eq!(result.method, BumpMethod::Rbf);
        assert!(
            result.fee_sats > original_fee,
            "replacement fee {} must exceed original {}",
            result.fee_sats,
            original_fee
        );
        assert!(
            result.fee_rate_sat_per_kvb >= 5_000 - 1_000,
            "rate near requested"
        );

        let sent = mock.sent_single();
        assert_eq!(sent.len(), 1, "exactly one replacement must be broadcast");
        let sent_tx: Transaction =
            bdk_wallet::bitcoin::consensus::encode::deserialize_hex(&sent[0])
                .expect("broadcast hex decodes");
        assert_eq!(sent_tx.compute_txid().to_string(), result.new_txid);
        assert!(
            sent_tx
                .output
                .iter()
                .any(|o| o.script_pubkey == external_script()),
            "replacement must keep paying the original recipient"
        );
    }

    #[tokio::test]
    async fn bump_fee_all_broadcasters_failing_returns_broadcast_failed() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let svc = signing_service(wallet);
        let chain = mock_chain(&[
            Arc::new(MockBroadcaster::failing("Electrum", "connection refused")),
            Arc::new(MockBroadcaster::failing("Bitcoin node", "insufficient fee")),
        ]);

        let result = svc
            .bump_fee(
                &tx.compute_txid().to_string(),
                higher_rate(),
                &HashMap::new(),
                &chain,
            )
            .await;

        match result {
            Err(BumpFeeError::BroadcastFailed { message }) => {
                assert!(message.contains("connection refused"), "got: {message}");
                assert!(message.contains("insufficient fee"), "got: {message}");
            }
            other => panic!("expected BroadcastFailed, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn bump_fee_falls_back_to_second_broadcaster() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let svc = signing_service(wallet);
        let node = Arc::new(MockBroadcaster::ok("Bitcoin node"));
        let chain = mock_chain(&[
            Arc::new(MockBroadcaster::failing("Electrum", "connection refused")),
            Arc::clone(&node),
        ]);

        svc.bump_fee(
            &tx.compute_txid().to_string(),
            higher_rate(),
            &HashMap::new(),
            &chain,
        )
        .await
        .expect("fallback broadcast must succeed");

        assert_eq!(
            node.sent_single().len(),
            1,
            "node fallback must receive the tx"
        );
    }

    // ── fee_rate_sat_per_kvb ────────────────────────────────────────────────

    #[test]
    fn fee_rate_from_fee_and_vsize_rounds_up() {
        // 141 sats / 140 vB = 1007.14… sat/kvB → ceil 1008
        assert_eq!(fee_rate_sat_per_kvb(141, 140), 1_008);
    }

    #[test]
    fn fee_rate_guards_zero_vsize() {
        assert_eq!(fee_rate_sat_per_kvb(1_000, 0), 1_000_000);
    }

    // ── DTO serialization contract ──────────────────────────────────────────

    #[test]
    fn unconfirmed_tx_dto_serializes_camel_case() {
        let dto = UnconfirmedTxDto {
            txid: "ab".into(),
            sent_sats: 100,
            received_sats: 40,
            net_sats: -60,
            fee_sats: Some(10),
            fee_rate_sat_per_kvb: Some(1_000),
            vsize_vbytes: 10,
            is_rbf_signaling: true,
            is_governance_commit: true,
            bump_method: Some(BumpMethod::Cpfp),
            package_fee_sats: Some(470),
            package_vsize_vbytes: Some(270),
            package_fee_rate_sat_per_kvb: Some(1_741),
            max_bump_rate_sat_per_kvb: Some(2_916_000),
            last_seen_secs: Some(1),
        };
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["sentSats"], 100);
        assert_eq!(json["netSats"], -60);
        assert_eq!(json["feeRateSatPerKvb"], 1_000);
        assert_eq!(json["isRbfSignaling"], true);
        assert_eq!(json["isGovernanceCommit"], true);
        assert_eq!(json["bumpMethod"], "cpfp");
        assert_eq!(json["packageFeeSats"], 470);
        assert_eq!(json["packageVsizeVbytes"], 270);
        assert_eq!(json["packageFeeRateSatPerKvb"], 1_741);
        assert_eq!(json["maxBumpRateSatPerKvb"], 2_916_000);
        assert_eq!(json["lastSeenSecs"], 1);
        assert_eq!(json["vsizeVbytes"], 10);
    }

    #[test]
    fn bump_fee_result_dto_serializes_method_lowercase() {
        let dto = BumpFeeResultDto {
            new_txid: "cd".into(),
            target_txid: "ab".into(),
            fee_sats: 500,
            fee_rate_sat_per_kvb: 5_000,
            method: BumpMethod::Rbf,
            sync_warning: None,
        };
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["newTxid"], "cd");
        assert_eq!(json["targetTxid"], "ab");
        assert_eq!(json["method"], "rbf");
        assert!(
            json.get("syncWarning").is_none(),
            "None sync_warning must be skipped"
        );
    }

    #[test]
    fn bump_fee_result_dto_serializes_sync_warning_when_present() {
        let dto = BumpFeeResultDto {
            new_txid: "cd".into(),
            target_txid: "ab".into(),
            fee_sats: 500,
            fee_rate_sat_per_kvb: 5_000,
            method: BumpMethod::Rbf,
            sync_warning: Some("Wallet sync failed".to_string()),
        };
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["syncWarning"], "Wallet sync failed");
    }

    // ── PackageStats::required_child_fee ────────────────────────────────────

    fn sample_package() -> PackageStats {
        PackageStats {
            fee_sats: 470,
            vsize_vbytes: 270,
        }
    }

    #[test]
    fn required_child_fee_reaches_target_package_rate_for_the_child_size_at_hand() {
        let rate = FeeRate::new(5_000, 1_000).expect("valid rate");
        // 5 sat/vB over `270 + child_vsize`, minus the 470 sats the package already pays.
        for (child_vsize, expected_fee) in [(111u64, 1_435u64), (169, 1_725), (227, 2_015)] {
            assert_eq!(
                sample_package()
                    .required_child_fee(rate, child_vsize)
                    .expect("fee ok"),
                expected_fee,
                "child of {child_vsize} vB"
            );
        }
    }

    /// F-006's decision, exercised directly. The integrated path cannot reach the
    /// shortfall branch — the fee is priced from an upper-bound size model, so the
    /// realized rate is always at or above the request — which leaves this the only
    /// place the threshold arithmetic is actually pinned.
    #[test]
    fn package_rate_shortfall_flags_only_packages_under_the_tolerance() {
        let rate = FeeRate::new(5_000, 1_000).expect("valid rate");
        let package = sample_package(); // 470 sats over 270 vB
                                        // Exactly on target: 5 sat/vB over 270 + 111 vB.
        assert_eq!(
            package_rate_shortfall(package, 1_435, 111, rate),
            None,
            "a package that meets the request must pass"
        );
        // The exact edge of the 10% tolerance, one sat either side of it.
        assert_eq!(
            package_rate_shortfall(package, 1_245, 111, rate),
            None,
            "the 10% tolerance must be honoured, not rounded away"
        );
        assert_eq!(
            package_rate_shortfall(package, 1_244, 111, rate),
            Some(4_499),
            "one sat below the tolerance is a shortfall"
        );
        // Under the tolerance: the child pays its relay floor and nothing more.
        assert_eq!(
            package_rate_shortfall(package, 111, 111, rate),
            Some(fee_rate_sat_per_kvb(470 + 111, 270 + 111)),
            "a short package must be reported with its realized rate"
        );
    }

    #[test]
    fn required_child_fee_below_child_relay_floor_is_rejected() {
        // 1 sat/vB → 381 total < 470 already paid → child fee would be 0.
        let rate = FeeRate::new(1_000, 1_000).expect("valid rate");
        match sample_package().required_child_fee(rate, 111) {
            Err(BumpFeeError::FeeRateTooLow {
                required_sat_per_kvb,
            }) => {
                // (470 + 111) * 1000 / 381 = 1524.93… → ceil 1525
                assert_eq!(required_sat_per_kvb, 1_525);
            }
            other => panic!("expected FeeRateTooLow, got: {other:?}"),
        }
    }

    /// The package from the #431 report: commit + reveal of 311 vB paying 556 sats,
    /// accelerated by the 150 vB child the wallet built for it.
    fn reported_package() -> PackageStats {
        PackageStats {
            fee_sats: 556,
            vsize_vbytes: 311,
        }
    }

    #[test]
    fn max_package_rate_is_the_rate_whose_child_lands_exactly_on_the_broadcast_ceiling() {
        // The reported case: (10_000_000·150 + 556·1000) / 461 → 3255.0 sat/vB, a third
        // of the 10,000 sat/vB the operator was offered.
        assert_eq!(
            reported_package().max_package_rate_sat_per_kvb(150),
            3_255_002
        );
        // And the ordinary 111 vB child on the test package.
        assert_eq!(
            sample_package().max_package_rate_sat_per_kvb(111),
            2_914_619
        );
    }

    #[test]
    fn required_child_fee_at_the_package_ceiling_is_accepted_and_one_step_over_is_not() {
        let package = reported_package();
        let ceiling = package.max_package_rate_sat_per_kvb(150);

        let fee = package
            .required_child_fee(FeeRate::new(ceiling, 1_000).expect("valid rate"), 150)
            .expect("the ceiling itself must be reachable");
        assert_eq!(
            fee_rate_sat_per_kvb(fee, 150),
            MAX_BROADCAST_SAT_PER_KVB,
            "the ceiling must put the child exactly on the broadcast limit, not under it"
        );

        assert!(
            matches!(
                package
                    .required_child_fee(FeeRate::new(ceiling + 1, 1_000).expect("valid rate"), 150),
                Err(BumpFeeError::FeeRateTooHigh { .. })
            ),
            "one sat/kvB over the ceiling must be rejected"
        );
    }

    /// #431: the rate the UI used to offer — the PRD's 10,000 sat/vB — priced the child
    /// at ~30,730 sat/vB, which `Psbt::extract_tx` refused as an "absurdly high fee rate
    /// of 7699471" (sat/kwu) at signing time. It must now fail early, in the package's
    /// own terms.
    #[test]
    fn required_child_fee_over_the_child_broadcast_ceiling_is_rejected_with_the_package_ceiling() {
        let rate = FeeRate::new(10_000_000, 1_000).expect("valid rate");
        match reported_package().required_child_fee(rate, 150) {
            Err(BumpFeeError::FeeRateTooHigh {
                max_sat_per_kvb,
                child_sat_per_kvb,
            }) => {
                assert_eq!(max_sat_per_kvb, 3_255_002);
                // 4_609_444 sats over 150 vB — the child fee the report screenshot showed.
                assert_eq!(child_sat_per_kvb, 30_729_627);
            }
            other => panic!("expected FeeRateTooHigh, got: {other:?}"),
        }
    }

    /// The 111 vB the old constant hard-coded must fall out of the descriptor-driven
    /// model for a single BIP-86 `tr()` input, and grow by one full input after that.
    #[test]
    fn cpfp_child_vsize_matches_the_signed_taproot_child() {
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let script = wallet
            .peek_address(bdk_wallet::KeychainKind::Internal, 0)
            .address
            .script_pubkey();
        let witness_wu = wallet
            .public_descriptor(bdk_wallet::KeychainKind::Internal)
            .max_weight_to_satisfy()
            .expect("satisfiable")
            .to_wu();

        assert_eq!(cpfp_child_vsize(1, &script, witness_wu), 111);
        assert_eq!(cpfp_child_vsize(2, &script, 2 * witness_wu), 169);
    }

    // ── F-001 regression: governance commit CPFP guard survives restart ─────

    /// F-001 regression: after an app restart (simulated by loading pending reveals
    /// from a persisted state), a governance commit must still be flagged as
    /// `is_governance_commit: true` and offered CPFP — never RBF.
    ///
    /// This test simulates the restart scenario by:
    /// 1. Creating a governance package in the wallet graph
    /// 2. Creating a pending reveals map with the commit→reveal mapping (as if loaded from disk)
    /// 3. Verifying that list_unconfirmed_sent_txs correctly identifies the commit
    #[tokio::test]
    async fn f001_governance_commit_after_restart_shows_cpfp_not_rbf() {
        let mut wallet = funded_wallet();
        let (commit, reveal) = insert_governance_package(&mut wallet, 4_000_000_100);
        let commit_txid = commit.compute_txid().to_string();
        let svc = WalletService::new(wallet, test_node_config());

        // Simulate app restart: the pending reveals map is loaded from disk
        // (as if persistence restored it after restart)
        let pending_after_restart = pending_map(&commit, &reveal);

        let rows = svc
            .list_unconfirmed_sent_txs(&pending_after_restart)
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.txid, commit_txid);
        assert!(
            row.is_governance_commit,
            "F-001: governance commit must be flagged after restart"
        );
        assert_eq!(
            row.bump_method,
            Some(BumpMethod::Cpfp),
            "F-001: governance commit must offer CPFP, never RBF"
        );
    }

    /// F-001 regression: attempting RBF on a governance commit (when pending reveals
    /// map is empty, simulating a bug where persistence failed) must not succeed.
    ///
    /// Without the pending reveals entry, the commit would appear as a regular RBF-signaling
    /// transaction. This test documents the expected behavior: the bump should fail because
    /// the commit's sequence is MAX (non-RBF) when built via build_tx() without explicit
    /// RBF signaling — but if it were RBF-signaling, the persistence fix ensures we still
    /// know it's a governance commit.
    #[tokio::test]
    async fn f001_governance_commit_without_pending_map_is_not_flagged() {
        let mut wallet = funded_wallet();
        let (_commit, _reveal) = insert_governance_package(&mut wallet, 4_000_000_100);
        let svc = WalletService::new(wallet, test_node_config());

        // Simulate the bug scenario: pending reveals map is empty (persistence failed)
        let empty_pending: HashMap<String, String> = HashMap::new();

        let rows = svc
            .list_unconfirmed_sent_txs(&empty_pending)
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        // Without the pending reveals entry, the commit is NOT flagged as governance
        // This documents the bug that F-001 fixes: persistence ensures the mapping survives
        assert!(
            !row.is_governance_commit,
            "without pending reveals, commit is not flagged (this is the bug F-001 fixes)"
        );
        // The commit built by build_tx() signals RBF by default (BDK default sequence)
        // So without persistence, it would incorrectly offer RBF
        assert_eq!(
            row.bump_method,
            Some(BumpMethod::Rbf),
            "without pending reveals, RBF is incorrectly offered (this is the bug F-001 fixes)"
        );
    }
}
