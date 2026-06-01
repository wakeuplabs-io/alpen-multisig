//! Ledger hardware wallet adapter.
//!
//! Provides taproot key-path signing via Ledger device.

use crate::error::AppError;
use crate::infrastructure::hw_wallet::hw_psbt_signer::HwDevice;

/// Adapter for Ledger hardware wallet communication.
pub(crate) struct LedgerAdapter;

#[allow(dead_code)]
impl LedgerAdapter {
    pub(crate) fn new() -> Self {
        Self
    }

    /// Sign a PSBT using taproot key-path on Ledger device.
    pub(crate) fn sign_psbt(
        &self,
        _psbt: &mut bitcoin::psbt::Psbt,
        _fingerprint: u32,
    ) -> Result<(), AppError> {
        Err(AppError::Internal(anyhow::anyhow!(
            "Ledger signing not yet implemented"
        )))
    }
}

impl HwDevice for LedgerAdapter {
    fn sign_psbt(&self, fingerprint: u32, psbt: &mut bitcoin::psbt::Psbt) -> Result<(), AppError> {
        self.sign_psbt(psbt, fingerprint)
    }
}
