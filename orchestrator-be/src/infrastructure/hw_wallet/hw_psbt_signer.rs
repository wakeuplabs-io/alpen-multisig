//! Hardware wallet PSBT signer — infrastructure adapter.
//!
//! `HwPsbtSigner` implements `PsbtSigner` for hardware wallets (Trezor/Ledger).
//! It stores the master fingerprint captured at connect time and re-opens the
//! device by fingerprint at sign time (no live connection held).

use bitcoin::Network;

use crate::application::psbt_signer::PsbtSigner;
use crate::error::AppError;

/// Hardware wallet signer that re-opens the device by fingerprint at sign time.
#[allow(dead_code)]
pub(crate) struct HwPsbtSigner {
    master_fingerprint: u32,
    account_xpub: String,
    network: Network,
}

impl HwPsbtSigner {
    /// Create a new hardware wallet signer.
    ///
    /// The `master_fingerprint` is captured at connect time — NOT derived from
    /// the xpub's parent_fingerprint.
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

        Ok(Self {
            master_fingerprint,
            account_xpub: account_xpub.to_string(),
            network,
        })
    }
}

impl PsbtSigner for HwPsbtSigner {
    fn sign_psbt(
        &self,
        _wallet: &mut bdk_wallet::Wallet,
        _psbt: &mut bitcoin::psbt::Psbt,
    ) -> Result<(), AppError> {
        // TODO: re-open device by fingerprint and sign
        Err(AppError::Internal(anyhow::anyhow!(
            "hardware signing not yet implemented"
        )))
    }

    fn allowed_on(&self, _network: Network) -> bool {
        // Hardware wallets are allowed on all networks
        true
    }
}
