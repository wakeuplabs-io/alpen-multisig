//! Trezor hardware wallet adapter.
//!
//! Provides taproot key-path signing via Trezor device.

use crate::error::AppError;

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
