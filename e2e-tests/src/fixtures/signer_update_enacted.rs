//! Fixtures for the enacted signer-update flow (desktop signing + regtest + ASM).
//!
//! ## Invariant: derivation path
//!
//! [`SignerUpdateEnactedFixture::derivation_path_prefix`] must stay aligned with
//! [`desktop_app::infrastructure::signing::list_mnemonic_addresses`] and
//! [`desktop_app::infrastructure::signing::sign_with_mnemonic_path`], which derive
//! keys under `m/84'/0'/73'/0/{n}`.

use strata_asm_common::Subprotocol;
use strata_asm_params::AdministrationInitConfig;
use strata_asm_proto_admin::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_asm_proto_bridge_v1::{BridgeV1State, BridgeV1Subproto};
use strata_asm_proto_checkpoint::CheckpointState;
use strata_asm_proto_checkpoint::CheckpointSubprotocol;
use strata_asm_worker::AsmState;
use strata_predicate::PredicateTypeId;

/// Named configuration for [`crate::test_harness::AsmTestHarnessBuilder::with_admin_config`]
/// plus mnemonic metadata used by enacted signer-update tests.
#[derive(Debug, Clone, Copy)]
pub struct SignerUpdateEnactedFixture {
    pub name: &'static str,
    pub mnemonic: &'static str,
    pub passphrase: &'static str,
    /// Prefix without trailing `/index` (e.g. `m/84'/0'/73'/0`).
    pub derivation_path_prefix: &'static str,
    /// JSON object matching [`AdministrationInitConfig`] serde shape (admin section only).
    pub admin_section_json: &'static str,
    pub seq_no: u64,
}

/// Admin JSON aligned with `repo/asm/asm-params.json` intent: production-like
/// confirmation depth (144) for Strata admin updates.
/// Second Strata Administrator seed at the same canonical path as [`DEFAULT_REPO_ASM::mnemonic`].
pub const DEMO_COSIGN_MNEMONIC: &str =
    "multiply toss magic exclude crawl obey garden black apart room village absent";

pub const DEFAULT_REPO_ASM: SignerUpdateEnactedFixture = SignerUpdateEnactedFixture {
    name: "default_repo_asm",
    mnemonic: "multiply toss magic exclude crawl obey garden black apart room village neglect",
    passphrase: "",
    derivation_path_prefix: "m/84'/0'/73'/0",
    admin_section_json: DEFAULT_ADMIN_SECTION_JSON,
    seq_no: 1,
};

/// Same signer set and mnemonic as [`DEFAULT_REPO_ASM`], but low confirmation depth for
/// faster local/CI runs while still exercising the full commit → reveal → enact path.
pub const FAST_ENACTMENT: SignerUpdateEnactedFixture = SignerUpdateEnactedFixture {
    name: "fast_enactment",
    mnemonic: DEFAULT_REPO_ASM.mnemonic,
    passphrase: DEFAULT_REPO_ASM.passphrase,
    derivation_path_prefix: DEFAULT_REPO_ASM.derivation_path_prefix,
    admin_section_json: FAST_ADMIN_SECTION_JSON,
    seq_no: DEFAULT_REPO_ASM.seq_no,
};

const DEFAULT_ADMIN_SECTION_JSON: &str = r#"{
  "strata_administrator": {
    "keys": [
      "02300dc42e67165c78256d5ef816bad845428841f54f1ecb6da8a3eb1d066f4df7",
      "029b8c2ba19ce259e500c1b57c664b60bbb81820938af58a5fab3158fc1386119e"
    ],
    "threshold": 2
  },
  "strata_sequencer_manager": {
    "keys": [
      "03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c"
    ],
    "threshold": 1
  },
  "alpen_administrator": {
    "keys": [
      "03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c"
    ],
    "threshold": 1
  },
  "strata_security_council": {
    "keys": [
      "02300dc42e67165c78256d5ef816bad845428841f54f1ecb6da8a3eb1d066f4df7",
      "029b8c2ba19ce259e500c1b57c664b60bbb81820938af58a5fab3158fc1386119e"
    ],
    "threshold": 2
  },
  "confirmation_depths": {
    "strata_admin_multisig_update": 144,
    "strata_seq_manager_multisig_update": 144,
    "alpen_admin_multisig_update": 144,
    "operator_update": 144,
    "sequencer_update": 144,
    "ol_stf_vk_update": 144,
    "asm_stf_vk_update": 144,
    "ee_stf_vk_update": 144,
    "strata_security_council_multisig_update": 144,
    "defcon3": 144,
    "safe_harbour_address_update": 144
  },
  "max_seqno_gap": 10
}"#;

const FAST_ADMIN_SECTION_JSON: &str = r#"{
  "strata_administrator": {
    "keys": [
      "02300dc42e67165c78256d5ef816bad845428841f54f1ecb6da8a3eb1d066f4df7",
      "029b8c2ba19ce259e500c1b57c664b60bbb81820938af58a5fab3158fc1386119e"
    ],
    "threshold": 2
  },
  "strata_sequencer_manager": {
    "keys": [
      "03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c"
    ],
    "threshold": 1
  },
  "alpen_administrator": {
    "keys": [
      "03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c"
    ],
    "threshold": 1
  },
  "strata_security_council": {
    "keys": [
      "02300dc42e67165c78256d5ef816bad845428841f54f1ecb6da8a3eb1d066f4df7",
      "029b8c2ba19ce259e500c1b57c664b60bbb81820938af58a5fab3158fc1386119e"
    ],
    "threshold": 2
  },
  "confirmation_depths": {
    "strata_admin_multisig_update": 5,
    "strata_seq_manager_multisig_update": 5,
    "alpen_admin_multisig_update": 5,
    "operator_update": 5,
    "sequencer_update": 5,
    "ol_stf_vk_update": 5,
    "asm_stf_vk_update": 5,
    "ee_stf_vk_update": 5,
    "strata_security_council_multisig_update": 5,
    "defcon3": 5,
    "safe_harbour_address_update": 5
  },
  "max_seqno_gap": 10
}"#;

/// Parse the admin subsection JSON into a [`serde_json::Value`].
pub fn parse_admin_section(admin_section_json: &str) -> serde_json::Value {
    serde_json::from_str(admin_section_json).expect("parse admin section JSON")
}

/// Ordered hex pubkeys for `strata_administrator` (canonical multisig order).
pub fn strata_admin_keys_hex(admin: &serde_json::Value) -> Vec<String> {
    admin["strata_administrator"]["keys"]
        .as_array()
        .expect("strata_administrator.keys is an array")
        .iter()
        .map(|v| v.as_str().expect("each key is a string").to_string())
        .collect()
}

/// Deserialize [`AdministrationInitConfig`] from the admin JSON object.
pub fn administration_init_config(admin: &serde_json::Value) -> AdministrationInitConfig {
    serde_json::from_value(admin.clone())
        .expect("admin section deserializes into AdministrationInitConfig")
}

/// Confirmation depth (in blocks) for Strata administrator multisig updates.
pub fn strata_admin_confirmation_depth(admin: &serde_json::Value) -> u16 {
    admin["confirmation_depths"]["strata_admin_multisig_update"]
        .as_u64()
        .expect("confirmation_depths.strata_admin_multisig_update is u64") as u16
}

/// Ordered hex pubkeys for `strata_security_council` (canonical multisig order).
pub fn strata_security_council_keys_hex(admin: &serde_json::Value) -> Vec<String> {
    admin["strata_security_council"]["keys"]
        .as_array()
        .expect("strata_security_council.keys is an array")
        .iter()
        .map(|v| v.as_str().expect("each key is a string").to_string())
        .collect()
}

/// Confirmation depth (in blocks) for Defcon 3 updates.
///
/// Defcon 1 has no counterpart here on purpose: upstream hard-codes its depth to zero
/// (`ConfirmationDepths::get` returns `None`), so it can never be delayed by config.
pub fn defcon3_confirmation_depth(admin: &serde_json::Value) -> u16 {
    admin["confirmation_depths"]["defcon3"]
        .as_u64()
        .expect("confirmation_depths.defcon3 is u64") as u16
}

/// Confirmation depth (in blocks) for OL STF verifying-key updates.
pub fn ol_stf_vk_confirmation_depth(admin: &serde_json::Value) -> u16 {
    admin["confirmation_depths"]["ol_stf_vk_update"]
        .as_u64()
        .expect("confirmation_depths.ol_stf_vk_update is u64") as u16
}

/// Assert demo mnemonic (canonical index 0) and cosign mnemonic (same path) match fixture keys.
pub fn assert_mnemonic_matches_strata_admin_keys(
    a_hex: &str,
    b_hex: &str,
    admin: &serde_json::Value,
    derivation_path_prefix: &str,
) {
    let json_keys = strata_admin_keys_hex(admin);
    assert_eq!(
        a_hex, json_keys[0],
        "demo mnemonic {derivation_path_prefix}/0 must match asm-params strata_administrator.keys[0]"
    );
    assert_eq!(
        b_hex, json_keys[1],
        "cosign mnemonic {derivation_path_prefix}/0 must match asm-params strata_administrator.keys[1]"
    );
}

/// Decode the administration subprotocol state from the latest [`AsmState`].
pub fn decode_administration_subproto(asm_state: &AsmState) -> Option<AdministrationSubprotoState> {
    asm_state
        .state()
        .find_section(AdministrationSubprotocol::ID)
        .and_then(|section| section.try_to_state::<AdministrationSubprotocol>().ok())
}

/// Decode the bridge subprotocol state from an [`AsmState`].
pub fn decode_bridge_subproto(asm_state: &AsmState) -> Option<BridgeV1State> {
    asm_state
        .state()
        .find_section(BridgeV1Subproto::ID)
        .and_then(|section| section.try_to_state::<BridgeV1Subproto>().ok())
}

/// Decode the checkpoint subprotocol state from an [`AsmState`].
pub fn decode_checkpoint_subproto(asm_state: &AsmState) -> Option<CheckpointState> {
    asm_state
        .state()
        .find_section(CheckpointSubprotocol::ID)
        .and_then(|section| section.try_to_state::<CheckpointSubprotocol>().ok())
}

/// Extract the `PredicateTypeId` from the checkpoint predicate stored in [`AsmState`].
pub fn checkpoint_ol_stf_vk_type(asm_state: &AsmState) -> Option<PredicateTypeId> {
    let state = decode_checkpoint_subproto(asm_state)?;
    PredicateTypeId::try_from(state.checkpoint_predicate().id()).ok()
}

#[cfg(test)]
mod tests {
    use strata_asm_params::AsmParams;

    /// The repo example must deserialize against the pinned `AsmParams` shape. Substituting the
    /// blkid placeholder is what an operator does before bootstrapping; the test does the same so
    /// a future pin bump that adds required fields fails here instead of in someone's first
    /// `asm-params.json` edit.
    #[test]
    fn asm_params_example_deserializes_after_blkid_placeholder_substituted() {
        const PLACEHOLDER: &str = "REPLACE_WITH_64_HEX_CHARS_FROM_getblockhash_101";
        const DUMMY_BLKID: &str =
            "0000000000000000000000000000000000000000000000000000000000000000";

        let raw = include_str!("../../../scripts/asm-params.example.json");
        assert!(
            raw.contains(PLACEHOLDER),
            "example must keep the operator-facing blkid placeholder"
        );
        let filled = raw.replace(PLACEHOLDER, DUMMY_BLKID);
        let params: AsmParams = serde_json::from_str(&filled)
            .expect("asm-params.example.json deserializes as AsmParams");
        assert!(
            params.admin_config().is_some(),
            "example must carry an Admin subprotocol"
        );
    }
}
