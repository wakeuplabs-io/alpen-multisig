//! Membership checks for Strata ASM administration: a signer is in a role iff their
//! [`CompressedPublicKey`](strata_crypto::keys::compressed::CompressedPublicKey) appears in that
//! role's [`ThresholdConfig::keys`](strata_crypto::threshold_signature::ThresholdConfig::keys) on
//! the canonical [`AdministrationSubprotoState`], including when that state is embedded in
//! [`AnchorState`](strata_asm_common::AnchorState) (same pattern as upstream `tests/harness/admin.rs`).

use std::num::NonZero;

use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
use rand::rngs::OsRng;
use serde_json::json;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::{AdministrationInitConfig, AsmParams, Role};
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_asm_spec::construct_genesis_state;
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::ThresholdConfig;
use strata_l1_txfmt::MagicBytes;

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

/// Genesis [`AsmParams`] with a replaceable admin section; checkpoint/bridge copied from upstream
/// `strata_asm_params` JSON fixture (valid predicates / layout).
fn asm_params_with_admin(admin: AdministrationInitConfig) -> anyhow::Result<AsmParams> {
    let root = json!({
      "magic": "ALPN",
      "anchor": {
        "block": {
          "height": 50462976,
          "blkid": "0405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223"
        },
        "next_target": 656811300,
        "epoch_start_timestamp": 724183336,
        "network": "regtest"
      },
      "subprotocols": [
        { "Admin": admin },
        {
          "Checkpoint": {
            "sequencer_predicate": "Sp1Groth16",
            "checkpoint_predicate": "AlwaysAccept",
            "genesis_l1_height": 100,
            "genesis_ol_blkid": "c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6"
          }
        },
        {
          "Bridge": {
            "operators": [
              "02becdf7aab195ab0a42ba2f2eca5b7fa5a246267d802c627010e1672f08657f70"
            ],
            "denomination": 0,
            "assignment_duration": 0,
            "operator_fee": 0,
            "recovery_delay": 0
          }
        }
      ]
    });

    let params: AsmParams = serde_json::from_value(root)?;
    Ok(params)
}

fn compressed_pk(sk: &SecretKey) -> CompressedPublicKey {
    CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, sk))
}

#[test]
fn signer_membership_on_admin_subproto_state() -> anyhow::Result<()> {
    let admin_sk = SecretKey::new(&mut OsRng);
    let seq_sk = SecretKey::new(&mut OsRng);
    let outsider_sk = SecretKey::new(&mut OsRng);

    let admin_pk = compressed_pk(&admin_sk);
    let seq_pk = compressed_pk(&seq_sk);
    let outsider_pk = compressed_pk(&outsider_sk);

    let admin_threshold = NonZero::new(1).unwrap();
    let strata_administrator = ThresholdConfig::try_new(vec![admin_pk], admin_threshold)?;
    let strata_sequencer_manager = ThresholdConfig::try_new(vec![seq_pk], admin_threshold)?;

    let init = AdministrationInitConfig::new(
        strata_administrator.clone(),
        strata_sequencer_manager,
        2016,
        NonZero::new(10).unwrap(),
    );

    let state = AdministrationSubprotoState::new(&init);

    assert!(
        signer_is_in_role(&state, Role::StrataAdministrator, &admin_pk),
        "Strata admin signer should match keys in StrataAdministrator config"
    );
    assert!(
        !signer_is_in_role(&state, Role::StrataAdministrator, &seq_pk),
        "sequencer-manager-only key must not qualify as Strata admin (role isolation)"
    );
    assert!(
        !signer_is_in_role(&state, Role::StrataAdministrator, &outsider_pk),
        "unrelated pubkey must not be a Strata admin signer"
    );

    assert!(signer_is_in_role(
        &state,
        Role::StrataSequencerManager,
        &seq_pk
    ));
    assert!(!signer_is_in_role(
        &state,
        Role::StrataSequencerManager,
        &admin_pk
    ));

    // Same check via init config only (genesis / params path).
    assert!(init
        .get_config(Role::StrataAdministrator)
        .keys()
        .contains(&admin_pk));
    assert!(!init
        .get_config(Role::StrataAdministrator)
        .keys()
        .contains(&seq_pk));

    Ok(())
}

#[test]
fn signer_membership_via_anchor_state_genesis() -> anyhow::Result<()> {
    let admin_sk = SecretKey::new(&mut OsRng);
    let seq_sk = SecretKey::new(&mut OsRng);
    let admin_pk = compressed_pk(&admin_sk);
    let seq_pk = compressed_pk(&seq_sk);

    let t = NonZero::new(1).unwrap();
    let admin_init = AdministrationInitConfig::new(
        ThresholdConfig::try_new(vec![admin_pk], t)?,
        ThresholdConfig::try_new(vec![seq_pk], t)?,
        2016,
        NonZero::new(10).unwrap(),
    );

    let params = asm_params_with_admin(admin_init)?;
    assert_eq!(params.magic, MagicBytes::new(*b"ALPN"));

    let anchor = construct_genesis_state(&params);
    let admin_state = admin_subproto_state(&anchor)?;

    assert!(signer_is_in_role(
        &admin_state,
        Role::StrataAdministrator,
        &admin_pk
    ));
    assert!(!signer_is_in_role(
        &admin_state,
        Role::StrataAdministrator,
        &seq_pk
    ));

    Ok(())
}

#[test]
fn signer_membership_reflects_multisig_update() -> anyhow::Result<()> {
    use strata_crypto::threshold_signature::ThresholdConfigUpdate;

    let sk0 = SecretKey::new(&mut OsRng);
    let sk1 = SecretKey::new(&mut OsRng);
    let sk_new = SecretKey::new(&mut OsRng);
    let pk0 = compressed_pk(&sk0);
    let pk1 = compressed_pk(&sk1);
    let pk_new = compressed_pk(&sk_new);

    let threshold = NonZero::new(2).unwrap();
    let strata_administrator = ThresholdConfig::try_new(vec![pk0, pk1], threshold)?;
    let seq_cfg = ThresholdConfig::try_new(
        vec![compressed_pk(&SecretKey::new(&mut OsRng))],
        NonZero::new(1).unwrap(),
    )?;

    let init = AdministrationInitConfig::new(
        strata_administrator,
        seq_cfg,
        2016,
        NonZero::new(10).unwrap(),
    );
    let mut state = AdministrationSubprotoState::new(&init);

    assert!(signer_is_in_role(&state, Role::StrataAdministrator, &pk0));

    let update = ThresholdConfigUpdate::new(vec![pk_new], vec![pk0], threshold);
    state.apply_multisig_update(Role::StrataAdministrator, &update)?;

    assert!(
        !signer_is_in_role(&state, Role::StrataAdministrator, &pk0),
        "removed admin key should no longer be a member"
    );
    assert!(signer_is_in_role(
        &state,
        Role::StrataAdministrator,
        &pk_new
    ));
    assert!(signer_is_in_role(&state, Role::StrataAdministrator, &pk1));

    Ok(())
}
