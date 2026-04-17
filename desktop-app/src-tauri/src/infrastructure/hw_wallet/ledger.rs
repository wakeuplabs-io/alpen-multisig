use super::HwWalletInfo;
use crate::infrastructure::signing::SignatureResult;

// Ledger implementation is pending Speculos emulator validation.
// The ledger-transport-hid v0.11 API changed significantly from v0.4 (APDUCommand is no longer
// re-exported). Implementation will be revisited once the Speculos Docker setup is in place.
// See docs/2-discovery/07-hardware-wallet-library-analysis.md — Section 7 (Testing Strategy).

pub fn connect(_derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    Err(
        "Ledger implementation pending — validate against Speculos emulator (see doc 07)."
            .to_string(),
    )
}

pub fn sign_message(_sighash_hex: &str, _derivation_path: &str) -> Result<SignatureResult, String> {
    Err(
        "Ledger implementation pending — validate against Speculos emulator (see doc 07)."
            .to_string(),
    )
}
