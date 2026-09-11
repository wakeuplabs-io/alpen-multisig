//! E2E test: cancel-approved-proposal flow (on-chain regtest + ASM).
//!
//! Exercises the complete cancel lifecycle against a live Bitcoin regtest node:
//!   1. Submit a `StrataAdminMultisig` Update action (adds a new signer) and mine it.
//!   2. Verify the update is queued in ASM state (not yet enacted).
//!   3. Submit a Cancel action targeting the queued update and mine it.
//!   4. Advance blocks past what would have been the activation height.
//!   5. Verify ASM state matches the initial state — the new signer was NOT added.

use std::num::NonZero;
use std::process::Command;

use alpen_multisig_e2e_tests::fixtures::decode_administration_subproto;
use alpen_multisig_e2e_tests::test_harness::AsmTestHarnessBuilder;
use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
use rand::rngs::OsRng;
use ssz::encode::Encode;
use strata_asm_params::{AdministrationInitConfig, ConfirmationDepths, Role};
use strata_asm_txs_admin::actions::updates::StrataAdminMultisigUpdate;
use strata_asm_txs_admin::actions::{CancelAction, MultisigAction, UpdateAction};
use strata_asm_txs_admin::parser::SignedPayload;
use strata_asm_txs_admin::test_utils::create_signature_set;
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::{ThresholdConfig, ThresholdConfigUpdate};

const CONFIRMATION_DEPTH: u16 = 5;

#[tokio::test(flavor = "multi_thread")]
async fn e2e_cancel_prevents_update_enactment() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!(
            "Skipping e2e_cancel_prevents_update_enactment: bitcoind is not available in PATH"
        );
        return;
    }

    // --- Setup: 3-signer 2-of-3 StrataAdministrator config ---

    let privkeys: Vec<SecretKey> = (0..3).map(|_| SecretKey::new(&mut OsRng)).collect();
    let pubkeys: Vec<CompressedPublicKey> = privkeys
        .iter()
        .map(|sk| CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, sk)))
        .collect();
    let threshold = NonZero::new(2).expect("threshold is non-zero");
    let strata_admin_config =
        ThresholdConfig::try_new(pubkeys, threshold).expect("valid threshold config");

    let other_sk = SecretKey::new(&mut OsRng);
    let other_pk = CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, &other_sk));
    let other_config = ThresholdConfig::try_new(vec![other_pk], NonZero::new(1).unwrap())
        .expect("valid 1-of-1 config");

    let admin_config = AdministrationInitConfig {
        strata_administrator: strata_admin_config,
        strata_sequencer_manager: other_config.clone(),
        alpen_administrator: other_config.clone(),
        strata_security_council: other_config,
        confirmation_depths: ConfirmationDepths {
            strata_admin_multisig_update: CONFIRMATION_DEPTH,
            strata_seq_manager_multisig_update: CONFIRMATION_DEPTH,
            alpen_admin_multisig_update: CONFIRMATION_DEPTH,
            operator_update: CONFIRMATION_DEPTH,
            sequencer_update: CONFIRMATION_DEPTH,
            ol_stf_vk_update: CONFIRMATION_DEPTH,
            asm_stf_vk_update: CONFIRMATION_DEPTH,
            ee_stf_vk_update: CONFIRMATION_DEPTH,
            strata_security_council_multisig_update: CONFIRMATION_DEPTH,
            defcon3: CONFIRMATION_DEPTH,
            safe_harbour_address_update: CONFIRMATION_DEPTH,
        },
        max_seqno_gap: NonZero::new(10).unwrap(),
    };

    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(admin_config)
        .build()
        .await
        .expect("harness should build");

    // --- Record initial state ---

    let (_, initial_asm_state) = harness
        .get_latest_asm_state()
        .expect("ASM state query should succeed")
        .expect("ASM state should be present after genesis");
    let initial_admin = decode_administration_subproto(&initial_asm_state)
        .expect("admin section should be present in initial ASM state");
    let initial_authority = initial_admin
        .authority(Role::StrataAdministrator)
        .expect("StrataAdministrator authority should exist");
    let initial_keys: Vec<CompressedPublicKey> = initial_authority.config().keys().to_vec();
    let initial_threshold = initial_authority.config().threshold();

    // --- Step 1: Broadcast Update action (add a new signer) ---

    let new_signer = CompressedPublicKey::from(PublicKey::from_secret_key(
        SECP256K1,
        &SecretKey::new(&mut OsRng),
    ));
    let config_update = ThresholdConfigUpdate::new(vec![new_signer], vec![], threshold);
    let update_inner =
        UpdateAction::StrataAdminMultisig(StrataAdminMultisigUpdate::new(config_update));
    let update = MultisigAction::Update(update_inner.clone());

    let update_seqno = 1u64;
    let signer_indices: Vec<u8> = vec![0, 2];
    let update_sigs = create_signature_set(&privkeys, &signer_indices, &update, update_seqno);
    let update_payload =
        SignedPayload::new(update_seqno, update.clone(), update_sigs).as_ssz_bytes();

    let update_reveal_tx = harness
        .build_envelope_tx(update.tag(), update_payload)
        .await
        .expect("should build update envelope tx");
    harness
        .submit_and_mine_tx(&update_reveal_tx)
        .await
        .expect("update reveal tx should be broadcast and confirmed");

    // --- Step 2: Verify the update is queued (not yet enacted) ---

    let (_, post_update_state) = harness
        .get_latest_asm_state()
        .expect("ASM state query should succeed")
        .expect("ASM state should be present");
    let post_update_admin = decode_administration_subproto(&post_update_state)
        .expect("admin section should be present");
    assert_eq!(
        post_update_admin.queued().len(),
        1,
        "update should be queued in ASM after reveal confirmation"
    );
    let queued_id = *post_update_admin.queued()[0].id();
    assert_eq!(queued_id, 0, "first queued update should have id=0");

    // --- Step 3: Broadcast Cancel action targeting the queued update ---

    // Embed the exact queued UpdateAction — required for role resolution and handler equality check.
    let queued_update_action = post_update_admin
        .find_queued(&queued_id)
        .expect("queued update should be findable by id")
        .action()
        .clone();
    let cancel = MultisigAction::Cancel(CancelAction::new(queued_id, queued_update_action));

    // seqno=2: StrataAdministrator seqno advanced to 1 after the update, so next valid is 2.
    let cancel_seqno = 2u64;
    let cancel_sigs = create_signature_set(&privkeys, &signer_indices, &cancel, cancel_seqno);
    let cancel_payload =
        SignedPayload::new(cancel_seqno, cancel.clone(), cancel_sigs).as_ssz_bytes();

    let cancel_reveal_tx = harness
        .build_envelope_tx(cancel.tag(), cancel_payload)
        .await
        .expect("should build cancel envelope tx");
    harness
        .submit_and_mine_tx(&cancel_reveal_tx)
        .await
        .expect("cancel reveal tx should be broadcast and confirmed");

    // --- Step 4: Advance blocks past what would have been the activation height ---
    // If the cancel had not worked, the update would have been enacted here.
    harness
        .mine_blocks(CONFIRMATION_DEPTH as usize + 1)
        .await
        .expect("should mine blocks past activation height");

    // --- Step 5: Verify state is unchanged ---

    let (_, final_asm_state) = harness
        .get_latest_asm_state()
        .expect("ASM state query should succeed")
        .expect("ASM state should be present");
    let final_admin = decode_administration_subproto(&final_asm_state)
        .expect("admin section should be present in final ASM state");

    assert_eq!(
        final_admin.queued().len(),
        0,
        "queue should be empty — cancel removed the update before it could activate"
    );

    let final_authority = final_admin
        .authority(Role::StrataAdministrator)
        .expect("StrataAdministrator authority should exist");
    let final_keys: Vec<CompressedPublicKey> = final_authority.config().keys().to_vec();

    assert_eq!(
        final_authority.config().threshold(),
        initial_threshold,
        "threshold should be unchanged after cancelled update"
    );
    assert_eq!(
        final_keys, initial_keys,
        "signer set should be unchanged after cancelled update — update was cancelled before enactment"
    );
    assert!(
        !final_keys.contains(&new_signer),
        "new signer should NOT be in the signer set — update was cancelled"
    );
}
