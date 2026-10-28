use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use desktop_app::domain::action::SafeHarborDescriptor;
use desktop_app::domain::auth::AuthRole;
use desktop_app::domain::authority::Authority;
use desktop_app::domain::signer_id::SignerId;
use desktop_app::infrastructure::asm_status_rpc;
use desktop_app::infrastructure::bitcoin_rpc::BitcoinRpcClient;
use desktop_app::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient;
use desktop_app::infrastructure::network_env;
use desktop_app::infrastructure::node_config_store::NodeConfigState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultisigConfigDto {
    pub signers: Vec<String>,
    pub threshold: u8,
}

/// The bridge's safe harbor, in both the form a signer recognises and the form their device
/// shows.
///
/// `address_hex` is the BOSD descriptor — what upstream renders into the signing message — and
/// `address` is the same destination written for the active network. Both travel because they
/// answer different questions: one is comparable against a device screen, the other against a
/// wallet.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeHarborStatusDto {
    pub activated: bool,
    pub address_hex: String,
    pub address: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentVkDto {
    pub type_id: u8,
    pub type_name: String,
    pub condition_hex: String,
}

fn strata_rpc_url(node_config: &State<'_, NodeConfigState>) -> Result<String, String> {
    Ok(node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .strata_rpc_url()
        .to_string())
}

#[tauri::command]
pub async fn get_multisig_config(
    authority: String,
    node_config: State<'_, NodeConfigState>,
) -> Result<MultisigConfigDto, String> {
    let parsed = Authority::from_wire(authority.trim())
        .map_err(|e| format!("invalid authority `{}`: {e}", authority))?;

    let role = AuthRole::for_authority(parsed).ok_or_else(|| {
        format!(
            "authority `{}` is not supported in the desktop app yet",
            parsed.as_str()
        )
    })?;

    let rpc_url = strata_rpc_url(&node_config)?;
    let config = asm_status_rpc::fetch_multisig_config(&rpc_url, role).await?;

    Ok(MultisigConfigDto {
        signers: config.signers,
        threshold: config.threshold,
    })
}

#[tauri::command]
pub async fn get_current_operators(
    node_config: State<'_, NodeConfigState>,
) -> Result<Vec<String>, String> {
    let rpc_url = strata_rpc_url(&node_config)?;
    asm_status_rpc::fetch_current_operators(&rpc_url).await
}

#[tauri::command]
pub async fn get_safe_harbor_status(
    node_config: State<'_, NodeConfigState>,
) -> Result<SafeHarborStatusDto, String> {
    let rpc_url = strata_rpc_url(&node_config)?;
    let safe_harbor = asm_status_rpc::fetch_safe_harbor(&rpc_url).await?;
    // A network that cannot be resolved blanks only the address: the hex is what the device shows
    // and what the no-op rule compares, so it must survive a misconfigured environment.
    let address = network_env::network_from_env()
        .ok()
        .and_then(|network| {
            SafeHarborDescriptor::from_hex(&safe_harbor.address_hex)
                .ok()
                .map(|d| d.to_address(network))
        })
        .unwrap_or_default();
    Ok(SafeHarborStatusDto {
        activated: safe_harbor.activated,
        address_hex: safe_harbor.address_hex,
        address,
    })
}

#[tauri::command]
pub async fn get_current_vk(
    node_config: State<'_, NodeConfigState>,
) -> Result<CurrentVkDto, String> {
    let rpc_url = strata_rpc_url(&node_config)?;
    let vk = asm_status_rpc::fetch_current_vk(&rpc_url).await?;
    Ok(CurrentVkDto {
        type_id: vk.type_id,
        type_name: vk.type_name,
        condition_hex: vk.condition_hex,
    })
}

#[tauri::command]
pub async fn get_bitcoin_block_height(
    node_config: State<'_, NodeConfigState>,
) -> Result<u64, String> {
    let cfg = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();
    // getblockcount is a node-level RPC — no bitcoind Core wallet needed.
    let btc_rpc =
        HttpBitcoinRpcClient::new(cfg.btc_rpc_url(), cfg.btc_rpc_user(), cfg.btc_rpc_pass());
    btc_rpc.get_block_count().await
}

#[tauri::command]
pub async fn check_authority_memberships(
    pubkey_hex: String,
    node_config: State<'_, NodeConfigState>,
) -> Result<HashMap<String, bool>, String> {
    let rpc_url = strata_rpc_url(&node_config)?;
    let (role_to_admin_ids, _) = asm_status_rpc::fetch_role_membership(&rpc_url).await?;
    Ok(memberships(&role_to_admin_ids, &pubkey_hex))
}

/// Whether the key's signer is in each role, compared by witness program: the ASM lists Admin IDs.
fn memberships(
    role_to_admin_ids: &HashMap<AuthRole, Vec<String>>,
    pubkey_hex: &str,
) -> HashMap<String, bool> {
    let signer = SignerId::from_pubkey_hex(pubkey_hex);
    role_to_admin_ids
        .iter()
        .map(|(role, admin_ids)| {
            let is_member = signer.is_some()
                && admin_ids
                    .iter()
                    .any(|id| SignerId::from_admin_id(id) == signer);
            (role.as_wire_str().to_string(), is_member)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator point and its Admin ID on regtest (BIP-173 vector).
    const KEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    const ADMIN_ID: &str = "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080";

    #[test]
    fn a_key_is_a_member_where_its_admin_id_is_listed() {
        let roles = HashMap::from([
            (AuthRole::StrataAdministrator, vec![ADMIN_ID.to_string()]),
            (AuthRole::AlpenAdministrator, vec![]),
        ]);
        let result = memberships(&roles, KEY);
        assert_eq!(result.get("strata_administrator"), Some(&true));
        assert_eq!(result.get("alpen_administrator"), Some(&false));
    }

    #[test]
    fn an_invalid_key_is_a_member_of_nothing() {
        let roles = HashMap::from([(AuthRole::StrataAdministrator, vec![ADMIN_ID.to_string()])]);
        assert_eq!(
            memberships(&roles, "not a key").get("strata_administrator"),
            Some(&false)
        );
    }
}
