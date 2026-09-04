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
//! - A Defcon 3 cancelled by the council while queued leaves the queue and never activates the
//!   safe harbour, even past the height it would have activated at.

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
use strata_asm_txs_admin::actions::{CancelAction, MultisigAction, UpdateAction};
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

/// A Defcon 3 cancelled by the council while it sits in the queue must never activate the safe
/// harbour, even once the chain passes the height it would have activated at (Constraint 3).
#[tokio::test(flavor = "multi_thread")]
async fn e2e_defcon3_canceled_never_activates_the_safe_harbour() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!("Skipping e2e_defcon3_canceled_never_activates_the_safe_harbour: bitcoind is not available in PATH");
        return;
    }
    run_defcon3_canceled(&FAST_ENACTMENT)
        .await
        .expect("a cancelled defcon 3 never activates the safe harbour");
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
    let _ =
        submit_council_action(&harness, fixture, &admin_section, &action, fixture.seq_no).await?;

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
    let _ =
        submit_council_action(&harness, fixture, &admin_section, &action, fixture.seq_no).await?;

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

async fn run_defcon3_canceled(fixture: &SignerUpdateEnactedFixture) -> anyhow::Result<()> {
    let admin_section = parse_admin_section(fixture.admin_section_json);
    let depth = defcon3_confirmation_depth(&admin_section) as u64;
    anyhow::ensure!(
        depth > 0,
        "the fixture must configure a non-zero defcon3 depth: at 0 there is no window to cancel in"
    );

    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(administration_init_config(&admin_section))
        .build()
        .await?;
    anyhow::ensure!(
        !bridge_safe_harbour_activated(&harness)?,
        "safe harbour must start deactivated"
    );

    // 1 — queue a Defcon 3.
    let action = MultisigAction::Update(UpdateAction::Defcon3(Defcon3Update));
    let reveal_height =
        submit_council_action(&harness, fixture, &admin_section, &action, fixture.seq_no).await?;
    // `process_queued` drains at `activation_height <= tip`, and the activation height is the
    // reveal height plus the depth.
    let activation_height = reveal_height + depth;

    let (queued_id, queued_action) = queued_defcon3(&harness)?.ok_or_else(|| {
        anyhow::anyhow!("Defcon 3 must sit in the admin queue before its depth elapses")
    })?;
    anyhow::ensure!(
        !bridge_safe_harbour_activated(&harness)?,
        "safe harbour must stay off while the Defcon 3 is queued"
    );

    // 2 — cancel it, signed by the same council.
    //
    // A cancel's authorizing role is the role of the update it cancels, so a Defcon 3 cancel is a
    // council action. Upstream consumed the council seqno when it *accepted* the Defcon 3 at the
    // reveal — not when the queued entry matures — so the next valid seqno is `+ 1`.
    //
    // The queued `UpdateAction` is embedded verbatim rather than reconstructed: the upstream
    // handler resolves the role from it and checks it for equality against the queue entry.
    let cancel = MultisigAction::Cancel(CancelAction::new(queued_id, queued_action));
    let cancel_height = submit_council_action(
        &harness,
        fixture,
        &admin_section,
        &cancel,
        fixture.seq_no + 1,
    )
    .await?;
    anyhow::ensure!(
        cancel_height <= activation_height,
        "the cancel must land inside the window (landed at {cancel_height}, activation {activation_height}); \
         past it upstream rejects it as UnknownAction and the queue would be empty because the update enacted"
    );

    // A cancel has depth 0, so the entry is gone in the cancel's own reveal block.
    anyhow::ensure!(
        queued_defcon3(&harness)?.is_none(),
        "the cancel must remove the Defcon 3 from the queue"
    );
    anyhow::ensure!(
        !bridge_safe_harbour_activated(&harness)?,
        "the cancel must not activate the harbour it removed"
    );

    // 3 — take the tip past the height the Defcon 3 would have activated at. Measured, not
    // assumed.
    let tip = harness.get_chain_tip().await?;
    let _ = harness
        .mine_blocks((activation_height + 1).saturating_sub(tip) as usize)
        .await?;
    let tip = harness.get_chain_tip().await?;
    anyhow::ensure!(
        tip > activation_height,
        "tip {tip} must have passed the original activation height {activation_height}"
    );

    // 4 — Constraint 3: leaving the queue is not evidence of enactment.
    anyhow::ensure!(
        queued_defcon3(&harness)?.is_none(),
        "the queue must stay empty past the activation height"
    );
    anyhow::ensure!(
        !bridge_safe_harbour_activated(&harness)?,
        "a cancelled Defcon 3 must never activate the safe harbour"
    );

    // Both actions were accepted by the council, not silently dropped. Never `==`: the council
    // may accept further actions, exactly as Constraint 2 says. `last > fixture.seq_no` is
    // `last >= fixture.seq_no + 1` without tripping clippy's `int_plus_one` lint.
    let last = council_last_seqno(&harness)?;
    anyhow::ensure!(
        last > fixture.seq_no,
        "the council seqno must have consumed the cancel (is {last})"
    );

    Ok(())
}

/// The single queued Defcon 3, if any — its `UpdateId` and the entry's `UpdateAction`.
///
/// Asserts that at most one Defcon 3 is queued: two are byte-identical, and the contract records
/// that as a recorded ambiguity rather than defining resolution order.
fn queued_defcon3(harness: &AsmTestHarness) -> anyhow::Result<Option<(u32, UpdateAction)>> {
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let admin = decode_administration_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("admin section missing"))?;

    let mut matches = admin
        .queued()
        .iter()
        .filter(|q| matches!(q.action(), UpdateAction::Defcon3(_)));
    let first = matches.next();
    anyhow::ensure!(
        matches.next().is_none(),
        "at most one Defcon 3 is expected to be queued at a time"
    );

    Ok(first.map(|q| (*q.id(), q.action().clone())))
}

/// Sign `action` at `seq_no` with both security-council keys, drive it through commit → reveal,
/// and return the height of the block the reveal landed in.
///
/// The height is returned rather than counted by the caller: this function mines one block for the
/// commit and then up to ten until the reveal confirms, so any arithmetic done from a caller's
/// guess about the tip is a race.
async fn submit_council_action(
    harness: &AsmTestHarness,
    fixture: &SignerUpdateEnactedFixture,
    admin_section: &serde_json::Value,
    action: &MultisigAction,
    seq_no: u64,
) -> anyhow::Result<u64> {
    let passphrase = fixture.passphrase;
    let path = format!("{}/0", fixture.derivation_path_prefix);

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

    let reveal_block_hash = harness.submit_and_mine_tx(&reveal_tx).await?;
    let reveal_height = harness.client.get_block_height(&reveal_block_hash).await?;

    Ok(reveal_height)
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
