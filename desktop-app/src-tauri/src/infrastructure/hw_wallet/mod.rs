pub mod ledger;
pub mod trezor;

use serde::Serialize;

/// Device info returned to the WebView on connect.
/// Field names are camelCase to match the WalletAccountInfo TypeScript type.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HwWalletInfo {
    pub device_label: String,
    pub derivation_path: String,
    pub address_sample: Option<String>,
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
