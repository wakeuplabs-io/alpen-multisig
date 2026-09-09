use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use desktop_app::domain::action::SafeHarbourDescriptor;
use desktop_app::domain::auth::AuthRole;
use desktop_app::domain::authority::Authority;
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

/// The bridge's safe harbour, in both the form a signer recognises and the form their device
/// shows.
///
/// `address_hex` is the BOSD descriptor — what upstream renders into the signing message — and
/// `address` is the same destination written for the active network. Both travel because they
/// answer different questions: one is comparable against a device screen, the other against a
/// wallet.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeHarbourStatusDto {
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
pub async fn get_safe_harbour_status(
    node_config: State<'_, NodeConfigState>,
) -> Result<SafeHarbourStatusDto, String> {
    let rpc_url = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .strata_rpc_url()
        .to_string();
    let safe_harbour = asm_status_rpc::fetch_safe_harbour(&rpc_url).await?;
    // A network that cannot be resolved blanks only the address: the hex is what the device shows
    // and what the no-op rule compares, so it must survive a misconfigured environment.
    let address = network_env::network_from_env()
        .ok()
        .and_then(|network| {
            SafeHarbourDescriptor::from_hex(&safe_harbour.address_hex)
                .ok()
                .map(|d| d.to_address(network))
        })
        .unwrap_or_default();
    Ok(SafeHarbourStatusDto {
        activated: safe_harbour.activated,
        address_hex: safe_harbour.address_hex,
        address,
    })
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
