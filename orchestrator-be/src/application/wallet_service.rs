//! Wallet service — application service (driving port).
//!
//! `WalletService` holds an optional signer and reports signing capability
//! based on whether the attached signer is allowed on the wallet's network.

use std::sync::Arc;

use bitcoin::Network;

use crate::application::psbt_signer::PsbtSigner;
use crate::infrastructure::admin_wallet::wallet::AdminWalletError;

/// Application service that manages signing capability for a wallet.
pub(crate) struct WalletService {
    #[allow(dead_code)]
    network: Network,
    signer: Option<Arc<dyn PsbtSigner>>,
}

#[allow(dead_code)]
impl WalletService {
    pub(crate) fn new(network: Network, signer: Option<Arc<dyn PsbtSigner>>) -> Self {
        Self { network, signer }
    }

    /// Whether the session has signing capability: signer present AND
    /// the signer is allowed on this wallet's network.
    pub(crate) fn can_sign(&self) -> bool {
        self.signer
            .as_ref()
            .map(|s| s.allowed_on(self.network))
            .unwrap_or(false)
    }

    /// Build a signed commit transaction.
    ///
    /// Flow: build_psbt → signer.sign_psbt → finalize → extract_tx.
    /// Returns the extracted, signed commit transaction ready for broadcast.
    pub(crate) fn build_signed_commit(
        &self,
        wallet: &mut bdk_wallet::Wallet,
        commit_addr: bitcoin::Address,
        amount_sats: u64,
        fee_rate_sats_per_vb: u64,
    ) -> Result<bitcoin::Transaction, AdminWalletError> {
        let signer = self.signer.as_ref().ok_or(AdminWalletError::ReadOnly)?;
        if !signer.allowed_on(self.network) {
            return Err(AdminWalletError::SignerNotAllowedOnNetwork {
                network: self.network,
            });
        }

        let mut tx_builder = wallet.build_tx();
        tx_builder.add_recipient(
            commit_addr.script_pubkey(),
            bitcoin::Amount::from_sat(amount_sats),
        );
        let fee_rate = bitcoin::FeeRate::from_sat_per_vb(fee_rate_sats_per_vb)
            .unwrap_or(bitcoin::FeeRate::BROADCAST_MIN);
        tx_builder.fee_rate(fee_rate);

        let mut psbt = tx_builder
            .finish()
            .map_err(|e| AdminWalletError::PsbtBuild(e.to_string()))?;

        signer
            .sign_psbt(wallet, &mut psbt)
            .map_err(|e| AdminWalletError::SigningFailed(e.to_string()))?;

        // Finalize inputs (BDK finalize_psbt returns bool indicating success)
        let finalized = wallet
            .finalize_psbt(&mut psbt, Default::default())
            .map_err(|e| AdminWalletError::FinalizeFailed(e.to_string()))?;

        if !finalized {
            return Err(AdminWalletError::FinalizeFailed(
                "PSBT not fully finalized".to_string(),
            ));
        }

        psbt.extract_tx()
            .map_err(|e| AdminWalletError::ExtractFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::application::psbt_signer::MnemonicPsbtSigner;
    use bdk_wallet::KeychainKind;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn build_test_wallet(network: Network) -> bdk_wallet::Wallet {
        use bdk_chain::{BlockId, ConfirmationBlockTime};
        use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv};
        use bdk_wallet::bitcoin::hashes::Hash;
        use bdk_wallet::bitcoin::secp256k1::Secp256k1;
        use bdk_wallet::bitcoin::{Amount, BlockHash, Transaction, TxOut};
        use bdk_wallet::test_utils::{insert_anchor, insert_checkpoint, insert_tx};
        use bdk_wallet::KeychainKind;
        use bip39::Mnemonic;
        use std::str::FromStr;

        let mnemonic = Mnemonic::parse(TEST_MNEMONIC).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();
        let xpriv = Xpriv::new_master(network, &seed).expect("master key");
        let path = DerivationPath::from_str("m/86'/0'/73'").expect("valid path");
        let account_xpriv = xpriv.derive_priv(&secp, &path).expect("derive");

        let external_desc = format!("tr({}/0/*)", account_xpriv);
        let internal_desc = format!("tr({}/1/*)", account_xpriv);

        let mut wallet = bdk_wallet::Wallet::create(external_desc, internal_desc)
            .network(network)
            .create_wallet_no_persist()
            .expect("wallet creation");

        // Fund the wallet with a confirmed UTXO
        let receive_addr = wallet.peek_address(KeychainKind::External, 0).address;
        let funding_tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![TxOut {
                value: Amount::from_sat(100_000),
                script_pubkey: receive_addr.script_pubkey(),
            }],
        };

        // Insert checkpoint and transaction as confirmed
        insert_checkpoint(
            &mut wallet,
            BlockId {
                height: 100,
                hash: BlockHash::all_zeros(),
            },
        );
        insert_tx(&mut wallet, funding_tx.clone());
        insert_anchor(
            &mut wallet,
            funding_tx.compute_txid(),
            ConfirmationBlockTime {
                block_id: BlockId {
                    height: 100,
                    hash: BlockHash::all_zeros(),
                },
                confirmation_time: 1000,
            },
        );

        wallet
    }

    #[test]
    fn test_wallet_service_can_sign_reflects_network_capability() {
        // Signer present AND allowed on network → can_sign = true
        let signer = Arc::new(MnemonicPsbtSigner::new(Network::Regtest, TEST_MNEMONIC).unwrap());
        let svc = WalletService::new(Network::Regtest, Some(signer));
        assert!(
            svc.can_sign(),
            "signer present + allowed on regtest → can_sign=true"
        );

        // Signer present but NOT allowed on network → can_sign = false
        let signer = Arc::new(MnemonicPsbtSigner::new(Network::Regtest, TEST_MNEMONIC).unwrap());
        let svc = WalletService::new(Network::Bitcoin, Some(signer));
        assert!(
            !svc.can_sign(),
            "signer present but rejected on mainnet → can_sign=false"
        );

        // No signer → can_sign = false
        let svc: WalletService = WalletService::new(Network::Regtest, None);
        assert!(!svc.can_sign(), "no signer → can_sign=false");
    }

    #[test]
    fn test_build_signed_commit_no_signer_returns_readonly() {
        // HW session present but no signer attached → ReadOnly error
        let svc: WalletService = WalletService::new(Network::Regtest, None);
        let mut wallet = build_test_wallet(Network::Regtest);
        let commit_addr = wallet.peek_address(KeychainKind::External, 1).address;
        let result = svc.build_signed_commit(&mut wallet, commit_addr, 50_000, 1);
        assert!(
            matches!(result, Err(AdminWalletError::ReadOnly)),
            "no signer attached → build_signed_commit returns ReadOnly"
        );
    }

    #[test]
    fn test_mnemonic_rejected_on_mainnet_fail_fast() {
        // Mnemonic signer present but wallet is on mainnet → SignerNotAllowedOnNetwork
        // Must fail BEFORE any sync/RPC/PSBT build (no network I/O)
        let signer = Arc::new(MnemonicPsbtSigner::new(Network::Regtest, TEST_MNEMONIC).unwrap());
        let svc = WalletService::new(Network::Bitcoin, Some(signer));
        let mut wallet = build_test_wallet(Network::Bitcoin);
        let commit_addr = wallet.peek_address(KeychainKind::External, 1).address;
        let result = svc.build_signed_commit(&mut wallet, commit_addr, 50_000, 1);
        assert!(
            matches!(result, Err(AdminWalletError::SignerNotAllowedOnNetwork { .. })),
            "mnemonic signer on mainnet → build_signed_commit returns SignerNotAllowedOnNetwork before any I/O"
        );
    }

    #[test]
    fn test_mnemonic_on_regtest_without_env_flag_succeeds() {
        // Mnemonic signer on regtest — no ALLOW_DEV_MNEMONIC_SIGNING env var needed.
        // The per-signer network capability (allowed_on) replaces the legacy env flag.
        // Broadcast must succeed purely based on signer capability.
        let signer = Arc::new(MnemonicPsbtSigner::new(Network::Regtest, TEST_MNEMONIC).unwrap());
        let svc = WalletService::new(Network::Regtest, Some(signer));
        let mut wallet = build_test_wallet(Network::Regtest);

        let commit_addr = wallet.peek_address(KeychainKind::External, 1).address;
        let result = svc.build_signed_commit(&mut wallet, commit_addr, 50_000, 1);
        assert!(
            result.is_ok(),
            "mnemonic signer on regtest without ALLOW_DEV_MNEMONIC_SIGNING → build_signed_commit succeeds: {:?}",
            result.err()
        );
    }

    // --- Step 01-07: Mnemonic happy path — build_signed_commit produces valid extractable tx ---

    #[test]
    fn test_build_signed_commit_mnemonic_happy_path_regtest() {
        // Given: Mnemonic-initialized session on regtest with a funded wallet
        let signer = Arc::new(MnemonicPsbtSigner::new(Network::Regtest, TEST_MNEMONIC).unwrap());
        let svc = WalletService::new(Network::Regtest, Some(signer));
        let mut wallet = build_test_wallet(Network::Regtest);

        // When: build_signed_commit is called
        let commit_addr = wallet.peek_address(KeychainKind::External, 1).address;

        let result = svc.build_signed_commit(&mut wallet, commit_addr, 50_000, 1);

        // Then: produces a valid, extractable commit transaction
        assert!(
            result.is_ok(),
            "mnemonic signer on regtest with funded wallet → build_signed_commit succeeds: {:?}",
            result.err()
        );
        let tx = result.unwrap();
        assert!(
            !tx.input.is_empty(),
            "commit tx must have at least one input"
        );
        assert!(
            !tx.output.is_empty(),
            "commit tx must have at least one output"
        );
        // The tx should be signed (inputs have witness data)
        assert!(
            tx.input.iter().any(|i| !i.witness.is_empty()),
            "commit tx inputs must have witness data (signed)"
        );
    }
}
