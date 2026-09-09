//! SSZ hygiene for proposal `action_hex` (P-026). Protocol rules stay on-chain.

use ssz::Decode;
use strata_asm_txs_admin::actions::MultisigAction;

pub(crate) fn decode_multisig_action_hex(action_hex: &str) -> Result<MultisigAction, String> {
    let bytes = hex::decode(action_hex.trim()).map_err(|e| format!("invalid action hex: {e}"))?;
    MultisigAction::from_ssz_bytes(&bytes).map_err(|e| format!("invalid SSZ MultisigAction: {e:?}"))
}

/// Valid `action_hex` for handler/integration tests (minimal Strata admin update).
#[cfg(test)]
pub(crate) fn test_fixture_action_hex() -> String {
    use std::num::NonZeroU8;

    use ssz::Encode;
    use strata_asm_txs_admin::actions::updates::StrataAdminMultisigUpdate;
    use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
    use strata_crypto::threshold_signature::ThresholdConfigUpdate;

    let config_update =
        ThresholdConfigUpdate::new(vec![], vec![], NonZeroU8::new(2).expect("threshold"));
    let action = MultisigAction::Update(UpdateAction::StrataAdminMultisig(
        StrataAdminMultisigUpdate::new(config_update),
    ));
    hex::encode(action.as_ssz_bytes())
}

/// Valid tx type 15 fixture: a Security Council signer update authorized by Strata Admin.
#[cfg(test)]
pub(crate) fn test_fixture_council_rotation_action_hex() -> String {
    use std::num::NonZeroU8;

    use ssz::Encode;
    use strata_asm_txs_admin::actions::updates::StrataSecurityCouncilMultisigUpdate;
    use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
    use strata_crypto::threshold_signature::ThresholdConfigUpdate;

    let config_update =
        ThresholdConfigUpdate::new(vec![], vec![], NonZeroU8::new(2).expect("threshold"));
    let action = MultisigAction::Update(UpdateAction::StrataSecurityCouncilMultisig(
        StrataSecurityCouncilMultisigUpdate::new(config_update),
    ));
    hex::encode(action.as_ssz_bytes())
}

/// Valid tx type 14 fixture: a safe harbour address update, authorized by Strata Admin.
///
/// The destination is the taproot output for the secp256k1 generator point — the same one the
/// local stack ships with, and the one upstream pins in its own signing-message test.
#[cfg(test)]
pub(crate) fn test_fixture_safe_harbour_address_action_hex() -> String {
    use ssz::Encode;
    use strata_asm_txs_admin::actions::updates::SafeHarbourAddressUpdate;
    use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};

    let payload = [
        0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87, 0x0B,
        0x07, 0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B, 0x16, 0xF8,
        0x17, 0x98,
    ];
    let descriptor = bitcoin_bosd::Descriptor::new_p2tr(&payload).expect("valid x-only key");
    let address = strata_asm_proto_bridge_v1_types::SafeHarbourAddress::try_from(descriptor)
        .expect("p2tr descriptor accepted");
    let action = MultisigAction::Update(UpdateAction::SafeHarbourAddress(
        SafeHarbourAddressUpdate::new(address),
    ));
    hex::encode(action.as_ssz_bytes())
}

/// Valid `action_hex` for a Defcon 1 update — the action upstream hardcodes to depth `0`.
///
/// Test-only: the desktop cannot build this action until Phase 3. See
/// docs/specs/security-council-defcon-phase-2.md §7.
#[cfg(test)]
pub(crate) fn test_fixture_defcon_1_action_hex() -> String {
    use ssz::Encode;
    use strata_asm_txs_admin::actions::updates::Defcon1Update;
    use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};

    let action = MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update));
    hex::encode(action.as_ssz_bytes())
}

/// Test-only fixture for Defcon 3 enactment and cancel paths in later phases.
#[cfg(test)]
pub(crate) fn test_fixture_defcon_3_action_hex() -> String {
    use ssz::Encode;
    use strata_asm_txs_admin::actions::updates::Defcon3Update;
    use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};

    let action = MultisigAction::Update(UpdateAction::Defcon3(Defcon3Update));
    hex::encode(action.as_ssz_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_ssz_bytes() {
        assert!(decode_multisig_action_hex("deadbeef").is_err());
    }

    #[test]
    fn accepts_fixture_action() {
        decode_multisig_action_hex(&test_fixture_action_hex()).unwrap();
    }

    #[test]
    fn accepts_defcon_3_fixture_action() {
        decode_multisig_action_hex(&test_fixture_defcon_3_action_hex()).unwrap();
    }
}
