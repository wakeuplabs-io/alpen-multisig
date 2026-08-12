//! Upstream capability probe for the Security Council Defcon levers.
//!
//! This is the go/no-go gate for the Security Council feature: it proves the pinned ASM
//! really exposes everything the product will need, before any product code exists.
//!
//! Deliberately bypasses the domain `Action` / `action_codec` layer (which has no Defcon
//! variants yet) and builds [`MultisigAction`] straight from the Alpen crates, reusing only
//! the generic signing/broadcast utilities that operate on an opaque SSZ `action_hex`.
//!
//! Covered:
//! - Defcon 1 activates the bridge safe harbour in the same block as the reveal (depth 0).
//! - Defcon 3 stays queued until its configured depth elapses, then activates.
//! - The Defcon 1 signing message renders exactly the four canonical lines, with no details
//!   block.

use std::process::Command;

use alpen_multisig_e2e_tests::fixtures::{
    administration_init_config, decode_administration_subproto, decode_bridge_subproto,
    defcon3_confirmation_depth, parse_admin_section, strata_security_council_keys_hex,
    SignerUpdateEnactedFixture, DEMO_COSIGN_MNEMONIC, FAST_ENACTMENT,
};
use alpen_multisig_e2e_tests::test_harness::{AsmTestHarness, AsmTestHarnessBuilder};
use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Amount;
use bitcoind_async_client::traits::Reader;
use rand::rngs::OsRng;
use ssz::Encode;
use strata_asm_params::Role;
use strata_asm_txs_admin::actions::updates::{Defcon1Update, Defcon3Update};
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_asm_txs_admin::signing_message::SigningMessage;

use desktop_app::domain::proposal::ProposalSignature;
use desktop_app::infrastructure::{broadcast_tx, signing};

fn anyhow_string<T>(r: Result<T, String>) -> anyhow::Result<T> {
    r.map_err(|e| anyhow::anyhow!(e))
}

/// Defcon 1 has confirmation depth 0 upstream, so it must apply inside the reveal block
/// itself: no queue entry, safe harbour already activated, council seqno advanced.
#[tokio::test(flavor = "multi_thread")]
async fn e2e_defcon1_activates_safe_harbour_in_the_reveal_block() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!("Skipping e2e_defcon1_activates_safe_harbour_in_the_reveal_block: bitcoind is not available in PATH");
        return;
    }
    run_defcon1(&FAST_ENACTMENT)
        .await
        .expect("defcon 1 activates the safe harbour immediately");
}

/// Defcon 3 is the delayed lever: queued on reveal, applied only once its configured
/// confirmation depth has elapsed.
#[tokio::test(flavor = "multi_thread")]
async fn e2e_defcon3_activates_safe_harbour_only_after_its_depth() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!("Skipping e2e_defcon3_activates_safe_harbour_only_after_its_depth: bitcoind is not available in PATH");
        return;
    }
    run_defcon3(&FAST_ENACTMENT)
        .await
        .expect("defcon 3 activates the safe harbour after its confirmation depth");
}

/// The signer sees exactly these four lines. Defcon 1 carries no payload, so the rendered
/// message must have no `Action Details:` block at all — a signer who sees one is looking at
/// a different action.
#[test]
fn defcon1_signing_message_renders_the_four_canonical_lines() {
    let action = MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update));

    let message = SigningMessage::for_action(&action, 1);

    assert_eq!(
        message.as_str(),
        "Strata ASM Administration v1\n\
         Action: Defcon 1\n\
         Authorized By: Strata Security Council\n\
         Sequence: 1",
    );
    assert!(
        !message.as_str().contains("Action Details:"),
        "Defcon 1 carries no payload, so it must render without a details block"
    );
}

async fn run_defcon1(fixture: &SignerUpdateEnactedFixture) -> anyhow::Result<()> {
    let admin_section = parse_admin_section(fixture.admin_section_json);
    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(administration_init_config(&admin_section))
        .build()
        .await?;

    let initial = bridge_safe_harbour_activated(&harness)?;
    anyhow::ensure!(!initial, "safe harbour must start deactivated");
    let seqno_before = council_last_seqno(&harness)?;

    let action = MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update));
    submit_council_action(&harness, fixture, &admin_section, &action).await?;

    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let bridge = decode_bridge_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("bridge section missing"))?;
    anyhow::ensure!(
        bridge.safe_harbour().is_activated(),
        "Defcon 1 must activate the safe harbour in the reveal block"
    );

    let admin = decode_administration_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("admin section missing"))?;
    anyhow::ensure!(
        admin.queued().is_empty(),
        "Defcon 1 must bypass the admin queue, found {} queued update(s)",
        admin.queued().len()
    );

    let seqno_after = council_last_seqno(&harness)?;
    anyhow::ensure!(
        seqno_after > seqno_before,
        "security council seqno must advance ({seqno_before} -> {seqno_after})"
    );

    Ok(())
}

async fn run_defcon3(fixture: &SignerUpdateEnactedFixture) -> anyhow::Result<()> {
    let admin_section = parse_admin_section(fixture.admin_section_json);
    let depth = defcon3_confirmation_depth(&admin_section);
    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(administration_init_config(&admin_section))
        .build()
        .await?;

    let action = MultisigAction::Update(UpdateAction::Defcon3(Defcon3Update));
    submit_council_action(&harness, fixture, &admin_section, &action).await?;

    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let admin = decode_administration_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("admin section missing"))?;
    anyhow::ensure!(
        admin
            .queued()
            .iter()
            .any(|q| matches!(q.action(), UpdateAction::Defcon3(_))),
        "Defcon 3 must sit in the admin queue before its depth elapses"
    );
    anyhow::ensure!(
        !bridge_safe_harbour_activated(&harness)?,
        "safe harbour must stay deactivated while Defcon 3 is queued"
    );

    // `process_queued` drains at `activation_height <= tip`, and the activation height is
    // the reveal height plus the depth, so exactly `depth` further blocks are needed.
    let _ = harness.mine_blocks(depth as usize).await?;

    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let admin = decode_administration_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("admin section missing"))?;
    anyhow::ensure!(
        !admin
            .queued()
            .iter()
            .any(|q| matches!(q.action(), UpdateAction::Defcon3(_))),
        "Defcon 3 must leave the queue once enacted"
    );
    let bridge = decode_bridge_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("bridge section missing"))?;
    anyhow::ensure!(
        bridge.safe_harbour().is_activated(),
        "Defcon 3 must activate the safe harbour after its confirmation depth"
    );

    Ok(())
}

/// Sign `action` with both security-council keys and drive it through commit → reveal,
/// returning once the reveal block has been processed by the ASM worker.
async fn submit_council_action(
    harness: &AsmTestHarness,
    fixture: &SignerUpdateEnactedFixture,
    admin_section: &serde_json::Value,
    action: &MultisigAction,
) -> anyhow::Result<()> {
    let passphrase = fixture.passphrase;
    let path = format!("{}/0", fixture.derivation_path_prefix);
    let seq_no = fixture.seq_no;

    // The fixture gives the council the same two keys as the administrator, so the demo
    // mnemonic pair can reach the council threshold of 2.
    let council_keys = strata_security_council_keys_hex(admin_section);
    let a_hex = anyhow_string(signing::list_mnemonic_addresses(
        fixture.mnemonic,
        passphrase,
        1,
    ))?[0]
        .public_key_hex
        .clone();
    let b_hex = anyhow_string(signing::list_mnemonic_addresses(
        DEMO_COSIGN_MNEMONIC,
        passphrase,
        1,
    ))?[0]
        .public_key_hex
        .clone();
    anyhow::ensure!(
        council_keys == vec![a_hex.clone(), b_hex.clone()],
        "fixture strata_security_council keys must match the demo mnemonics"
    );

    let action_hex = hex::encode(action.as_ssz_bytes());
    let sighash = anyhow_string(signing::compute_sighash(seq_no, &action_hex))?;
    let sig_a = anyhow_string(signing::sign_with_mnemonic_path(
        fixture.mnemonic,
        passphrase,
        &path,
        &sighash.sighash_hex,
    ))?;
    let sig_b = anyhow_string(signing::sign_with_mnemonic_path(
        DEMO_COSIGN_MNEMONIC,
        passphrase,
        &path,
        &sighash.sighash_hex,
    ))?;
    let proposal_sigs = vec![
        ProposalSignature {
            signer_pubkey: a_hex,
            signature_hex: sig_a.signature_hex,
        },
        ProposalSignature {
            signer_pubkey: b_hex,
            signature_hex: sig_b.signature_hex,
        },
    ];

    let sighash_bytes: [u8; 32] = hex::decode(&sighash.sighash_hex)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("sighash_hex must be 32 bytes"))?;
    let payload_bytes = anyhow_string(broadcast_tx::build_signed_payload_bytes(
        seq_no,
        &action_hex,
        &proposal_sigs,
        &council_keys,
        &sighash_bytes,
    ))?;

    let secp = Secp256k1::new();
    let envelope_keypair = UntweakedKeypair::new(&secp, &mut OsRng);
    let (commit_address, reveal_script, taproot_spend_info) =
        anyhow_string(broadcast_tx::derive_commit_address(
            &envelope_keypair,
            &payload_bytes,
            bitcoin::Network::Regtest,
        ))?;

    let commit_txid: bitcoin::Txid = harness
        .bitcoind
        .client
        .send_to_address(&commit_address, Amount::from_sat(10_000))?
        .0
        .parse()?;
    let commit_tx = harness
        .client
        .get_raw_transaction_verbosity_zero(&commit_txid)
        .await?
        .0;
    let _ = harness.mine_block(None).await?;

    let change_keypair = UntweakedKeypair::new(&secp, &mut OsRng);
    let change_pubkey = bitcoin::CompressedPublicKey::from_private_key(
        &secp,
        &bitcoin::PrivateKey::new(change_keypair.secret_key(), bitcoin::Network::Regtest),
    )
    .expect("valid compressed public key");
    let change_spk =
        bitcoin::Address::p2wpkh(&change_pubkey, bitcoin::Network::Regtest).script_pubkey();
    let reveal_tx = anyhow_string(broadcast_tx::build_reveal_tx(
        &envelope_keypair,
        &reveal_script,
        &taproot_spend_info,
        &commit_tx,
        &commit_address.script_pubkey(),
        action,
        harness.asm_params.magic,
        change_spk,
        1_000,
    ))?;

    let _ = harness.submit_and_mine_tx(&reveal_tx).await?;

    Ok(())
}

fn bridge_safe_harbour_activated(harness: &AsmTestHarness) -> anyhow::Result<bool> {
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let bridge = decode_bridge_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("bridge section missing"))?;
    Ok(bridge.safe_harbour().is_activated())
}

fn council_last_seqno(harness: &AsmTestHarness) -> anyhow::Result<u64> {
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let admin = decode_administration_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("admin section missing"))?;
    Ok(admin
        .authority(Role::StrataSecurityCouncil)
        .ok_or_else(|| anyhow::anyhow!("security council authority missing"))?
        .last_seqno())
}
