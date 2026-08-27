use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use desktop_app::domain::auth::AuthRole;
use desktop_app::domain::authority::Authority;
use desktop_app::infrastructure::asm_status_rpc;
use desktop_app::infrastructure::bitcoin_rpc::BitcoinRpcClient;
use desktop_app::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient;
use desktop_app::infrastructure::node_config_store::NodeConfigState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultisigConfigDto {
    pub signers: Vec<String>,
    pub threshold: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentVkDto {
    pub type_id: u8,
    pub type_name: String,
    pub condition_hex: String,
}

#[tauri::command]
pub async fn get_multisig_config(
    authority: String,
    node_config: State<'_, NodeConfigState>,
) -> Result<MultisigConfigDto, String> {
    let parsed = Authority::from_wire(authority.trim())
        .map_err(|e| format!("invalid authority `{}`: {e}", authority))?;

    let role = match parsed {
        Authority::StrataAdmin => AuthRole::StrataAdministrator,
        Authority::SequencerManager => AuthRole::StrataSequencerManager,
        Authority::AlpenAdmin => AuthRole::AlpenAdministrator,
        Authority::SecurityCouncil => AuthRole::StrataSecurityCouncil,
        // Listed rather than caught by `_`: a catch-all is how the council reached this arm
        // silently in the first place, and the next authority added should be a compile error.
        Authority::PayoutAdmin => {
            return Err(format!(
                "authority `{}` is not supported in the desktop app yet",
                parsed.as_str()
            ));
        }
    };

    let rpc_url = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .strata_rpc_url()
        .to_string();
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
    let rpc_url = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .strata_rpc_url()
        .to_string();
    asm_status_rpc::fetch_current_operators(&rpc_url).await
}

#[tauri::command]
pub async fn get_current_vk(
    node_config: State<'_, NodeConfigState>,
) -> Result<CurrentVkDto, String> {
    let rpc_url = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .strata_rpc_url()
        .to_string();
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
    let rpc_url = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .strata_rpc_url()
        .to_string();
    let (role_to_keys, _) = asm_status_rpc::fetch_role_membership(&rpc_url).await?;

    let mut result = HashMap::new();
    for (role, keys) in &role_to_keys {
        let is_member = keys.iter().any(|k| k.eq_ignore_ascii_case(&pubkey_hex));
        result.insert(role.as_wire_str().to_string(), is_member);
    }
    Ok(result)
}
