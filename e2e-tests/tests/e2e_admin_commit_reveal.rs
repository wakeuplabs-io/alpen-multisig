//! E2E integration test: real commit -> reveal -> broadcast for admin actions.
//!
//! This test complements `e2e_admin_subprotocol.rs` by exercising the on-chain path
//! with regtest + harness infrastructure:
//!   1. Build and sign an admin `MultisigAction`
//!   2. Build a funded envelope tx (commit-funded reveal spend)
//!   3. Mine/confirm commit funding tx
//!   4. Broadcast reveal tx and confirm inclusion
//!   5. Parse SPS payload and verify threshold signatures

use std::num::NonZero;
use std::process::Command;

use alpen_multisig_e2e_tests::test_harness::AsmTestHarnessBuilder;
use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey, SECP256K1};
use rand::rngs::OsRng;
use ssz::encode::Encode;
use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate;
use strata_asm_txs_admin::actions::{MultisigAction, Sighash, UpdateAction};
use strata_asm_txs_admin::parser::SignedPayload;
use strata_asm_txs_admin::test_utils::create_signature_set;
use strata_asm_txs_test_utils::TEST_MAGIC_BYTES;
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::{
    verify_threshold_signatures, ThresholdConfig, ThresholdConfigUpdate,
};
use strata_l1_txfmt::ParseConfig;

#[tokio::test(flavor = "multi_thread")]
async fn e2e_admin_commit_reveal_broadcast_and_verify() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!(
			"Skipping e2e_admin_commit_reveal_broadcast_and_verify: bitcoind is not available in PATH"
		);
        return;
    }

    let harness = AsmTestHarnessBuilder::default()
        .build()
        .await
        .expect("harness should build");

    // Step 1: signer setup (3 signers, threshold = 2).
    let privkeys: Vec<SecretKey> = (0..3).map(|_| SecretKey::new(&mut OsRng)).collect();
    let pubkeys: Vec<CompressedPublicKey> = privkeys
        .iter()
        .map(|sk| CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, sk)))
        .collect();
    let threshold = NonZero::new(2).expect("threshold should be non-zero");
    let config = ThresholdConfig::try_new(pubkeys, threshold).expect("valid threshold config");

    // Step 2: build an admin signer-set update action.
    let new_signer = CompressedPublicKey::from(PublicKey::from_secret_key(
        SECP256K1,
        &SecretKey::new(&mut OsRng),
    ));
    let config_update = ThresholdConfigUpdate::new(vec![new_signer], vec![], threshold);
    let action = MultisigAction::Update(UpdateAction::Multisig(MultisigUpdate::new(
        config_update,
        strata_asm_params::Role::StrataAdministrator,
    )));

    // Step 3: sign payload (2-of-3 signatures over SPS-65 sighash).
    let seqno = 1u64;
    let sighash = action.compute_sighash(seqno);
    let signer_indices: Vec<u8> = vec![0, 2];
    let signatures = create_signature_set(&privkeys, &signer_indices, sighash);
    let payload = SignedPayload::new(seqno, action.clone(), signatures).as_ssz_bytes();

    // Step 4: build reveal tx with real commit funding output.
    let secp = Secp256k1::new();
    let envelope_keypair = UntweakedKeypair::new(&secp, &mut OsRng);
    let reveal_tx = harness
        .build_envelope_tx_with_keypair(action.tag(), payload, &envelope_keypair)
        .await
        .expect("should build reveal tx with commit-funded input");
    let commit_txid = reveal_tx.input[0].previous_output.txid;

    // Confirm commit funding tx first.
    let commit_block_hash = harness
        .mine_block(None)
        .await
        .expect("should mine a block for commit confirmation");
    let commit_block = harness
        .get_block(commit_block_hash)
        .await
        .expect("should fetch mined commit block");
    assert!(
        commit_block
            .txdata
            .iter()
            .any(|tx| tx.compute_txid() == commit_txid),
        "commit tx should be included in mined block before reveal broadcast"
    );

    // Step 5: broadcast reveal tx and confirm inclusion.
    let reveal_block_hash = harness
        .submit_and_mine_tx(&reveal_tx)
        .await
        .expect("reveal tx should be broadcast and confirmed");
    let reveal_block = harness
        .get_block(reveal_block_hash)
        .await
        .expect("should fetch reveal block");
    let reveal_txid = reveal_tx.compute_txid();
    assert!(
        reveal_block
            .txdata
            .iter()
            .any(|tx| tx.compute_txid() == reveal_txid),
        "reveal tx should be included in mined block"
    );

    // Step 6: parse SPS-50/SPS-51 data and verify threshold signatures.
    let parse_config = ParseConfig::new(TEST_MAGIC_BYTES);
    let tag_data_ref = parse_config
        .try_parse_tx(&reveal_tx)
        .expect("reveal tx should contain valid SPS-50 tag");
    let tx_input = strata_asm_common::TxInputRef::new(&reveal_tx, tag_data_ref);
    let parsed = strata_asm_txs_admin::parser::parse_tx(&tx_input)
        .expect("should parse SignedPayload from reveal witness envelope");

    assert_eq!(parsed.seqno, seqno, "parsed seqno should match");
    assert_eq!(parsed.action, action, "parsed action should match");
    let verification =
        verify_threshold_signatures(&config, parsed.signatures.signatures(), &sighash.0);
    assert!(verification.is_ok(), "threshold signatures should verify");
}
