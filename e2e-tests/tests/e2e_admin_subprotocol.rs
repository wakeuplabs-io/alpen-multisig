//! End-to-end test: build, sign, and verify an admin subprotocol transaction.
//!
//! This test walks through the full admin action flow as documented in
//! `docs/2-discovery/03-poc1-findings.md`, stopping just before broadcast.
//!
//! Steps covered:
//!   1. Generate signer keys (simulates hardware wallets)
//!   2. Build a MultisigAction (signer set update)
//!   3. Compute the sighash (SPS-65 tagged hash)
//!   4. Collect ECDSA signatures from a threshold of signers
//!   5. Pack into SignedPayload
//!   6. SSZ-serialize and build the Bitcoin transaction (SPS-50 tag + SPS-51 envelope)
//!   7. Parse back and verify signatures against the threshold config
//!
//! Step 8 (broadcast) is intentionally skipped — in production, the reveal tx
//! would be broadcast to the Bitcoin network via `bitcoind-async-client` or
//! copied as raw hex for manual broadcast.

use std::num::NonZero;

use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
use rand::rngs::OsRng;

use strata_asm_txs_admin::actions::updates::StrataAdminMultisigUpdate;
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_asm_txs_admin::signing_message::SigningMessage;
use strata_asm_txs_admin::test_utils::create_test_admin_tx;
use strata_asm_txs_test_utils::TEST_MAGIC_BYTES;
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::verify_threshold_signatures;
use strata_crypto::threshold_signature::ThresholdConfig;
use strata_crypto::threshold_signature::ThresholdConfigUpdate;
use strata_l1_txfmt::ParseConfig;

/// Full end-to-end flow: create an admin signer-set update, sign it, build the
/// Bitcoin transaction, then parse it back and verify.
#[test]
fn e2e_build_and_verify_admin_signer_update() {
    // ---------------------------------------------------------------
    // Step 1: Generate signer keys (3 signers, threshold = 2)
    // In production these come from hardware wallets via HWI.
    // ---------------------------------------------------------------
    let privkeys: Vec<SecretKey> = (0..3).map(|_| SecretKey::new(&mut OsRng)).collect();

    let pubkeys: Vec<CompressedPublicKey> = privkeys
        .iter()
        .map(|sk| CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, sk)))
        .collect();

    let threshold = NonZero::new(2).unwrap();
    let config =
        ThresholdConfig::try_new(pubkeys.clone(), threshold).expect("valid threshold config");

    // ---------------------------------------------------------------
    // Step 2: Build a MultisigAction — update the Strata Admin signer set
    // This is the simplest update: add a new key, keep threshold at 2.
    // ---------------------------------------------------------------
    let new_signer = CompressedPublicKey::from(PublicKey::from_secret_key(
        SECP256K1,
        &SecretKey::new(&mut OsRng),
    ));

    let config_update = ThresholdConfigUpdate::new(
        vec![new_signer], // add_keys
        vec![],           // remove_keys
        threshold,        // new threshold (unchanged)
    );

    let strata_update = StrataAdminMultisigUpdate::new(config_update);

    let action = MultisigAction::Update(UpdateAction::StrataAdminMultisig(strata_update));

    // ---------------------------------------------------------------
    // Step 3: Compute the sighash (SPS-65 tagged hash)
    // Formula: SHA256(SHA256("strata/admin/strata_admin_multisig_update") || seqno_be || payload)
    // ---------------------------------------------------------------
    let seqno: u64 = 1; // first action, seqno must be > last_seqno (0)
    let sighash = SigningMessage::for_action(&action, seqno).compute_sighash();

    // Sighash is a 32-byte hash that each signer will sign
    assert_eq!(sighash.0.len(), 32, "sighash must be 32 bytes");

    // ---------------------------------------------------------------
    // Steps 4-6: Sign, pack into SignedPayload, serialize, and build
    // the Bitcoin transaction (SPS-50 OP_RETURN + SPS-51 envelope).
    //
    // `create_test_admin_tx` does all of this internally:
    //   - Signs the sighash with the selected private keys
    //   - Packs signatures into a SignatureSet
    //   - Creates a SignedPayload { seqno, action, signatures }
    //   - SSZ-serializes the payload
    //   - Builds the SPS-51 witness envelope (chunked at 520 bytes)
    //   - Adds the SPS-50 OP_RETURN tag
    //   - Returns a complete Bitcoin reveal transaction
    // ---------------------------------------------------------------
    let signer_indices: Vec<u8> = vec![0, 2]; // signers 0 and 2 reach threshold of 2
    let reveal_tx = create_test_admin_tx(&privkeys, &signer_indices, &action, seqno);

    // ---------------------------------------------------------------
    // Step 7: Parse back and verify
    // This simulates what the ASM does when it scans a Bitcoin block:
    //   - Finds the OP_RETURN tag (SPS-50)
    //   - Extracts the SignedPayload from the witness envelope (SPS-51)
    //   - Verifies threshold signatures against the authority config
    // ---------------------------------------------------------------

    // Parse the SPS-50 tag from the transaction
    let parse_config = ParseConfig::new(TEST_MAGIC_BYTES);
    let tag_data_ref = parse_config
        .try_parse_tx(&reveal_tx)
        .expect("transaction should have a valid SPS-50 tag");

    // Build a TxInputRef (how the ASM sees the transaction)
    let tx_input = strata_asm_common::TxInputRef::new(&reveal_tx, tag_data_ref);

    // Parse the SignedPayload from the witness envelope
    let parsed = strata_asm_txs_admin::parser::parse_tx(&tx_input)
        .expect("should parse SignedPayload from witness envelope");

    // Verify: parsed data matches what we sent
    assert_eq!(parsed.seqno, seqno, "seqno should match");
    assert_eq!(parsed.action, action, "action should match");

    // Verify threshold signatures against the signer config
    let verification =
        verify_threshold_signatures(&config, parsed.signatures.signatures(), &sighash.0);
    assert!(verification.is_ok(), "threshold signatures should verify");

    // ---------------------------------------------------------------
    // Step 8 (NOT DONE): Broadcast
    //
    // In production, this reveal_tx would be broadcast to Bitcoin:
    //   let client = bitcoind_async_client::new(...);
    //   client.send_raw_transaction(&reveal_tx).await?;
    //
    // Or the user can copy the raw hex and broadcast manually:
    //   bitcoin::consensus::encode::serialize_hex(&reveal_tx)
    //
    // The ASM running inside the Strata node would then pick up the
    // transaction from the Bitcoin block, parse it (as we did above),
    // and enqueue the update with activation_height = current + 2016.
    // ---------------------------------------------------------------

    println!("E2E admin flow completed successfully:");
    println!("  - Action: Strata Admin signer set update (add 1 key)");
    println!("  - Signers: 2 of 3 (indices 0, 2)");
    println!("  - SeqNo: {seqno}");
    println!(
        "  - Tx inputs: {}, outputs: {}",
        reveal_tx.input.len(),
        reveal_tx.output.len()
    );
    println!("  - Signatures verified against threshold config");
    println!("  - Ready for broadcast (skipped in this test)");
}
