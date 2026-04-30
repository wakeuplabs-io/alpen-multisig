use std::num::NonZeroU8;

use desktop_app::domain::action::{Action, CompressedPubKey, MultisigUpdate};
use desktop_app::domain::authority::Authority;
use desktop_app::infrastructure::action_codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildAdminMultisigUpdateHexInput {
    pub role: String,
    pub add_keys: Vec<String>,
    pub remove_keys: Vec<String>,
    pub new_threshold: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildActionHexResponse {
    pub action_hex: String,
}

#[tauri::command]
pub fn build_admin_multisig_update_hex(
    input: BuildAdminMultisigUpdateHexInput,
) -> Result<BuildActionHexResponse, String> {
    let authority = Authority::from_wire(input.role.trim())
        .map_err(|e| format!("invalid role `{}`: {e}", input.role))?;
    let new_threshold = NonZeroU8::new(input.new_threshold)
        .ok_or_else(|| "newThreshold must be > 0".to_string())?;

    let add_keys = input
        .add_keys
        .iter()
        .map(|k| CompressedPubKey::from_hex(k.trim()).map_err(|e| format!("invalid add key: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
    let remove_keys = input
        .remove_keys
        .iter()
        .map(|k| {
            CompressedPubKey::from_hex(k.trim()).map_err(|e| format!("invalid remove key: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let action = Action::MultisigUpdate(MultisigUpdate {
        role: authority,
        add_keys,
        remove_keys,
        new_threshold,
    });

    let action_hex =
        action_codec::encode_hex(&action).map_err(|e| format!("failed to encode action: {e}"))?;
    Ok(BuildActionHexResponse { action_hex })
}
