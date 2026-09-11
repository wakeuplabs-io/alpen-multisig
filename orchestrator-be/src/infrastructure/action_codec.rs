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
}
