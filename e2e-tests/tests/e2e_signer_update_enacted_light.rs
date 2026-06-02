//! Lightweight enacted e2e test (no orchestrator, real regtest + ASM).
//!
//! Drives the desktop signing & broadcast modules end-to-end against a regtest
//! ASM harness, **without** spinning up the orchestrator backend or HTTP layer.
//!
//! Flow:
//!   1. Derive A, B, D from a known mnemonic on BIP-84 `m/84'/0'/73'/0/n` (see fixtures).
//!   2. Assert A and B match the canonical `strata_administrator` keys in the fixture JSON.
//!   3. Boot the harness with `AdministrationInitConfig` parsed from the JSON.
//!   4. Build a `MultisigUpdate` action (add D, threshold = 2) via the desktop
//!      action codec, sign with A and B via the desktop mnemonic-signing path.
//!   5. Build a `SignedPayload` SSZ blob via the desktop broadcast helper, then
//!      derive the commit address, fund + mine the commit, build and submit the
//!      reveal transaction, and mine past `confirmation_depth` to trigger the
//!      enactment of the queued update.
//!   6. Read the ASM `AdministrationSubprotoState` directly from harness state
//!      and assert that D is now part of the canonical signer set and that
//!      `last_seqno` has advanced.
//!
//! What this proves (that neither existing e2e test covers):
//!   - Desktop-generated signatures round-trip correctly through the on-chain
//!     SPS-50/SPS-51 path produced by `broadcast_tx::build_signed_payload_bytes`.
//!   - After mining past `activation_height`, the canonical ASM admin state
//!     actually reflects the added signer.

use std::num::NonZeroU8;
use std::process::Command;

use alpen_multisig_e2e_tests::fixtures::{
    administration_init_config, assert_mnemonic_matches_strata_admin_keys,
    decode_administration_subproto, parse_admin_section, strata_admin_confirmation_depth,
    strata_admin_keys_hex, SignerUpdateEnactedFixture, DEFAULT_REPO_ASM, DEMO_COSIGN_MNEMONIC,
    FAST_ENACTMENT,
};
use alpen_multisig_e2e_tests::test_harness::AsmTestHarnessBuilder;
use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Amount;
use bitcoind_async_client::traits::Reader;
use rand::rngs::OsRng;
use ssz::Decode;
use strata_asm_params::Role;
use strata_asm_txs_admin::actions::MultisigAction;
use strata_crypto::keys::compressed::CompressedPublicKey;

use desktop_app::domain::action::{Action, CompressedPubKey, MultisigUpdate};
use desktop_app::domain::authority::Authority;
use desktop_app::domain::proposal::ProposalSignature;
use desktop_app::infrastructure::{action_codec, broadcast_tx, signing};

fn anyhow_string<T>(r: Result<T, String>) -> anyhow::Result<T> {
    r.map_err(|e| anyhow::anyhow!(e))
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_signer_update_enacted_lightweight() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!(
            "Skipping e2e_signer_update_enacted_lightweight: bitcoind is not available in PATH"
        );
        return;
    }
    run_signer_update_enacted(&DEFAULT_REPO_ASM)
        .await
        .expect("enacted signer update (default confirmation depth)");
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_signer_update_enacted_fast_confirmation() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!(
            "Skipping e2e_signer_update_enacted_fast_confirmation: bitcoind is not available in PATH"
        );
        return;
    }
    run_signer_update_enacted(&FAST_ENACTMENT)
        .await
        .expect("enacted signer update (fast confirmation depth)");
}

async fn run_signer_update_enacted(fixture: &SignerUpdateEnactedFixture) -> anyhow::Result<()> {
    let mnemonic = fixture.mnemonic;
    let passphrase = fixture.passphrase;
    let path_prefix = fixture.derivation_path_prefix;
    let seq_no = fixture.seq_no;

    let addrs = anyhow_string(signing::list_mnemonic_addresses(mnemonic, passphrase, 3))?;
    let cosign_addrs =
        anyhow_string(signing::list_mnemonic_addresses(DEMO_COSIGN_MNEMONIC, passphrase, 1))?;
    let a_hex = addrs[0].public_key_hex.clone();
    let b_hex = cosign_addrs[0].public_key_hex.clone();
    let d_hex = addrs[2].public_key_hex.clone();

    let admin_section = parse_admin_section(fixture.admin_section_json);
    assert_mnemonic_matches_strata_admin_keys(&a_hex, &b_hex, &admin_section, path_prefix);

    let admin_cfg = administration_init_config(&admin_section);
    let confirmation_depth = strata_admin_confirmation_depth(&admin_section);

    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(admin_cfg)
        .build()
        .await?;

    let json_keys = strata_admin_keys_hex(&admin_section);

    let d_bytes: [u8; 33] = hex::decode(&d_hex)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("d_hex must be 33 bytes"))?;
    let action = Action::MultisigUpdate(MultisigUpdate {
        role: Authority::StrataAdmin,
        add_keys: vec![CompressedPubKey::new(d_bytes)],
        remove_keys: vec![],
        new_threshold: NonZeroU8::new(2).expect("non-zero threshold"),
    });
    let action_hex = action_codec::encode_hex(&action)?;

    let sighash = anyhow_string(signing::compute_sighash(seq_no, &action_hex))?;
    let path_a = format!("{path_prefix}/0");
    let path_b = format!("{path_prefix}/0");
    let sig_a = anyhow_string(signing::sign_with_mnemonic_path(
        mnemonic,
        passphrase,
        &path_a,
        &sighash.sighash_hex,
    ))?;
    let sig_b = anyhow_string(signing::sign_with_mnemonic_path(
        DEMO_COSIGN_MNEMONIC,
        passphrase,
        &path_b,
        &sighash.sighash_hex,
    ))?;
    assert_eq!(sig_a.public_key_hex, a_hex, "A derivation/signing mismatch");
    assert_eq!(sig_b.public_key_hex, b_hex, "B derivation/signing mismatch");

    let verify = anyhow_string(signing::verify_threshold(
        &[a_hex.clone(), b_hex.clone()],
        2,
        &[sig_a.signature_hex.clone(), sig_b.signature_hex.clone()],
        &sighash.sighash_hex,
    ))?;
    anyhow::ensure!(verify.valid, "threshold-of-2 must validate both signatures");

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
        .map_err(|_| anyhow::anyhow!("sighash_hex must be 32 bytes"))?;
    let payload_bytes = anyhow_string(broadcast_tx::build_signed_payload_bytes(
        seq_no,
        &action_hex,
        &proposal_sigs,
        &json_keys,
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

    let _ = harness.mine_block(None).await?;

    let action_strata = MultisigAction::from_ssz_bytes(&hex::decode(&action_hex)?)?;
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
        &action_strata,
        harness.asm_params.magic,
        change_spk,
        1_000,
    ))?;

    let _ = harness.submit_and_mine_tx(&reveal_tx).await?;

    let _ = harness.mine_blocks(confirmation_depth as usize + 1).await?;

    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let admin = decode_administration_subproto(&asm_state).ok_or_else(|| {
        anyhow::anyhow!(
            "fixture {}: admin section missing from ASM state",
            fixture.name
        )
    })?;
    let authority = admin
        .authority(Role::StrataAdministrator)
        .ok_or_else(|| anyhow::anyhow!("strata administrator authority missing"))?;
    let canonical_hex: Vec<String> = authority
        .config()
        .keys()
        .iter()
        .map(|k: &CompressedPublicKey| hex::encode(k.serialize()))
        .collect();
    anyhow::ensure!(
        canonical_hex.contains(&d_hex),
        "fixture {}: signer D not in enacted keys: {canonical_hex:?}",
        fixture.name
    );
    anyhow::ensure!(
        canonical_hex.contains(&a_hex) && canonical_hex.contains(&b_hex),
        "fixture {}: original A/B missing from enacted keys: {canonical_hex:?}",
        fixture.name
    );
    anyhow::ensure!(
        authority.last_seqno() == seq_no,
        "fixture {}: last_seqno expected {} got {}",
        fixture.name,
        seq_no,
        authority.last_seqno()
    );

    Ok(())
}
