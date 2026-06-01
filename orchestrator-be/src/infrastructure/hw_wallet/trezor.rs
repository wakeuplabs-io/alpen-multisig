//! Trezor hardware wallet adapter.
//!
//! Provides taproot key-path signing via Trezor device.

use crate::error::AppError;
use crate::infrastructure::hw_wallet::hw_psbt_signer::HwDevice;

/// Adapter for Trezor hardware wallet communication.
pub(crate) struct TrezorAdapter;

#[allow(dead_code)]
impl TrezorAdapter {
    pub(crate) fn new() -> Self {
        Self
    }

    /// Sign a PSBT using taproot key-path on Trezor device.
    pub(crate) fn sign_psbt(
        &self,
        _psbt: &mut bitcoin::psbt::Psbt,
        _fingerprint: u32,
    ) -> Result<(), AppError> {
        Err(AppError::Internal(anyhow::anyhow!(
            "Trezor signing not yet implemented"
        )))
    }
}

impl HwDevice for TrezorAdapter {
    fn device_fingerprint(&self) -> u32 {
        // TODO: query actual device fingerprint via Trezor HID protocol
        0
    }

    fn sign_psbt(&self, fingerprint: u32, psbt: &mut bitcoin::psbt::Psbt) -> Result<(), AppError> {
        self.sign_psbt(psbt, fingerprint)
    }
}
