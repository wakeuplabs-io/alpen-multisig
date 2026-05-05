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
    };

    let rpc_url = asm_status_rpc::default_rpc_url();
    let config = asm_status_rpc::fetch_multisig_config(&rpc_url, role).await?;

    Ok(MultisigConfigDto {
        signers: config.signers,
        threshold: config.threshold,
    })
}
