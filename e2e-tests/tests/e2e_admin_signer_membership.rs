//! E2E membership checks for Strata ASM administration.
//!
//! This test validates role membership against the real canonical state produced
//! by the ASM worker + bitcoind harness, without relying on local state mocks.

use alpen_multisig_e2e_tests::test_harness::AsmTestHarnessBuilder;
use std::process::Command;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::{Role, SubprotocolInstance};
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_crypto::keys::compressed::CompressedPublicKey;

/// Extract administration subprotocol state from an anchor snapshot (STF output shape).
fn admin_subproto_state(anchor: &AnchorState) -> anyhow::Result<AdministrationSubprotoState> {
    let section = anchor
        .find_section(AdministrationSubprotocol::ID)
        .ok_or_else(|| anyhow::anyhow!("admin section missing"))?;
    let state = section.try_to_state::<AdministrationSubprotocol>()?;
    Ok(state)
}

/// Returns true if `signer_pk` is a member of `role`'s canonical multisig set in `admin`.
fn signer_is_in_role(
    admin: &AdministrationSubprotoState,
    role: Role,
    signer_pk: &CompressedPublicKey,
) -> bool {
    let Some(auth) = admin.authority(role) else {
        return false;
    };
    auth.config().keys().contains(signer_pk)
}

#[tokio::test(flavor = "multi_thread")]
async fn signer_membership_on_real_harness_state() {
    println!("[e2e-membership] start");

    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!(
            "Skipping signer_membership_on_real_harness_state: bitcoind is not available in PATH"
        );
        return;
    }

    let harness = AsmTestHarnessBuilder::default()
        .build()
        .await
        .expect("harness should build");

    harness
        .mine_block(None)
        .await
        .expect("should mine and process one block");

    let (commitment, asm_state) = harness
        .get_latest_asm_state()
        .expect("latest ASM state query should succeed")
        .expect("latest ASM state should exist");
    println!(
        "[e2e-membership] latest ASM state at height={}",
        commitment.height()
    );

    let admin_state =
        admin_subproto_state(asm_state.state()).expect("admin subprotocol state should exist");

    let admin_cfg = harness
        .asm_params
        .subprotocols
        .iter()
        .find_map(|instance| match instance {
            SubprotocolInstance::Admin(cfg) => Some(cfg),
            _ => None,
        })
        .expect("admin config should exist in ASM params");

    let admin_keys = admin_cfg.get_config(Role::StrataAdministrator).keys();
    let seq_keys = admin_cfg.get_config(Role::StrataSequencerManager).keys();

    let admin_only = admin_keys
        .iter()
        .find(|pk| !seq_keys.contains(pk))
        .copied()
        .unwrap_or_else(|| {
            *admin_keys
                .first()
                .expect("admin role should have at least one key")
        });
    let seq_only = seq_keys
        .iter()
        .find(|pk| !admin_keys.contains(pk))
        .copied()
        .unwrap_or_else(|| {
            *seq_keys
                .first()
                .expect("sequencer role should have at least one key")
        });

    let is_admin_admin = signer_is_in_role(&admin_state, Role::StrataAdministrator, &admin_only);
    let is_admin_seq = signer_is_in_role(&admin_state, Role::StrataAdministrator, &seq_only);
    let is_seq_seq = signer_is_in_role(&admin_state, Role::StrataSequencerManager, &seq_only);
    let is_seq_admin = signer_is_in_role(&admin_state, Role::StrataSequencerManager, &admin_only);

    assert!(
        is_admin_admin,
        "an admin signer from configured params should be present in canonical admin state"
    );
    assert!(
        !is_admin_seq,
        "a sequencer-only signer should not be a Strata admin signer"
    );
    assert!(
        is_seq_seq,
        "a sequencer signer from configured params should be present in canonical admin state"
    );
    assert!(
        !is_seq_admin,
        "an admin-only signer should not be a Strata sequencer-manager signer"
    );
    println!("[e2e-membership] assertions passed");
}
