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
use crate::domain::fee_rate::FeeRate;
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
    #[error("insufficient funds to pay the increased fee: {message}")]
    InsufficientFunds { message: String },
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
        BumpFeeError::InsufficientFunds { .. } => "InsufficientFunds",
        BumpFeeError::InvalidFeeRate(_) => "InvalidFeeRate",
        BumpFeeError::BuildFailed { .. } => "BuildFailed",
        BumpFeeError::SignFailed { .. } => "SignFailed",
        BumpFeeError::BroadcastFailed { .. } => "BroadcastFailed",
    }
}

// ── Fee arithmetic ──────────────────────────────────────────────────────────

/// Fee rate in sat/kvB from an absolute fee and a vsize (ceiling, never underreports).
fn fee_rate_sat_per_kvb(fee_sats: u64, vsize_vbytes: u64) -> u64 {
    fee_sats.saturating_mul(1_000).div_ceil(vsize_vbytes.max(1))
}

/// Conservative vsize of the CPFP child: 1 P2TR keypath input (~57.5 vB) +
/// 1 P2TR output (43 vB) + tx overhead (~10.5 vB). If BDK must add an extra
/// wallet input to fund the fee, the realized package rate lands slightly below
/// the requested one — acceptable for an accelerator (the child is RBF-bumpable).
const CPFP_CHILD_VSIZE_EST_VBYTES: u64 = 111;

/// Known fee + vsize of the unconfirmed `commit → reveal` pair.
#[derive(Debug, Clone, Copy)]
struct PackageStats {
    fee_sats: u64,
    vsize_vbytes: u64,
}

impl PackageStats {
    /// Absolute child fee needed so `(package_fee + child_fee) / (package_vsize +
    /// child_vsize)` reaches `rate`. Errors when the result would not even pay the
    /// child's own 1 sat/vB relay floor — i.e. the package already meets the rate.
    fn required_child_fee(self, rate: FeeRate) -> Result<u64, BumpFeeError> {
        let total_vsize = self.vsize_vbytes + CPFP_CHILD_VSIZE_EST_VBYTES;
        let child_fee = rate.fee_sats(total_vsize).saturating_sub(self.fee_sats);
        // The child must pay at least its own min-relay share (1 sat/vB).
        let child_floor = CPFP_CHILD_VSIZE_EST_VBYTES;
        if child_fee < child_floor {
            return Err(BumpFeeError::FeeRateTooLow {
                required_sat_per_kvb: fee_rate_sat_per_kvb(
                    self.fee_sats + child_floor,
                    total_vsize,
                ),
            });
        }
        Ok(child_fee)
    }
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
        rows.sort_by(|a, b| b.last_seen_secs.cmp(&a.last_seen_secs));
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
                    .build_cpfp_child_psbt(txid, parsed_txid, reveal_txid, new_rate)
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
    async fn build_cpfp_child_psbt(
        &self,
        txid: &str,
        commit_txid: bdk_wallet::bitcoin::Txid,
        reveal_txid: &str,
        new_rate: FeeRate,
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

        // Anchor: the reveal's change output back to the Admin Wallet.
        let vout = reveal_tx
            .output
            .iter()
            .position(|out| wallet.is_mine(out.script_pubkey.clone()))
            .ok_or_else(|| {
                unavailable("the reveal pays no change back to the admin wallet".to_string())
            })?;
        let anchor = OutPoint {
            txid: reveal_parsed,
            vout: vout as u32,
        };
        if wallet.get_utxo(anchor).is_none() {
            return Err(unavailable(
                "the reveal change is already spent — bump the existing child transaction instead"
                    .to_string(),
            ));
        }

        let package = governance_package_stats(&wallet, &commit_tx, reveal_txid)
            .ok_or_else(|| unavailable("the package fee is not known to the wallet".to_string()))?;
        let child_fee = package.required_child_fee(new_rate)?;

        let drain_script = wallet
            .reveal_next_address(bdk_wallet::KeychainKind::Internal)
            .address
            .script_pubkey();
        let mut builder = wallet.build_tx();
        builder
            .add_utxo(anchor)
            .map_err(|e| unavailable(e.to_string()))?;
        builder.drain_to(drain_script);
        builder.fee_absolute(Amount::from_sat(child_fee));
        let psbt = builder.finish().map_err(map_create_tx_error)?;
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
        insert_checkpoint, insert_seen_at, insert_tx, receive_output,
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

    /// Admin wallet with one confirmed 100_000-sat UTXO at external index 0.
    fn funded_wallet() -> bdk_wallet::Wallet {
        let mut wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        insert_checkpoint(
            &mut wallet,
            BlockId {
                height: 1_000,
                hash: BlockHash::all_zeros(),
            },
        );
        receive_output_in_latest_block(&mut wallet, 100_000);
        wallet
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
        let envelope_script = external_script();
        let mut builder = wallet.build_tx();
        builder.add_recipient(envelope_script.clone(), Amount::from_sat(20_000));
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
        let change_script = wallet
            .reveal_next_address(bdk_wallet::KeychainKind::Internal)
            .address
            .script_pubkey();
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
            output: vec![
                TxOut {
                    value: Amount::ZERO,
                    script_pubkey: ScriptBuf::new_op_return(b"sps50-action"),
                },
                TxOut {
                    value: Amount::from_sat(19_700),
                    script_pubkey: change_script,
                },
            ],
        };
        insert_tx(wallet, reveal.clone());
        insert_seen_at(wallet, reveal.compute_txid(), seen_at + 1);
        (commit, reveal)
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
        let mut wallet = funded_wallet();
        // Two independent confirmed UTXOs so the two spends do not conflict.
        receive_output_in_latest_block(&mut wallet, 50_000);
        let older = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let newer = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_900);
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashMap::new())
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 2);
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
        };
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["newTxid"], "cd");
        assert_eq!(json["targetTxid"], "ab");
        assert_eq!(json["method"], "rbf");
    }

    // ── PackageStats::required_child_fee ────────────────────────────────────

    #[test]
    fn required_child_fee_reaches_target_package_rate() {
        let package = PackageStats {
            fee_sats: 470,
            vsize_vbytes: 270,
        };
        // Target 5 sat/vB over 270 + 111 = 381 vB → 1_905 total − 470 = 1_435 child.
        let rate = FeeRate::new(5_000, 1_000).expect("valid rate");
        assert_eq!(package.required_child_fee(rate).expect("fee ok"), 1_435);
    }

    #[test]
    fn required_child_fee_below_child_relay_floor_is_rejected() {
        let package = PackageStats {
            fee_sats: 470,
            vsize_vbytes: 270,
        };
        // 1 sat/vB → 381 total < 470 already paid → child fee would be 0.
        let rate = FeeRate::new(1_000, 1_000).expect("valid rate");
        match package.required_child_fee(rate) {
            Err(BumpFeeError::FeeRateTooLow {
                required_sat_per_kvb,
            }) => {
                // (470 + 111) * 1000 / 381 = 1524.93… → ceil 1525
                assert_eq!(required_sat_per_kvb, 1_525);
            }
            other => panic!("expected FeeRateTooLow, got: {other:?}"),
        }
    }
}
