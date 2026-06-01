//! Hardware wallet PSBT signer — infrastructure adapter.
//!
//! `HwPsbtSigner` implements `PsbtSigner` for hardware wallets (Trezor/Ledger).
//! It stores the master fingerprint captured at connect time and re-opens the
//! device by fingerprint at sign time (no live connection held).

use bitcoin::Network;

use crate::application::psbt_signer::PsbtSigner;
use crate::error::AppError;

/// Driven port: hardware device communication (HID).
///
/// Real implementations (Ledger/Trezor) perform actual USB HID I/O.
/// The device is opened by fingerprint at sign time — no live connection held.
pub(crate) trait HwDevice: Send + Sync {
    /// Sign the PSBT in-place using on-device key material.
    ///
    /// The `fingerprint` is used to re-open the device at sign time.
    fn sign_psbt(&self, fingerprint: u32, psbt: &mut bitcoin::psbt::Psbt) -> Result<(), AppError>;
}

/// Hardware wallet signer that re-opens the device by fingerprint at sign time.
#[allow(dead_code)]
pub(crate) struct HwPsbtSigner {
    master_fingerprint: u32,
    account_xpub: String,
    network: Network,
    device: Box<dyn HwDevice>,
}

impl HwPsbtSigner {
    /// Create a new hardware wallet signer.
    ///
    /// The `master_fingerprint` is captured at connect time — NOT derived from
    /// the xpub's parent_fingerprint.
    ///
    /// Uses a default device stub (panics on sign) — use `with_device` for
    /// testing with a `FakeHwDevice`.
    pub(crate) fn new(
        network: Network,
        account_xpub: &str,
        master_fingerprint: u32,
    ) -> Result<Self, AppError> {
        if account_xpub.is_empty() {
            return Err(AppError::BadRequest(
                "account_xpub must not be empty".to_string(),
            ));
        }

        // Default device stub — real device type (Ledger/Trezor) will be
        // determined by the HW connect flow. For now, this is a placeholder
        // that errors on sign until the device type is resolved.
        let device: Box<dyn HwDevice> = Box::new(StubHwDevice);

        Ok(Self {
            master_fingerprint,
            account_xpub: account_xpub.to_string(),
            network,
            device,
        })
    }

    /// Create a signer with an injectable device (test-only).
    #[cfg(test)]
    pub(crate) fn with_device(
        network: Network,
        account_xpub: &str,
        master_fingerprint: u32,
        device: Box<dyn HwDevice>,
    ) -> Result<Self, AppError> {
        if account_xpub.is_empty() {
            return Err(AppError::BadRequest(
                "account_xpub must not be empty".to_string(),
            ));
        }

        Ok(Self {
            master_fingerprint,
            account_xpub: account_xpub.to_string(),
            network,
            device,
        })
    }
}

impl PsbtSigner for HwPsbtSigner {
    fn sign_psbt(
        &self,
        _wallet: &mut bdk_wallet::Wallet,
        psbt: &mut bitcoin::psbt::Psbt,
    ) -> Result<(), AppError> {
        // Re-open device by fingerprint and sign (spawn_blocking + 60s timeout
        // applied at the application-layer call site).
        self.device.sign_psbt(self.master_fingerprint, psbt)
    }

    fn allowed_on(&self, _network: Network) -> bool {
        // Hardware wallets are allowed on all networks
        true
    }
}

/// Default stub device — errors on sign until real device type is resolved.
struct StubHwDevice;

impl HwDevice for StubHwDevice {
    fn sign_psbt(
        &self,
        _fingerprint: u32,
        _psbt: &mut bitcoin::psbt::Psbt,
    ) -> Result<(), AppError> {
        Err(AppError::Internal(anyhow::anyhow!(
            "hardware device type not yet resolved — use with_device() for testing"
        )))
    }
}

// ---------------------------------------------------------------------------
// Test doubles
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bdk_chain::{BlockId, ConfirmationBlockTime};
    use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv};
    use bdk_wallet::bitcoin::hashes::Hash;
    use bdk_wallet::bitcoin::secp256k1::Secp256k1;
    use bdk_wallet::bitcoin::{Amount, BlockHash, Transaction, TxOut};
    use bdk_wallet::test_utils::{insert_anchor, insert_checkpoint, insert_tx};
    use bdk_wallet::KeychainKind;
    use bip39::Mnemonic;
    use std::str::FromStr;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Fake hardware device that simulates successful on-device signing.
    struct FakeHwDevice;

    impl HwDevice for FakeHwDevice {
        fn sign_psbt(
            &self,
            _fingerprint: u32,
            psbt: &mut bitcoin::psbt::Psbt,
        ) -> Result<(), AppError> {
            // Simulate taproot key-path signing: add a dummy witness to each input.
            // A real device would produce a 64-byte Schnorr signature.
            for input in &mut psbt.inputs {
                let dummy_sig = bitcoin::secp256k1::schnorr::Signature::from_slice(&[0u8; 64])
                    .expect("valid dummy sig");
                input.tap_key_sig = Some(bitcoin::taproot::Signature {
                    signature: dummy_sig,
                    sighash_type: bitcoin::sighash::TapSighashType::Default,
                });
            }
            Ok(())
        }
    }

    fn build_test_wallet(network: bitcoin::Network) -> bdk_wallet::Wallet {
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
    fn test_hw_psbt_signer_happy_path_with_fake_device() {
        // Given: Hardware signer with a fake device that simulates signing success
        let device: Box<dyn HwDevice> = Box::new(FakeHwDevice);
        let signer = HwPsbtSigner::with_device(
            Network::Regtest,
            "tpubD6NzVbkrYhZ4X8L36T1DKRzVJQKJH7YbF3xGqVz5k3Z9w8R7T6Y5X4W3V2U1S0",
            0x12345678,
            device,
        )
        .expect("signer must be created");

        let mut wallet = build_test_wallet(Network::Regtest);
        let change_addr = wallet.peek_address(KeychainKind::Internal, 0).address;

        let mut tx_builder = wallet.build_tx();
        tx_builder.add_recipient(change_addr.script_pubkey(), Amount::from_sat(50_000));
        tx_builder.fee_rate(bitcoin::FeeRate::from_sat_per_vb(1).unwrap());
        let mut psbt = tx_builder.finish().expect("PSBT must build");

        // When: sign_psbt is called through the driving port (PsbtSigner trait)
        let result = signer.sign_psbt(&mut wallet, &mut psbt);

        // Then: signing succeeds and PSBT has taproot key-path signatures
        assert!(
            result.is_ok(),
            "sign_psbt must succeed with fake device: {:?}",
            result.err()
        );
        assert!(
            psbt.inputs.iter().all(|i| i.tap_key_sig.is_some()),
            "each PSBT input must have a tap_key_sig after HW signing"
        );
    }
}
