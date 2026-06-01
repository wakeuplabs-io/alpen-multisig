//! E2E: OlStfVk update — queued on reveal, enacted after confirmation depth.
//!
//! Mirrors `test_asm_predicate_update_emits_log` from the ASM test suite but
//! drives the full desktop-app path: action_codec → signing → broadcast_tx → harness.
//!
//! Flow (OlStfVk, StrataAdmin, threshold = 2):
//!   1. Encode a `VkUpdate { authority: StrataAdmin, type_id: 1 (AlwaysAccept) }` via
//!      `action_codec` — this produces an `OlStfVkUpdate` SSZ payload.
//!   2. Sign the SPS-65 sighash with A and B (mnemonic indices 0 and 1).
//!   3. Build `SignedPayload` bytes and broadcast via commit + reveal.
//!   4. Assert the admin queue holds 1 entry after the reveal block confirms.
//!   5. Mine `ol_stf_vk_update` confirmation_depth + 1 blocks.
//!   6. Assert the queue is empty — the VK update has been enacted/relayed to
//!      the checkpoint subprotocol.
//!   7. Assert the checkpoint subprotocol state reflects `PredicateTypeId::AlwaysAccept`.

#![allow(
    unused_crate_dependencies,
    reason = "test dependencies shared across test suite"
)]

use std::process::Command;

use alpen_multisig_e2e_tests::fixtures::{
    administration_init_config, assert_mnemonic_matches_strata_admin_keys,
    checkpoint_ol_stf_vk_type, decode_administration_subproto, ol_stf_vk_confirmation_depth,
    parse_admin_section, strata_admin_keys_hex, SignerUpdateEnactedFixture, FAST_ENACTMENT,
};
use alpen_multisig_e2e_tests::test_harness::AsmTestHarnessBuilder;
use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Amount;
use bitcoind_async_client::traits::{Reader, Wallet};
use rand::rngs::OsRng;
use ssz::Decode;
use strata_asm_txs_admin::actions::MultisigAction;
use strata_predicate::PredicateTypeId;

use desktop_app::domain::action::{Action, VkUpdate};
use desktop_app::domain::authority::Authority;
use desktop_app::domain::proposal::ProposalSignature;
use desktop_app::infrastructure::{action_codec, broadcast_tx, signing};

fn anyhow_string<T>(r: Result<T, String>) -> anyhow::Result<T> {
    r.map_err(|e| anyhow::anyhow!(e))
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_ol_stf_vk_update_queued_then_enacted() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!("Skipping e2e_ol_stf_vk_update_queued_then_enacted: bitcoind not in PATH");
        return;
    }
    run_ol_stf_vk_update_enacted(&FAST_ENACTMENT)
        .await
        .expect("OlStfVk update: queued then enacted");
}

async fn run_ol_stf_vk_update_enacted(fixture: &SignerUpdateEnactedFixture) -> anyhow::Result<()> {
    let mnemonic = fixture.mnemonic;
    let passphrase = fixture.passphrase;
    let path_prefix = fixture.derivation_path_prefix;
    let seq_no = fixture.seq_no;

    // Derive A and B and validate them against the fixture strata_administrator config.
    let addrs = anyhow_string(signing::list_mnemonic_addresses(mnemonic, passphrase, 2))?;
    let a_hex = addrs[0].public_key_hex.clone();
    let b_hex = addrs[1].public_key_hex.clone();

    let admin_section = parse_admin_section(fixture.admin_section_json);
    assert_mnemonic_matches_strata_admin_keys(&a_hex, &b_hex, &admin_section, path_prefix);

    let admin_cfg = administration_init_config(&admin_section);
    let confirmation_depth = ol_stf_vk_confirmation_depth(&admin_section);

    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(admin_cfg)
        .build()
        .await?;

    // Build an OlStfVk update with the AlwaysAccept predicate (type_id=1, no condition bytes).
    let action = Action::VkUpdate(VkUpdate {
        authority: Authority::StrataAdmin,
        type_id: 1, // AlwaysAccept
        condition: vec![],
    });
    let action_hex = action_codec::encode_hex(&action)?;

    // Sign with A and B (StrataAdmin requires threshold = 2).
    let sighash = anyhow_string(signing::compute_sighash(seq_no, &action_hex))?;
    let path_a = format!("{path_prefix}/0");
    let path_b = format!("{path_prefix}/1");
    let sig_a = anyhow_string(signing::sign_with_mnemonic_path(
        mnemonic,
        passphrase,
        &path_a,
        &sighash.sighash_hex,
    ))?;
    let sig_b = anyhow_string(signing::sign_with_mnemonic_path(
        mnemonic,
        passphrase,
        &path_b,
        &sighash.sighash_hex,
    ))?;
    assert_eq!(
        sig_a.public_key_hex, a_hex,
        "A key derivation/signing mismatch"
    );
    assert_eq!(
        sig_b.public_key_hex, b_hex,
        "B key derivation/signing mismatch"
    );

    let proposal_sigs = vec![
        ProposalSignature {
            signer_pubkey: a_hex.clone(),
            signature_hex: sig_a.signature_hex,
        },
        ProposalSignature {
            signer_pubkey: b_hex.clone(),
            signature_hex: sig_b.signature_hex,
        },
    ];
    let sighash_bytes: [u8; 32] = hex::decode(&sighash.sighash_hex)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("sighash must be 32 bytes"))?;
    let json_keys = strata_admin_keys_hex(&admin_section);
    let payload_bytes = anyhow_string(broadcast_tx::build_signed_payload_bytes(
        seq_no,
        &action_hex,
        &proposal_sigs,
        &json_keys,
        &sighash_bytes,
    ))?;

    // Derive commit address and build the reveal tx.
    let secp = Secp256k1::new();
    let envelope_keypair = UntweakedKeypair::new(&secp, &mut OsRng);
    let (commit_address, reveal_script, taproot_spend_info) =
        anyhow_string(broadcast_tx::derive_commit_address(
            &envelope_keypair,
            &payload_bytes,
            bitcoin::Network::Regtest,
        ))?;

    let commit_txid_str = harness
        .bitcoind
        .client
        .send_to_address(&commit_address, Amount::from_sat(10_000))?
        .0;
    let commit_txid: bitcoin::Txid = commit_txid_str.parse()?;
    let commit_tx = harness
        .client
        .get_raw_transaction_verbosity_zero(&commit_txid)
        .await?
        .0;

    // Mine the commit block before broadcasting the reveal.
    let _ = harness.mine_block(None).await?;

    let change_addr: bitcoin::Address = harness.client.get_new_address().await?;
    let change_spk = change_addr.script_pubkey();
    let action_strata = MultisigAction::from_ssz_bytes(&hex::decode(&action_hex)?)?;
    let reveal_tx = anyhow_string(broadcast_tx::build_reveal_tx(
        &envelope_keypair,
        &reveal_script,
        &taproot_spend_info,
        &commit_tx,
        &commit_address.script_pubkey(),
        &action_strata,
        harness.asm_params.magic,
        change_spk,
        1_000,
    ))?;

    let _ = harness.submit_and_mine_tx(&reveal_tx).await?;

    // After the reveal confirms: the VK update should be queued, not yet enacted.
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present after reveal"))?;
    let admin = decode_administration_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("admin section missing after reveal"))?;
    anyhow::ensure!(
        admin.queued().len() == 1,
        "VK update must be queued after reveal (got {} queued items)",
        admin.queued().len()
    );

    // Mine past activation_height = reveal_height + confirmation_depth.
    let _ = harness.mine_blocks(confirmation_depth as usize + 1).await?;

    // After activation: the queue must be empty — update relayed to checkpoint.
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present after activation"))?;
    let admin = decode_administration_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("admin section missing after activation"))?;
    anyhow::ensure!(
        admin.queued().is_empty(),
        "VK update must be enacted (queue empty) after confirmation depth (got {} queued items)",
        admin.queued().len()
    );

    // Verify the VK type was actually applied in the checkpoint subprotocol.
    let vk_type = checkpoint_ol_stf_vk_type(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("checkpoint section missing after activation"))?;
    anyhow::ensure!(
        vk_type == PredicateTypeId::AlwaysAccept,
        "checkpoint predicate must be AlwaysAccept after VK update (got {:?})",
        vk_type
    );

    Ok(())
}
