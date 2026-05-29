use std::num::NonZeroU8;

use desktop_app::domain::action::{Action, CompressedPubKey, MultisigUpdate};
use desktop_app::domain::authority::Authority;
use desktop_app::infrastructure::action_codec;
use desktop_app::infrastructure::asm_status_rpc;
use desktop_app::infrastructure::broadcast_env;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum DecodedAction {
    #[serde(rename = "multisig_update", rename_all = "camelCase")]
    MultisigUpdate {
        role: String,
        add_keys: Vec<String>,
        remove_keys: Vec<String>,
        new_threshold: u8,
    },
    #[serde(rename = "unknown", rename_all = "camelCase")]
    Unknown { raw_hex: String },
}

#[tauri::command]
pub fn decode_action_hex(action_hex: String) -> DecodedAction {
    let hex = action_hex
        .strip_prefix("0x")
        .unwrap_or(&action_hex)
        .to_string();
    match action_codec::decode_hex(&hex) {
        Ok(Action::MultisigUpdate(update)) => DecodedAction::MultisigUpdate {
            role: update.role.as_str().to_string(),
            add_keys: update.add_keys.iter().map(|k| k.to_hex()).collect(),
            remove_keys: update.remove_keys.iter().map(|k| k.to_hex()).collect(),
            new_threshold: update.new_threshold.get(),
        },
        Err(_) => DecodedAction::Unknown { raw_hex: hex },
    }
}

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

#[tauri::command]
pub async fn build_cancel_action_hex(
    target_action_hex: String,
    wallet_session: tauri::State<'_, desktop_app::application::wallet_session::WalletSession>,
) -> Result<BuildActionHexResponse, String> {
    let env = broadcast_env::load_broadcast_env(&wallet_session).map_err(|e| e.to_string())?;
    let update_id = asm_status_rpc::find_update_id_in_queue(&env.asm_rpc_url, &target_action_hex)
        .await?
        .ok_or_else(|| {
            "The update has not been confirmed in the ASM queue yet. \
             Wait for the reveal transaction to confirm before canceling."
                .to_string()
        })?;
    let action_hex =
        action_codec::encode_cancel_hex_for_target(&target_action_hex, update_id as u64)
            .map_err(|e| e.to_string())?;
    Ok(BuildActionHexResponse { action_hex })
}
