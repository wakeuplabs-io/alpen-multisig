pub mod hw_psbt_signer;
pub mod ledger;
pub mod trezor;

use serde::Serialize;

/// Address script type to confirm on a hardware device during verify-on-device.
///
/// The Admin Wallet receive address is BIP-86 / P2TR (`Taproot`); the Admin ID is
/// BIP-84 / P2WPKH (`WitnessPubkeyHash`). Adapters map this to their device-native
/// script encoding (Trezor `InputScriptType`, Ledger display call).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressScriptType {
    /// BIP-86 / P2TR — the Admin Wallet receive keychain (PRD §4.3.4.2).
    Taproot,
    /// BIP-84 / P2WPKH — the Admin ID authentication key (PRD §4.2).
    WitnessPubkeyHash,
}

impl AddressScriptType {
    /// Parses the IPC string token (`"p2tr"` | `"p2wpkh"`); case-insensitive.
    pub fn parse(token: &str) -> Result<Self, String> {
        match token.trim().to_ascii_lowercase().as_str() {
            "p2tr" | "taproot" => Ok(Self::Taproot),
            "p2wpkh" | "witness" => Ok(Self::WitnessPubkeyHash),
            other => Err(format!(
                "unknown script type '{other}' (expected p2tr or p2wpkh)"
            )),
        }
    }
}

/// Device info returned to the WebView on connect.
/// Field names are camelCase to match the WalletAccountInfo TypeScript type.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HwWalletInfo {
    pub device_label: String,
    pub derivation_path: String,
    pub address_sample: Option<String>,
    /// Full compressed secp256k1 public key (hex). Used for ASM membership checks.
    pub public_key_hex: Option<String>,
    pub xpub_or_fingerprint: Option<String>,
    pub key_label: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HwAddressEntry {
    pub index: u32,
    pub derivation_path: String,
    pub address: String,
    pub public_key_hex: String,
}

#[cfg(test)]
mod tests {
    use super::AddressScriptType;

    #[test]
    fn parse_taproot_tokens() {
        assert_eq!(
            AddressScriptType::parse("p2tr"),
            Ok(AddressScriptType::Taproot)
        );
        assert_eq!(
            AddressScriptType::parse("P2TR"),
            Ok(AddressScriptType::Taproot)
        );
        assert_eq!(
            AddressScriptType::parse("taproot"),
            Ok(AddressScriptType::Taproot)
        );
    }

    #[test]
    fn parse_witness_tokens() {
        assert_eq!(
            AddressScriptType::parse("p2wpkh"),
            Ok(AddressScriptType::WitnessPubkeyHash)
        );
        assert_eq!(
            AddressScriptType::parse(" P2WPKH "),
            Ok(AddressScriptType::WitnessPubkeyHash)
        );
    }

    #[test]
    fn parse_unknown_token_is_error() {
        assert!(AddressScriptType::parse("p2sh").is_err());
        assert!(AddressScriptType::parse("").is_err());
    }
}
