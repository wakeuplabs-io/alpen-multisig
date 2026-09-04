use std::num::NonZeroU8;

use desktop_app::domain::action::{
    Action, CompressedPubKey, EvenPubKey, MultisigUpdate, OperatorSetUpdate, SequencerKeyUpdate,
    VkUpdate,
};
use desktop_app::domain::authority::Authority;
use desktop_app::infrastructure::action_codec;
use desktop_app::infrastructure::asm_status_rpc;
use desktop_app::infrastructure::broadcast_env;
use desktop_app::infrastructure::node_config_store::NodeConfigState;
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
    #[serde(rename = "vk_update", rename_all = "camelCase")]
    VkUpdate {
        authority: String,
        type_id: u8,
        condition_hex: String,
    },
    #[serde(rename = "defcon_1")]
    Defcon1,
    #[serde(rename = "defcon_3")]
    Defcon3,
    #[serde(rename = "cancel", rename_all = "camelCase")]
    Cancel {
        target_update_id: u32,
        target_action_hex: String,
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
    // Tried first: a cancel hex fails `decode_hex` below (the domain `Action` has no `Cancel`
    // variant) and would otherwise land in the `Err(_) => Unknown` arm.
    if let Ok(Some((target_update_id, target_action_hex))) =
        action_codec::decode_cancel_target_hex(&hex)
    {
        return DecodedAction::Cancel {
            target_update_id,
            target_action_hex,
        };
    }
    match action_codec::decode_hex(&hex) {
        Ok(Action::MultisigUpdate(update)) => DecodedAction::MultisigUpdate {
            role: update.role.as_str().to_string(),
            add_keys: update.add_keys.iter().map(|k| k.to_hex()).collect(),
            remove_keys: update.remove_keys.iter().map(|k| k.to_hex()).collect(),
            new_threshold: update.new_threshold.get(),
        },
        Ok(Action::VkUpdate(update)) => DecodedAction::VkUpdate {
            authority: update.authority.as_str().to_string(),
            type_id: update.type_id,
            condition_hex: hex::encode(&update.condition),
        },
        Ok(Action::Defcon1) => DecodedAction::Defcon1,
        Ok(Action::Defcon3) => DecodedAction::Defcon3,
        // Still unregistered at this boundary, and unrelated to the council: both predate this
        // slice and both render through the raw-hex fallback today.
        Ok(Action::OperatorSetUpdate(_)) | Ok(Action::SequencerKeyUpdate(_)) | Err(_) => {
            DecodedAction::Unknown { raw_hex: hex }
        }
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildVkUpdateHexInput {
    pub authority: String,
    pub type_id: u8,
    pub condition_hex: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOperatorSetUpdateHexInput {
    pub add_operator_keys: Vec<String>,
    pub remove_operator_indices: Vec<u32>,
}

#[tauri::command]
pub fn build_operator_set_update_hex(
    input: BuildOperatorSetUpdateHexInput,
) -> Result<BuildActionHexResponse, String> {
    let add_members = input
        .add_operator_keys
        .iter()
        .map(|k| EvenPubKey::from_hex(k.trim()).map_err(|e| format!("invalid operator key: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
    let action = Action::OperatorSetUpdate(OperatorSetUpdate {
        add_members,
        remove_members: input.remove_operator_indices,
    });
    let action_hex =
        action_codec::encode_hex(&action).map_err(|e| format!("failed to encode action: {e}"))?;
    Ok(BuildActionHexResponse { action_hex })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildSequencerKeyUpdateHexInput {
    pub new_pub_key: String,
}

#[tauri::command]
pub fn build_sequencer_key_update_hex(
    input: BuildSequencerKeyUpdateHexInput,
) -> Result<BuildActionHexResponse, String> {
    let new_pub_key = EvenPubKey::from_hex(input.new_pub_key.trim())
        .map_err(|e| format!("invalid sequencer key: {e}"))?;
    let action = Action::SequencerKeyUpdate(SequencerKeyUpdate { new_pub_key });
    let action_hex =
        action_codec::encode_hex(&action).map_err(|e| format!("failed to encode action: {e}"))?;
    Ok(BuildActionHexResponse { action_hex })
}

/// Build the payload-less Defcon 1 action.
///
/// No input: the action carries nothing, and the sequence number is a field of the proposal
/// creation request, as it is for every other action type.
#[tauri::command]
pub fn build_defcon_1_action_hex() -> Result<BuildActionHexResponse, String> {
    let action_hex = action_codec::encode_hex(&Action::Defcon1)
        .map_err(|e| format!("failed to encode action: {e}"))?;
    Ok(BuildActionHexResponse { action_hex })
}

/// Build the payload-less Defcon 3 action.
///
/// Shaped exactly like Defcon 1's: same authority, same empty payload, same sequence number on the
/// creation request. The delay is not encoded here — it is `confirmation_depths.defcon3`, resolved
/// live from the ASM, and this hex would be wrong the moment it carried a copy of it.
#[tauri::command]
pub fn build_defcon_3_action_hex() -> Result<BuildActionHexResponse, String> {
    let action_hex = action_codec::encode_hex(&Action::Defcon3)
        .map_err(|e| format!("failed to encode action: {e}"))?;
    Ok(BuildActionHexResponse { action_hex })
}

#[tauri::command]
pub fn build_vk_update_hex(input: BuildVkUpdateHexInput) -> Result<BuildActionHexResponse, String> {
    let authority = Authority::from_wire(input.authority.trim())
        .map_err(|e| format!("invalid authority `{}`: {e}", input.authority))?;
    let condition = if input.condition_hex.trim().is_empty() {
        vec![]
    } else {
        hex::decode(input.condition_hex.trim())
            .map_err(|e| format!("invalid condition hex: {e}"))?
    };
    let action = Action::VkUpdate(VkUpdate {
        authority,
        type_id: input.type_id,
        condition,
    });
    let action_hex =
        action_codec::encode_hex(&action).map_err(|e| format!("failed to encode action: {e}"))?;
    Ok(BuildActionHexResponse { action_hex })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_vk_update_roundtrip() {
        let input = BuildVkUpdateHexInput {
            authority: "alpen_admin".to_string(),
            type_id: 1,
            condition_hex: String::new(),
        };
        let hex = build_vk_update_hex(input)
            .expect("build should succeed")
            .action_hex;
        match decode_action_hex(hex) {
            DecodedAction::VkUpdate {
                authority,
                type_id,
                condition_hex,
            } => {
                assert_eq!(authority, "alpen_admin");
                assert_eq!(type_id, 1);
                assert_eq!(condition_hex, "");
            }
            other => panic!("expected VkUpdate, got {other:?}"),
        }
    }

    /// The proposal DTO's `actionType` and this command are the two IPC boundaries Phase 3
    /// parked on `Unknown`; both are closed schemas on the TypeScript side, so this asserts the
    /// Rust half emits the value `decodedActionSchema` was taught to accept.
    #[test]
    fn decode_defcon_1_names_the_action() {
        let hex = build_defcon_1_action_hex()
            .expect("build should succeed")
            .action_hex;
        assert!(matches!(decode_action_hex(hex), DecodedAction::Defcon1));
    }

    /// The same round trip for the timelocked lever. Phase 1 could only encode it from the codec
    /// because no builder existed; going through the command is what proves the flow a council
    /// signer actually takes ends up at `Defcon3` and not at its neighbour.
    #[test]
    fn decode_defcon_3_names_the_action() {
        let hex = build_defcon_3_action_hex()
            .expect("build should succeed")
            .action_hex;
        assert!(matches!(decode_action_hex(hex), DecodedAction::Defcon3));
    }

    #[test]
    fn decode_vk_update_with_condition_hex() {
        let condition = "ab".repeat(32);
        let input = BuildVkUpdateHexInput {
            authority: "strata_admin".to_string(),
            type_id: 10,
            condition_hex: condition.clone(),
        };
        let hex = build_vk_update_hex(input)
            .expect("build should succeed")
            .action_hex;
        match decode_action_hex(hex) {
            DecodedAction::VkUpdate {
                authority,
                type_id,
                condition_hex,
            } => {
                assert_eq!(authority, "strata_admin");
                assert_eq!(type_id, 10);
                assert_eq!(condition_hex, condition);
            }
            other => panic!("expected VkUpdate, got {other:?}"),
        }
    }

    /// The exact gate `/manual` fails on today: a cancel hex must decode to `Cancel`, not fall
    /// through to `Unknown` because the domain `Action` has no `Cancel` variant.
    #[test]
    fn decode_cancel_names_the_action() {
        let target_hex = build_defcon_3_action_hex()
            .expect("build should succeed")
            .action_hex;
        let cancel_hex =
            action_codec::encode_cancel_hex_for_target(&target_hex, 7).expect("cancel encodes ok");
        match decode_action_hex(cancel_hex) {
            DecodedAction::Cancel {
                target_update_id,
                target_action_hex,
            } => {
                assert_eq!(target_update_id, 7);
                assert_eq!(target_action_hex, target_hex);
            }
            other => panic!("expected Cancel, got {other:?}"),
        }
    }
}

#[tauri::command]
pub async fn build_cancel_action_hex(
    target_action_hex: String,
    wallet_session: tauri::State<'_, desktop_app::application::wallet_session::WalletSession>,
    node_config: tauri::State<'_, NodeConfigState>,
) -> Result<BuildActionHexResponse, String> {
    let cfg = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();
    let env =
        broadcast_env::load_broadcast_env(&wallet_session, &cfg).map_err(|e| e.to_string())?;
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
