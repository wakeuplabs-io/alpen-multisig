use std::collections::HashMap;

use serde::Serialize;

use desktop_app::domain::auth::AuthRole;
use desktop_app::domain::authority::Authority;
use desktop_app::infrastructure::asm_status_rpc;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultisigConfigDto {
    pub signers: Vec<String>,
    pub threshold: u8,
}

#[tauri::command]
pub async fn get_multisig_config(authority: String) -> Result<MultisigConfigDto, String> {
    let parsed = Authority::from_wire(authority.trim())
        .map_err(|e| format!("invalid authority `{}`: {e}", authority))?;

    let role = match parsed {
        Authority::StrataAdmin => AuthRole::StrataAdministrator,
        Authority::SequencerManager => AuthRole::StrataSequencerManager,
        Authority::AlpenAdmin => AuthRole::AlpenAdministrator,
        _ => {
            return Err(format!(
                "authority `{}` is not supported in the desktop app yet",
                parsed.as_str()
            ));
        }
    };

    let rpc_url = asm_status_rpc::default_rpc_url();
    let config = asm_status_rpc::fetch_multisig_config(&rpc_url, role).await?;

    Ok(MultisigConfigDto {
        signers: config.signers,
        threshold: config.threshold,
    })
}

#[tauri::command]
pub async fn check_authority_memberships(
    pubkey_hex: String,
) -> Result<HashMap<String, bool>, String> {
    let rpc_url = asm_status_rpc::default_rpc_url();
    let (role_to_keys, _) = asm_status_rpc::fetch_role_membership(&rpc_url).await?;

    let mut result = HashMap::new();
    for (role, keys) in &role_to_keys {
        let is_member = keys.iter().any(|k| k.eq_ignore_ascii_case(&pubkey_hex));
        result.insert(role.as_wire_str().to_string(), is_member);
    }
    Ok(result)
}
