use std::num::NonZeroU8;

use desktop_app::domain::action::{
    Action, CompressedPubKey, EvenPubKey, MultisigUpdate, OperatorSetUpdate, SafeHarbourDescriptor,
    SequencerKeyUpdate, VkUpdate,
};
use desktop_app::domain::authority::Authority;
use desktop_app::infrastructure::action_codec;
use desktop_app::infrastructure::asm_status_rpc;
use desktop_app::infrastructure::broadcast_env;
use desktop_app::infrastructure::network_env;
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
    /// Both forms of one destination: `addressHex` is the BOSD descriptor, which is what the
    /// device displays and therefore what a signer compares; `address` is the same value rendered
    /// for the process's active network, which is what an operator recognises.
    #[serde(rename = "safe_harbour_address_update", rename_all = "camelCase")]
    SafeHarbourAddressUpdate {
        address_hex: String,
        address: String,
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
        Ok(Action::SafeHarbourAddressUpdate(destination)) => {
            // A network that cannot be resolved must not blank the destination: the hex is the
            // value the device shows, so it is rendered either way and only the address degrades.
            let address = network_env::network_from_env()
                .map(|network| destination.to_address(network))
                .unwrap_or_default();
            DecodedAction::SafeHarbourAddressUpdate {
                address_hex: destination.to_bosd_hex(),
                address,
            }
        }
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

/// Input for [`build_safe_harbour_address_update_hex`].
///
/// The address, not the descriptor hex: an operator holds an address, and nobody distributes a
/// safe harbour as a BOSD string. The conversion is the application's, and the rendered signing
/// message is what exposes its result for the signer to check.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildSafeHarbourAddressUpdateHexInput {
    pub address: String,
}

/// Build a safe harbour address update from a bech32m P2TR address.
///
/// The network is the process's, resolved once through `network_env` — the canonical resolution in
/// this repository, which deliberately does not live in `NodeConfig`.
#[tauri::command]
pub fn build_safe_harbour_address_update_hex(
    input: BuildSafeHarbourAddressUpdateHexInput,
) -> Result<BuildActionHexResponse, String> {
    let network = network_env::network_from_env().map_err(|e| e.to_string())?;
    let destination =
        SafeHarbourDescriptor::from_address(&input.address, network).map_err(|e| e.to_string())?;
    let action = Action::SafeHarbourAddressUpdate(destination);
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

    /// The wire-level expression of the segregation invariant (AC 2): a council rotation names
    /// itself in `Action:` but names the administrator in `Authorized By:` — two distinct lines,
    /// never merged. Runs the path the device actually signs over, out of the builder rather than
    /// a hand-built `Action`, because the claim this side can make is that *our* mapping
    /// (`Authority::SecurityCouncil` -> `UpdateAction::StrataSecurityCouncilMultisig`, wired in
    /// `action_codec.rs`) lands on the variant upstream renders as tx 15. Upstream's own nine
    /// lines are already pinned byte-for-byte in `strata_security_council_multisig.rs`, and
    /// restating them here would only test upstream's test.
    ///
    /// Asserts on `message.lines()`, not `contains()` over the whole string: a renderer that
    /// joined the two lines with a space would still pass a `contains` check and still put the
    /// wrong words in front of a signer. Literals are pinned here — unlike the neighbouring
    /// Defcon tripwire (`signing.rs:453-458`, which explicitly declines to pin upstream's)
    /// — because the new coverage *is* the pair of lines naming two different roles, the
    /// wire-level shape of the segregation invariant. This test replaces
    /// `decode_council_signer_update_names_the_target_role`, which built the `Action` by hand and
    /// asserted a subset of what this asserts.
    #[test]
    fn council_signer_update_signing_message_names_both_roles_on_separate_lines() {
        use desktop_app::infrastructure::signing::render_signing_message;

        let pk = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5".to_string();
        let seqno = 7;

        let council_hex = build_admin_multisig_update_hex(BuildAdminMultisigUpdateHexInput {
            role: "security_council".to_string(),
            add_keys: vec![pk.clone()],
            remove_keys: vec![],
            new_threshold: 2,
        })
        .expect("build should succeed")
        .action_hex;

        let admin_hex = build_admin_multisig_update_hex(BuildAdminMultisigUpdateHexInput {
            role: "strata_admin".to_string(),
            add_keys: vec![pk.clone()],
            remove_keys: vec![],
            new_threshold: 2,
        })
        .expect("build should succeed")
        .action_hex;

        let council_message =
            render_signing_message(seqno, &council_hex).expect("council message renders");
        let admin_message =
            render_signing_message(seqno, &admin_hex).expect("administrator message renders");

        assert_eq!(
            council_message,
            format!(
                concat!(
                    "Strata ASM Administration v1\n",
                    "Action: Strata Security Council Multisig Update\n",
                    "Authorized By: Strata Administrator\n",
                    "Sequence: {seqno}\n",
                    "Action Details:\n",
                    "  New Threshold: 2\n",
                    "  Members to Add: 1\n",
                    "  1. Add Member: {pk}\n",
                    "  Members to Remove: 0"
                ),
                seqno = seqno,
                pk = pk
            ),
            "the signer must see the exact canonical nine-line message"
        );

        assert_ne!(
            council_message, admin_message,
            "same seqno, same keys, same threshold — only the action differs, and the signer must see that"
        );
    }

    /// The claim upstream cannot make for us: that the address a signer typed reaches the variant
    /// that renders tx type 14, and that what the device shows is the **descriptor hex** rather
    /// than the address the signer entered. Run out of the builder rather than a hand-built
    /// `Action`, so it covers the mapping the device actually signs over.
    ///
    /// Asserts on `lines()`, not `contains()`: a renderer that joined two lines would still pass a
    /// `contains` check and still put the wrong thing in front of a signer.
    #[test]
    fn safe_harbour_signing_message_shows_the_descriptor_hex_not_the_address() {
        use desktop_app::infrastructure::signing::render_signing_message;

        // x-only key of the generator point G, the destination the local stack ships with.
        let descriptor_hex = "0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let network = network_env::network_from_env().expect("a valid network");
        let address = SafeHarbourDescriptor::from_hex(descriptor_hex)
            .expect("valid descriptor")
            .to_address(network);
        let seqno = 17;

        let action_hex =
            build_safe_harbour_address_update_hex(BuildSafeHarbourAddressUpdateHexInput {
                address: address.clone(),
            })
            .expect("build should succeed")
            .action_hex;

        let message = render_signing_message(seqno, &action_hex).expect("message renders");
        let lines: Vec<&str> = message.lines().collect();

        assert_eq!(
            lines,
            vec![
                "Strata ASM Administration v1",
                "Action: Safe Harbour Address Update",
                "Authorized By: Strata Administrator",
                "Sequence: 17",
                "Action Details:",
                &format!("  New Safe Harbour Address: {descriptor_hex}"),
            ],
            "the signer must see the exact canonical six-line message"
        );

        // The address is what the signer typed; it is deliberately *not* what they will be asked
        // to confirm. A surface that showed only the address would leave nothing to compare
        // against the device screen.
        assert!(
            !message.contains(&address),
            "the device shows the descriptor, never the address"
        );
    }

    /// `Authorized By` names the Strata Administrator even though the action reaches into the
    /// bridge. That line is the wire-level shape of the segregation invariant — the council
    /// triggers the sweep, the administrator picks the destination — and a change that moved it
    /// would be an upstream break worth catching here rather than on a signer's screen.
    #[test]
    fn safe_harbour_update_is_authorized_by_the_administrator_not_the_council() {
        use desktop_app::infrastructure::signing::render_signing_message;

        let network = network_env::network_from_env().expect("a valid network");
        let address = SafeHarbourDescriptor::from_hex(
            "0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        )
        .expect("valid descriptor")
        .to_address(network);

        let action_hex =
            build_safe_harbour_address_update_hex(BuildSafeHarbourAddressUpdateHexInput {
                address,
            })
            .expect("build should succeed")
            .action_hex;

        let message = render_signing_message(3, &action_hex).expect("message renders");
        assert!(
            message
                .lines()
                .any(|line| line == "Authorized By: Strata Administrator"),
            "expected the administrator to authorize tx type 14, got:\n{message}"
        );
    }

    /// The decode side of the same action, which is what every read surface renders from.
    #[test]
    fn decode_safe_harbour_update_carries_both_forms_of_the_destination() {
        let descriptor_hex = "0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let network = network_env::network_from_env().expect("a valid network");
        let address = SafeHarbourDescriptor::from_hex(descriptor_hex)
            .expect("valid descriptor")
            .to_address(network);

        let action_hex =
            build_safe_harbour_address_update_hex(BuildSafeHarbourAddressUpdateHexInput {
                address: address.clone(),
            })
            .expect("build should succeed")
            .action_hex;

        match decode_action_hex(action_hex) {
            DecodedAction::SafeHarbourAddressUpdate {
                address_hex,
                address: rendered,
            } => {
                assert_eq!(address_hex, descriptor_hex);
                assert_eq!(rendered, address);
            }
            other => panic!("expected SafeHarbourAddressUpdate, got {other:?}"),
        }
    }

    /// A destination that is not taproot never becomes an action: the form explains, the domain
    /// decides, and upstream refuses last. This pins the first of the three.
    #[test]
    fn build_safe_harbour_update_refuses_a_non_taproot_address() {
        let network = network_env::network_from_env().expect("a valid network");
        let mut raw = vec![0x00, 0x14];
        raw.extend_from_slice(&[0xAA; 20]);
        let p2wpkh =
            bitcoin::Address::from_script(bitcoin::ScriptBuf::from_bytes(raw).as_script(), network)
                .expect("a valid P2WPKH address");

        let err = build_safe_harbour_address_update_hex(BuildSafeHarbourAddressUpdateHexInput {
            address: p2wpkh.to_string(),
        })
        .expect_err("a P2WPKH destination must be refused");
        assert!(
            err.contains("taproot"),
            "the error must name what was wrong, got: {err}"
        );
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
