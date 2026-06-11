//! Phase 5 (PRD §4.3.3) — unconfirmed sent-transaction listing and RBF fee-bump
//! use-cases over [`WalletService`].
//!
//! Business logic only: depends on the [`TxBroadcaster`] port and the session
//! [`PsbtSigner`](crate::application::psbt_signer::PsbtSigner) port (via
//! `WalletService::sign_and_finalize_psbt`), never on concrete transports.

use std::collections::HashSet;
use std::sync::Arc;

use serde::Serialize;

use crate::application::tx_broadcaster::{broadcast_single_with_fallback, TxBroadcaster};
use crate::application::wallet_service::WalletService;
use crate::domain::fee_rate::FeeRate;
use crate::infrastructure::admin_wallet::AdminWalletError;

// ── DTOs ────────────────────────────────────────────────────────────────────

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
    /// bumping it would invalidate the reveal (R1.0.1), so the UI must not offer Bump.
    pub is_governance_commit: bool,
    /// Mempool last-seen, unix seconds. `None` when the indexer gave no timestamp.
    pub last_seen_secs: Option<u64>,
}

/// Outcome of a successful fee-bump: the replacement is signed and broadcast.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpFeeResultDto {
    pub new_txid: String,
    pub replaced_txid: String,
    pub fee_sats: u64,
    pub fee_rate_sat_per_kvb: u64,
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
    #[error(
        "transaction {txid} is a governance commit with a pending reveal — \
         replacing it would invalidate the pre-signed reveal"
    )]
    GovernanceCommitNotReplaceable { txid: String },
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
        BumpFeeError::GovernanceCommitNotReplaceable { .. } => "GovernanceCommitNotReplaceable",
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
    /// `pending_commit_txids` marks governance commits whose pre-signed reveal is still
    /// pending (see [`crate::application::pending_reveals::pending_commit_txids`]);
    /// those rows are flagged so the UI disables Bump for them.
    ///
    /// Pure read over the last-synced wallet state — no network I/O.
    pub async fn list_unconfirmed_sent_txs(
        &self,
        pending_commit_txids: &HashSet<String>,
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
                Some(UnconfirmedTxDto {
                    is_governance_commit: pending_commit_txids.contains(&txid),
                    txid,
                    sent_sats: sent.to_sat(),
                    received_sats: received.to_sat(),
                    net_sats: received.to_sat() as i64 - sent.to_sat() as i64,
                    fee_rate_sat_per_kvb: fee_sats
                        .map(|fee| fee_rate_sat_per_kvb(fee, vsize_vbytes)),
                    fee_sats,
                    vsize_vbytes,
                    is_rbf_signaling: tx.input.iter().any(|input| input.sequence.is_rbf()),
                    last_seen_secs,
                })
            })
            .collect();
        rows.sort_by(|a, b| b.last_seen_secs.cmp(&a.last_seen_secs));
        Ok(rows)
    }

    /// Replaces an unconfirmed, RBF-signaling wallet transaction with a higher-fee
    /// version (PRD §4.3.3): build via BDK `build_fee_bump`, sign through the session
    /// [`PsbtSigner`](crate::application::psbt_signer::PsbtSigner) port, broadcast
    /// Electrum-first with node fallback.
    ///
    /// Capability guards run **before** any wallet or network I/O. The caller is
    /// responsible for syncing the wallet beforehand (the IPC command does so
    /// best-effort); a stale view is ultimately caught by the node, which rejects
    /// replacements of confirmed or already-replaced transactions.
    pub async fn bump_fee(
        &self,
        txid: &str,
        new_rate: FeeRate,
        pending_commit_txids: &HashSet<String>,
        broadcasters: &[Arc<dyn TxBroadcaster>],
    ) -> Result<BumpFeeResultDto, BumpFeeError> {
        // 1–2. Capability guards — before any wallet lock or network contact.
        let signer = self.signer().ok_or(BumpFeeError::ReadOnly)?;
        if !signer.allowed_on(self.network()) {
            return Err(BumpFeeError::SignerNotAllowedOnNetwork);
        }
        // 3. Governance-commit guard — a pending pre-signed reveal spends this txid.
        if pending_commit_txids.contains(txid) {
            return Err(BumpFeeError::GovernanceCommitNotReplaceable {
                txid: txid.to_string(),
            });
        }
        let parsed_txid: bdk_wallet::bitcoin::Txid =
            txid.parse().map_err(|_| BumpFeeError::InvalidTxid {
                txid: txid.to_string(),
            })?;

        // 4. Build the replacement PSBT (BDK rejects confirmed / non-RBF / unknown).
        let psbt = {
            let mut wallet = self.wallet.lock().await;
            let mut builder = wallet
                .build_fee_bump(parsed_txid)
                .map_err(|e| map_build_fee_bump_error(e, txid))?;
            builder.fee_rate(new_rate.to_bdk());
            builder.finish().map_err(map_create_tx_error)?
        };

        // 5. Sign through the session signer port (same flow as commit funding, R1.1).
        let tx = self
            .sign_and_finalize_psbt(psbt)
            .await
            .map_err(|e| BumpFeeError::SignFailed {
                message: e.to_string(),
            })?;

        // 6. Result metadata — all prevouts are wallet-known, so the fee is exact.
        let new_txid = tx.compute_txid().to_string();
        let fee_sats = {
            let wallet = self.wallet.lock().await;
            wallet
                .calculate_fee(&tx)
                .map(|fee| fee.to_sat())
                .map_err(|e| BumpFeeError::BuildFailed {
                    message: format!("replacement fee unknown: {e}"),
                })?
        };
        let vsize_vbytes = tx.vsize() as u64;

        // 7. Broadcast: Electrum first, node RPC fallback.
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

        Ok(BumpFeeResultDto {
            new_txid,
            replaced_txid: txid.to_string(),
            fee_rate_sat_per_kvb: fee_rate_sat_per_kvb(fee_sats, vsize_vbytes),
            fee_sats,
        })
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

    // ── list_unconfirmed_sent_txs ───────────────────────────────────────────

    #[tokio::test]
    async fn list_returns_empty_on_fresh_wallet() {
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashSet::new())
            .await
            .expect("list ok");

        assert!(rows.is_empty(), "fresh wallet must list no transactions");
    }

    #[tokio::test]
    async fn list_excludes_confirmed_transactions() {
        let svc = WalletService::new(funded_wallet(), test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashSet::new())
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
            .list_unconfirmed_sent_txs(&HashSet::new())
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
        assert_eq!(row.last_seen_secs, Some(4_000_000_100));
    }

    #[tokio::test]
    async fn list_excludes_incoming_only_unconfirmed_tx() {
        let mut wallet = funded_wallet();
        receive_output(&mut wallet, 5_000, ReceiveTo::Mempool(4_000_000_200));
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashSet::new())
            .await
            .expect("list ok");

        assert!(
            rows.is_empty(),
            "incoming-only unconfirmed txs are not 'sent from the Admin Wallet'"
        );
    }

    #[tokio::test]
    async fn list_flags_pending_governance_commit() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let svc = WalletService::new(wallet, test_node_config());
        let pending: HashSet<String> = [tx.compute_txid().to_string()].into();

        let rows = svc
            .list_unconfirmed_sent_txs(&pending)
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].is_governance_commit,
            "txid present in PendingReveals must be flagged as governance commit"
        );
    }

    #[tokio::test]
    async fn list_reports_non_rbf_spend_as_not_signaling() {
        let mut wallet = funded_wallet();
        insert_unconfirmed_spend(&mut wallet, false, 4_000_000_100);
        let svc = WalletService::new(wallet, test_node_config());

        let rows = svc
            .list_unconfirmed_sent_txs(&HashSet::new())
            .await
            .expect("list ok");

        assert_eq!(rows.len(), 1);
        assert!(
            !rows[0].is_rbf_signaling,
            "MAX sequence on every input must report is_rbf_signaling=false"
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
            .list_unconfirmed_sent_txs(&HashSet::new())
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
            .list_unconfirmed_sent_txs(&HashSet::new())
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
                &HashSet::new(),
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
                &HashSet::new(),
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
                &HashSet::new(),
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
                &HashSet::new(),
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
                &HashSet::new(),
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        assert!(
            matches!(result, Err(BumpFeeError::TxNotReplaceable { .. })),
            "got: {result:?}"
        );
    }

    #[tokio::test]
    async fn bump_fee_pending_governance_commit_is_rejected() {
        let mut wallet = funded_wallet();
        let tx = insert_unconfirmed_spend(&mut wallet, true, 4_000_000_100);
        let txid = tx.compute_txid().to_string();
        let svc = signing_service(wallet);
        let pending: HashSet<String> = [txid.clone()].into();
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
            matches!(
                result,
                Err(BumpFeeError::GovernanceCommitNotReplaceable { .. })
            ),
            "got: {result:?}"
        );
        assert!(mock.sent_single().is_empty());
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
                &HashSet::new(),
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
                &HashSet::new(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await
            .expect("bump must succeed");

        assert_ne!(
            result.new_txid, original_txid,
            "replacement must have a new txid"
        );
        assert_eq!(result.replaced_txid, original_txid);
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
                &HashSet::new(),
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
            &HashSet::new(),
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
            is_governance_commit: false,
            last_seen_secs: Some(1),
        };
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["sentSats"], 100);
        assert_eq!(json["netSats"], -60);
        assert_eq!(json["feeRateSatPerKvb"], 1_000);
        assert_eq!(json["isRbfSignaling"], true);
        assert_eq!(json["isGovernanceCommit"], false);
        assert_eq!(json["lastSeenSecs"], 1);
        assert_eq!(json["vsizeVbytes"], 10);
    }
}
