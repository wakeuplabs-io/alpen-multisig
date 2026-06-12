//! Phase 6 (PRD §4.3.5) — Send BTC from the Admin Wallet.
//!
//! Composes the existing pieces into one user-facing spend flow: PSBT build via
//! BDK (`TxBuilder`), signing through the session
//! [`PsbtSigner`](crate::application::psbt_signer::PsbtSigner) port (R1.1, via
//! `WalletService::sign_and_finalize_psbt`), and Electrum-first broadcast with
//! node fallback through the [`TxBroadcaster`] port. No new protocol or custody
//! primitive — see `docs/specs/admin-wallet-send-btc-implementation.md`.
//!
//! Change discipline (PRD §4.3.5.4): BDK's `TxBuilder` sources the change script
//! from the wallet's **first unused internal index** (peek-based, non-advancing),
//! so normal sends route change to `…/1/*` by construction. Max sends are drain
//! builds (`drain_wallet` + `drain_to`) and produce no change output.

use std::sync::Arc;

use crate::application::tx_broadcaster::{broadcast_single_with_fallback, TxBroadcaster};
use crate::application::wallet_service::WalletService;
use crate::application::wallet_transactions::fee_rate_sat_per_kvb;
use crate::domain::fee_rate::FeeRate;

// ── DTOs ────────────────────────────────────────────────────────────────────

/// IPC payload for the send / estimate commands.
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

/// Outcome of a successful send: the transaction is signed and broadcast.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResultDto {
    pub txid: String,
    /// Effective amount paid to the destination (the drain value for Max sends).
    pub amount_sats: u64,
    pub fee_sats: u64,
    /// Realized rate: `ceil(fee · 1000 / vsize)` in sat/kvB.
    pub fee_rate_sat_per_kvb: u64,
    pub vsize_vbytes: u64,
    /// Change returned to the wallet's internal keychain. 0 for drain builds.
    pub change_sats: u64,
    pub drained: bool,
}

// ── Errors ──────────────────────────────────────────────────────────────────

/// Typed failure surface for the Send use-case (PRD §4.3.5).
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    // Capability (guard order 1–2; before any lock or I/O)
    #[error("admin wallet is read-only — a signing-capable session is required to send")]
    ReadOnly,
    #[error("the session signer is not allowed on this network")]
    SignerNotAllowedOnNetwork,

    // Destination (§4.3.5.1)
    #[error("'{address}' is not a bitcoin address")]
    InvalidAddress { address: String },
    #[error("'{address}' is not a {expected_network} bitcoin address")]
    WrongNetwork {
        address: String,
        expected_network: String,
    },

    // Amount (§4.3.5.2)
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

/// Stable error code for the tagged `{ "type", "message" }` IPC error shape
/// (mirrors `wallet_transactions::bump_error_code`). Codes shared with
/// `BumpFeeError` reuse the same strings so the frontend union needs no
/// duplicate handling.
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

// ── Destination validation ──────────────────────────────────────────────────

/// Network display word for PRD §4.3.5.1 copy ("Destination must be a
/// [correct network] bitcoin address."). Testnet variants collapse to
/// "testnet" — the PRD names only mainnet/testnet; regtest/signet are the
/// dev networks the same template covers.
pub fn network_display_name(network: bdk_wallet::bitcoin::Network) -> &'static str {
    use bdk_wallet::bitcoin::Network;
    match network {
        Network::Bitcoin => "mainnet",
        Network::Testnet | Network::Testnet4 => "testnet",
        Network::Signet => "signet",
        Network::Regtest => "regtest",
    }
}

/// Parses `address` and enforces it belongs to `network` (PRD §4.3.5.1).
///
/// Pure function — no wallet lock, no I/O. Accepts every address type the
/// rust-bitcoin parser considers standard (P2PKH, P2SH, P2WPKH, P2WSH, P2TR,
/// future witness versions). Note: base58 testnet and regtest share version
/// bytes upstream, so a legacy testnet address is accepted on regtest
/// (`is_valid_for_network` semantics) — mainnet prefixes are distinct.
pub fn parse_send_destination(
    address: &str,
    network: bdk_wallet::bitcoin::Network,
) -> Result<bdk_wallet::bitcoin::Address, SendError> {
    use bdk_wallet::bitcoin::address::{Address, NetworkUnchecked};
    let unchecked: Address<NetworkUnchecked> =
        address
            .trim()
            .parse()
            .map_err(|_| SendError::InvalidAddress {
                address: address.to_string(),
            })?;
    unchecked
        .require_network(network)
        .map_err(|_| SendError::WrongNetwork {
            address: address.to_string(),
            expected_network: network_display_name(network).to_string(),
        })
}

// ── BDK error mapping ───────────────────────────────────────────────────────

fn map_send_create_tx_error(e: bdk_wallet::error::CreateTxError) -> SendError {
    use bdk_wallet::error::CreateTxError as E;
    match e {
        E::CoinSelection(inner) => SendError::InsufficientFunds {
            message: inner.to_string(),
        },
        E::OutputBelowDustLimit(_) => SendError::AmountBelowDust,
        other => SendError::BuildFailed {
            message: other.to_string(),
        },
    }
}

/// Builds the send PSBT — shared by the send and (Phase 6.3) estimate paths so
/// an estimate can only diverge from the send by UTXO drift, never by
/// construction. Inputs keep the BDK default sequence (signals BIP-125), so
/// every send is listed and RBF-bumpable via the Phase 5 surfaces.
fn build_send_psbt(
    wallet: &mut bdk_wallet::Wallet,
    dest: &bdk_wallet::bitcoin::Address,
    input: &SendInput,
    rate: FeeRate,
) -> Result<bdk_wallet::bitcoin::Psbt, SendError> {
    let mut builder = wallet.build_tx();
    if input.drain_wallet {
        builder.drain_wallet();
        builder.drain_to(dest.script_pubkey());
    } else {
        builder.add_recipient(
            dest.script_pubkey(),
            bdk_wallet::bitcoin::Amount::from_sat(input.amount_sats),
        );
    }
    builder.fee_rate(rate.to_bdk());
    builder.finish().map_err(map_send_create_tx_error)
}

// ── Use-case ────────────────────────────────────────────────────────────────

impl WalletService {
    /// Sends BTC from the Admin Wallet (PRD §4.3.5): build → sign through the
    /// session [`PsbtSigner`](crate::application::psbt_signer::PsbtSigner) port
    /// → broadcast Electrum-first with node fallback → result with txid.
    ///
    /// Capability guards run **before** any wallet lock or network contact.
    /// A signer failure (step 7) returns `SignFailed` with nothing broadcast —
    /// the §4.3.5.5.1 reject path; the Phase 8 on-device rejection lands on the
    /// same branch. The caller is responsible for syncing the wallet beforehand
    /// (the IPC command does so best-effort, mirroring `admin_wallet_bump_fee`);
    /// a stale view is ultimately caught by the network and surfaces as
    /// `BroadcastFailed`.
    pub async fn send_to_address(
        &self,
        input: &SendInput,
        rate: FeeRate,
        broadcasters: &[Arc<dyn TxBroadcaster>],
    ) -> Result<SendResultDto, SendError> {
        // 1–2. Capability guards.
        let signer = self.signer().ok_or(SendError::ReadOnly)?;
        if !signer.allowed_on(self.network()) {
            return Err(SendError::SignerNotAllowedOnNetwork);
        }

        // 3. Destination — parsed and network-checked before any wallet work.
        let dest = parse_send_destination(&input.address, self.network())?;

        // 4. Amount guard (drain ignores the amount field).
        if !input.drain_wallet && input.amount_sats == 0 {
            return Err(SendError::InvalidAmount);
        }

        // 5. Build.
        let psbt = {
            let mut wallet = self.wallet.lock().await;
            build_send_psbt(&mut wallet, &dest, input, rate)?
        };

        // 6. Sign through the session signer port (same flow as commit funding, R1.1).
        let tx = self
            .sign_and_finalize_psbt(psbt)
            .await
            .map_err(|e| SendError::SignFailed {
                message: e.to_string(),
            })?;

        // 7. Result metadata — all prevouts are wallet-known, so the fee is exact.
        let (fee_sats, change_sats) = {
            let wallet = self.wallet.lock().await;
            let fee = wallet
                .calculate_fee(&tx)
                .map(|fee| fee.to_sat())
                .map_err(|e| SendError::BuildFailed {
                    message: format!("send fee unknown: {e}"),
                })?;
            let change = tx
                .output
                .iter()
                .filter(|out| wallet.is_mine(out.script_pubkey.clone()))
                .map(|out| out.value.to_sat())
                .sum::<u64>();
            (fee, change)
        };
        let vsize_vbytes = tx.vsize() as u64;
        let dest_spk = dest.script_pubkey();
        let amount_sats = if input.drain_wallet {
            tx.output
                .iter()
                .find(|out| out.script_pubkey == dest_spk)
                .map(|out| out.value.to_sat())
                .unwrap_or(0)
        } else {
            input.amount_sats
        };

        // 8. Broadcast: Electrum first, node RPC fallback.
        let tx_hex = bdk_wallet::bitcoin::consensus::encode::serialize_hex(&tx);
        broadcast_single_with_fallback(broadcasters, &tx_hex)
            .await
            .map_err(|errors| SendError::BroadcastFailed {
                message: errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            })?;

        Ok(SendResultDto {
            txid: tx.compute_txid().to_string(),
            amount_sats,
            fee_rate_sat_per_kvb: fee_rate_sat_per_kvb(fee_sats, vsize_vbytes),
            fee_sats,
            vsize_vbytes,
            change_sats,
            drained: input.drain_wallet,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::psbt_signer::MnemonicPsbtSigner;
    use crate::application::tx_broadcaster::tests::MockBroadcaster;
    use crate::infrastructure::admin_wallet::load_admin_wallet;
    use crate::infrastructure::node_config_store::NodeConfig;
    use bdk_wallet::bitcoin::hashes::Hash;
    use bdk_wallet::bitcoin::{BlockHash, Network, Transaction};
    use bdk_wallet::chain::BlockId;
    use bdk_wallet::test_utils::{insert_checkpoint, receive_output_in_latest_block};
    use bdk_wallet::KeychainKind;
    use std::sync::RwLock as StdRwLock;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Deterministic regtest P2WPKH destination (derived, so the checksum is
    /// always valid — hardcoded literals have bitten before).
    fn dest_p2wpkh_regtest() -> String {
        use bdk_wallet::bitcoin::{Address, CompressedPublicKey};
        let pk: CompressedPublicKey =
            "032e58afe51f9ed8ad3cc7897f634d881fdbe49a81564629ded8156bebd2ffd1af"
                .parse()
                .expect("valid pubkey");
        Address::p2wpkh(&pk, Network::Regtest).to_string()
    }

    /// Known mainnet P2WPKH address (BIP-173 test vector).
    const DEST_P2WPKH_MAINNET: &str = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";

    fn test_node_config() -> Arc<StdRwLock<NodeConfig>> {
        Arc::new(StdRwLock::new(NodeConfig::default()))
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

    /// 5 sat/vB.
    fn rate_5_sat_vb() -> FeeRate {
        FeeRate::new(5_000, 1_000).expect("valid rate")
    }

    fn send_input(address: &str, amount_sats: u64) -> SendInput {
        SendInput {
            address: address.to_string(),
            amount_sats,
            fee_rate_sat_per_kvb: 5_000,
            drain_wallet: false,
        }
    }

    fn dest_spk(address: &str) -> bdk_wallet::bitcoin::ScriptBuf {
        parse_send_destination(address, Network::Regtest)
            .expect("valid dest")
            .script_pubkey()
    }

    fn decode_broadcast(mock: &MockBroadcaster) -> Transaction {
        let sent = mock.sent_single();
        assert_eq!(sent.len(), 1, "exactly one tx must be broadcast");
        bdk_wallet::bitcoin::consensus::encode::deserialize_hex(&sent[0])
            .expect("broadcast hex decodes")
    }

    // ── parse_send_destination / network_display_name ───────────────────────

    #[test]
    fn parse_destination_accepts_each_standard_type_on_regtest() {
        use bdk_wallet::bitcoin::{Address, CompressedPublicKey, ScriptBuf};

        let pk: CompressedPublicKey =
            "032e58afe51f9ed8ad3cc7897f634d881fdbe49a81564629ded8156bebd2ffd1af"
                .parse()
                .expect("valid pubkey");
        let redeem = ScriptBuf::from_bytes(vec![0x51]); // OP_TRUE

        let p2pkh = Address::p2pkh(pk, Network::Regtest).to_string();
        let p2sh = Address::p2sh(&redeem, Network::Regtest)
            .expect("p2sh ok")
            .to_string();
        let p2wpkh = Address::p2wpkh(&pk, Network::Regtest).to_string();
        let p2wsh = Address::p2wsh(&redeem, Network::Regtest).to_string();
        let p2tr = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .expect("wallet ok")
            .peek_address(KeychainKind::External, 0)
            .address
            .to_string();

        for (label, addr) in [
            ("p2pkh", p2pkh),
            ("p2sh", p2sh),
            ("p2wpkh", p2wpkh),
            ("p2wsh", p2wsh),
            ("p2tr", p2tr),
        ] {
            assert!(
                parse_send_destination(&addr, Network::Regtest).is_ok(),
                "{label} address {addr} must parse on regtest"
            );
        }
    }

    #[test]
    fn parse_destination_garbage_returns_invalid_address() {
        for garbage in ["not-an-address", "", "   ", "bc1qqqqq"] {
            let result = parse_send_destination(garbage, Network::Regtest);
            assert!(
                matches!(result, Err(SendError::InvalidAddress { .. })),
                "{garbage:?} must be InvalidAddress, got: {result:?}"
            );
        }
    }

    #[test]
    fn parse_destination_mainnet_address_on_regtest_returns_wrong_network() {
        let result = parse_send_destination(DEST_P2WPKH_MAINNET, Network::Regtest);
        match result {
            Err(SendError::WrongNetwork {
                expected_network, ..
            }) => {
                assert_eq!(expected_network, "regtest");
            }
            other => panic!("expected WrongNetwork, got: {other:?}"),
        }
    }

    #[test]
    fn network_display_name_maps_all_networks() {
        assert_eq!(network_display_name(Network::Bitcoin), "mainnet");
        assert_eq!(network_display_name(Network::Testnet), "testnet");
        assert_eq!(network_display_name(Network::Testnet4), "testnet");
        assert_eq!(network_display_name(Network::Signet), "signet");
        assert_eq!(network_display_name(Network::Regtest), "regtest");
    }

    // ── send_to_address — guards ────────────────────────────────────────────

    #[tokio::test]
    async fn send_on_watch_only_returns_read_only_before_any_broadcast() {
        let svc = WalletService::new_watch_only(funded_wallet(), test_node_config());
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .send_to_address(
                &send_input(&dest_p2wpkh_regtest(), 30_000),
                rate_5_sat_vb(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(SendError::ReadOnly)),
            "got: {result:?}"
        );
        assert!(
            mock.sent_single().is_empty(),
            "no broadcaster may be contacted"
        );
    }

    #[tokio::test]
    async fn send_zero_amount_returns_invalid_amount() {
        let svc = signing_service(funded_wallet());
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .send_to_address(
                &send_input(&dest_p2wpkh_regtest(), 0),
                rate_5_sat_vb(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(SendError::InvalidAmount)),
            "got: {result:?}"
        );
        assert!(mock.sent_single().is_empty(), "nothing may be broadcast");
    }

    #[tokio::test]
    async fn send_invalid_destination_fails_before_broadcast() {
        let svc = signing_service(funded_wallet());
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let garbage = svc
            .send_to_address(
                &send_input("not-an-address", 30_000),
                rate_5_sat_vb(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;
        assert!(
            matches!(garbage, Err(SendError::InvalidAddress { .. })),
            "got: {garbage:?}"
        );

        let mainnet = svc
            .send_to_address(
                &send_input(DEST_P2WPKH_MAINNET, 30_000),
                rate_5_sat_vb(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;
        assert!(
            matches!(mainnet, Err(SendError::WrongNetwork { .. })),
            "got: {mainnet:?}"
        );

        assert!(mock.sent_single().is_empty(), "nothing may be broadcast");
    }

    // ── send_to_address — build errors ──────────────────────────────────────

    #[tokio::test]
    async fn send_insufficient_funds_returns_typed_error() {
        let svc = signing_service(funded_wallet());
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .send_to_address(
                &send_input(&dest_p2wpkh_regtest(), 200_000), // wallet holds 100_000
                rate_5_sat_vb(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await;

        assert!(
            matches!(result, Err(SendError::InsufficientFunds { .. })),
            "got: {result:?}"
        );
        assert!(mock.sent_single().is_empty(), "nothing may be broadcast");
    }

    #[tokio::test]
    async fn send_below_dust_returns_amount_below_dust() {
        let svc = signing_service(funded_wallet());

        let result = svc
            .send_to_address(
                &send_input(&dest_p2wpkh_regtest(), 100), // < P2WPKH dust (294 sats)
                rate_5_sat_vb(),
                &mock_chain(&[Arc::new(MockBroadcaster::ok("Electrum"))]),
            )
            .await;

        assert!(
            matches!(result, Err(SendError::AmountBelowDust)),
            "got: {result:?}"
        );
    }

    // ── send_to_address — happy path ────────────────────────────────────────

    #[tokio::test]
    async fn send_happy_path_signs_and_broadcasts() {
        let svc = signing_service(funded_wallet());
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .send_to_address(
                &send_input(&dest_p2wpkh_regtest(), 30_000),
                rate_5_sat_vb(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await
            .expect("send must succeed");

        let tx = decode_broadcast(&mock);
        assert_eq!(result.txid, tx.compute_txid().to_string());
        assert!(!result.drained);
        assert_eq!(result.amount_sats, 30_000);

        let recipient = tx
            .output
            .iter()
            .find(|out| out.script_pubkey == dest_spk(&dest_p2wpkh_regtest()))
            .expect("recipient output present");
        assert_eq!(recipient.value.to_sat(), 30_000);

        // Fee accuracy: single 100_000-sat input wallet ⇒ fee = input − outputs.
        let outputs_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
        assert_eq!(result.fee_sats, 100_000 - outputs_total);
        assert!(result.fee_sats > 0, "send must pay a fee");
        assert_eq!(result.vsize_vbytes, tx.vsize() as u64);
        assert!(
            result.fee_rate_sat_per_kvb >= 5_000,
            "realized rate {} must not undershoot the requested 5 sat/vB (ceiling)",
            result.fee_rate_sat_per_kvb
        );
    }

    #[tokio::test]
    async fn send_change_goes_to_first_unused_internal_index() {
        let wallet = funded_wallet();
        // First unused internal index on a fresh wallet is 0 — peek (non-revealing).
        let expected_change_spk = wallet
            .peek_address(KeychainKind::Internal, 0)
            .address
            .script_pubkey();
        let svc = signing_service(wallet);
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        let result = svc
            .send_to_address(
                &send_input(&dest_p2wpkh_regtest(), 30_000),
                rate_5_sat_vb(),
                &mock_chain(&[Arc::clone(&mock)]),
            )
            .await
            .expect("send must succeed");

        let tx = decode_broadcast(&mock);
        let change = tx
            .output
            .iter()
            .find(|out| out.script_pubkey == expected_change_spk)
            .expect("change output at the first unused internal index (PRD §4.3.5.4)");
        assert_eq!(result.change_sats, change.value.to_sat());
        assert!(result.change_sats > 0, "this send must produce change");
    }

    #[tokio::test]
    async fn send_inputs_signal_rbf() {
        let svc = signing_service(funded_wallet());
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));

        svc.send_to_address(
            &send_input(&dest_p2wpkh_regtest(), 30_000),
            rate_5_sat_vb(),
            &mock_chain(&[Arc::clone(&mock)]),
        )
        .await
        .expect("send must succeed");

        let tx = decode_broadcast(&mock);
        assert!(
            tx.input.iter().all(|input| input.sequence.is_rbf()),
            "every input must signal BIP-125 so Phase 5 RBF bump applies"
        );
    }

    #[tokio::test]
    async fn send_drain_spends_everything_with_no_change() {
        let svc = signing_service(funded_wallet());
        let mock = Arc::new(MockBroadcaster::ok("Electrum"));
        let input = SendInput {
            address: dest_p2wpkh_regtest(),
            amount_sats: 0, // ignored for drain
            fee_rate_sat_per_kvb: 5_000,
            drain_wallet: true,
        };

        let result = svc
            .send_to_address(&input, rate_5_sat_vb(), &mock_chain(&[Arc::clone(&mock)]))
            .await
            .expect("drain send must succeed");

        let tx = decode_broadcast(&mock);
        assert!(result.drained);
        assert_eq!(result.change_sats, 0, "drain builds produce no change");
        assert_eq!(
            tx.output.len(),
            1,
            "drain must pay a single recipient output"
        );
        assert_eq!(tx.output[0].script_pubkey, dest_spk(&dest_p2wpkh_regtest()));
        assert_eq!(result.amount_sats, tx.output[0].value.to_sat());
        assert_eq!(result.amount_sats + result.fee_sats, 100_000);
    }

    // ── send_to_address — broadcast chain ───────────────────────────────────

    #[tokio::test]
    async fn send_falls_back_to_second_broadcaster() {
        let svc = signing_service(funded_wallet());
        let node = Arc::new(MockBroadcaster::ok("Bitcoin node"));
        let chain = mock_chain(&[
            Arc::new(MockBroadcaster::failing("Electrum", "connection refused")),
            Arc::clone(&node),
        ]);

        svc.send_to_address(
            &send_input(&dest_p2wpkh_regtest(), 30_000),
            rate_5_sat_vb(),
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

    #[tokio::test]
    async fn send_all_broadcasters_failing_returns_broadcast_failed() {
        let svc = signing_service(funded_wallet());
        let chain = mock_chain(&[
            Arc::new(MockBroadcaster::failing("Electrum", "connection refused")),
            Arc::new(MockBroadcaster::failing("Bitcoin node", "missing inputs")),
        ]);

        let result = svc
            .send_to_address(
                &send_input(&dest_p2wpkh_regtest(), 30_000),
                rate_5_sat_vb(),
                &chain,
            )
            .await;

        match result {
            Err(SendError::BroadcastFailed { message }) => {
                assert!(message.contains("connection refused"), "got: {message}");
                assert!(message.contains("missing inputs"), "got: {message}");
            }
            other => panic!("expected BroadcastFailed, got: {other:?}"),
        }
    }

    // ── DTO serialization contracts ─────────────────────────────────────────

    #[test]
    fn send_input_deserializes_camel_case_with_drain_default_false() {
        let input: SendInput = serde_json::from_value(serde_json::json!({
            "address": "bcrt1q…",
            "amountSats": 30_000,
            "feeRateSatPerKvb": 5_000
        }))
        .expect("deserialize");
        assert_eq!(input.amount_sats, 30_000);
        assert_eq!(input.fee_rate_sat_per_kvb, 5_000);
        assert!(!input.drain_wallet, "drainWallet must default to false");

        let drain: SendInput = serde_json::from_value(serde_json::json!({
            "address": "bcrt1q…",
            "amountSats": 0,
            "feeRateSatPerKvb": 5_000,
            "drainWallet": true
        }))
        .expect("deserialize");
        assert!(drain.drain_wallet);
    }

    #[test]
    fn send_result_dto_serializes_camel_case() {
        let dto = SendResultDto {
            txid: "ab".into(),
            amount_sats: 30_000,
            fee_sats: 705,
            fee_rate_sat_per_kvb: 5_000,
            vsize_vbytes: 141,
            change_sats: 69_295,
            drained: false,
        };
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["txid"], "ab");
        assert_eq!(json["amountSats"], 30_000);
        assert_eq!(json["feeSats"], 705);
        assert_eq!(json["feeRateSatPerKvb"], 5_000);
        assert_eq!(json["vsizeVbytes"], 141);
        assert_eq!(json["changeSats"], 69_295);
        assert_eq!(json["drained"], false);
    }

    #[test]
    fn send_error_code_covers_every_variant() {
        use crate::domain::fee_rate::FeeRateError;
        let cases: Vec<(SendError, &str)> = vec![
            (SendError::ReadOnly, "ReadOnly"),
            (
                SendError::SignerNotAllowedOnNetwork,
                "SignerNotAllowedOnNetwork",
            ),
            (
                SendError::InvalidAddress {
                    address: "x".into(),
                },
                "InvalidAddress",
            ),
            (
                SendError::WrongNetwork {
                    address: "x".into(),
                    expected_network: "regtest".into(),
                },
                "WrongNetwork",
            ),
            (SendError::InvalidAmount, "InvalidAmount"),
            (SendError::AmountBelowDust, "AmountBelowDust"),
            (
                SendError::InsufficientFunds {
                    message: "m".into(),
                },
                "InsufficientFunds",
            ),
            (
                SendError::InvalidFeeRate(FeeRateError::Zero),
                "InvalidFeeRate",
            ),
            (
                SendError::BuildFailed {
                    message: "m".into(),
                },
                "BuildFailed",
            ),
            (
                SendError::SignFailed {
                    message: "m".into(),
                },
                "SignFailed",
            ),
            (
                SendError::BroadcastFailed {
                    message: "m".into(),
                },
                "BroadcastFailed",
            ),
        ];
        for (err, code) in cases {
            assert_eq!(send_error_code(&err), code);
        }
    }
}
