//! Admin IDs for the signers the app lists (approvals, signer-set changes, "Sent by").
//!
//! The UI only holds signers as compressed public keys. It asks here for the Admin ID of each one, so
//! the P2WPKH derivation stays in Rust, rendered on the chain's network.

use std::collections::HashMap;

use desktop_app::domain::signer_id::SignerId;
use desktop_app::infrastructure::network_env;

/// Admin ID per signer key, keyed by the key as the UI normalises it (lowercase, no `0x`). A key
/// that is not a valid compressed public key is left out rather than failing the whole batch.
#[tauri::command]
pub fn admin_ids_for_pubkeys(pubkeys: Vec<String>) -> Result<HashMap<String, String>, String> {
    let network = network_env::network_from_env().map_err(|e| e.to_string())?;
    Ok(admin_ids(&pubkeys, network))
}

fn admin_ids(pubkeys: &[String], network: bitcoin::Network) -> HashMap<String, String> {
    pubkeys
        .iter()
        .filter_map(|pubkey| {
            let trimmed = pubkey.trim();
            let hex = trimmed
                .strip_prefix("0x")
                .or_else(|| trimmed.strip_prefix("0X"))
                .unwrap_or(trimmed)
                .to_lowercase();
            let signer = SignerId::from_pubkey_hex(&hex)?;
            Some((hex, signer.admin_id(network)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    #[test]
    fn keys_the_admin_id_by_the_normalised_pubkey() {
        let ids = admin_ids(
            &[format!("0x{}", KEY.to_uppercase()), "not a key".to_string()],
            bitcoin::Network::Regtest,
        );
        assert_eq!(ids.len(), 1, "the invalid key is left out");
        assert_eq!(
            ids.get(KEY).map(String::as_str),
            Some("bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080")
        );
    }
}
