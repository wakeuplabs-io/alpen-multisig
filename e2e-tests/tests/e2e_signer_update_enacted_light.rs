//! Lightweight enacted e2e test (no orchestrator, real regtest + ASM).
//!
//! Drives the desktop signing & broadcast modules end-to-end against a regtest
//! ASM harness, **without** spinning up the orchestrator backend or HTTP layer.
//!
//! Flow:
//!   1. Derive A, B, D from a known mnemonic on BIP-84 `m/84'/0'/73'/0/n`.
//!   2. Assert A and B match the canonical `strata_administrator` keys in the
//!      inlined `asm-params.json` snippet.
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

use alpen_multisig_e2e_tests::test_harness::AsmTestHarnessBuilder;
use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Amount;
use bitcoind_async_client::traits::Reader;
use rand::rngs::OsRng;
use ssz::Decode;
use strata_asm_common::Subprotocol;
use strata_asm_params::{AdministrationInitConfig, Role};
use strata_asm_proto_admin::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_asm_txs_admin::actions::MultisigAction;
use strata_asm_worker::AsmState;
use strata_crypto::keys::compressed::CompressedPublicKey;

use desktop_app::domain::action::{Action, CompressedPubKey, MultisigUpdate};
use desktop_app::domain::authority::Authority;
use desktop_app::domain::proposal::ProposalSignature;
use desktop_app::infrastructure::{action_codec, broadcast_tx, signing};

const MNEMONIC: &str =
    "multiply toss magic exclude crawl obey garden black apart room village neglect";
const PASSPHRASE: &str = "";
const PATH_PREFIX: &str = "m/84'/0'/73'/0";

const SEQ_NO: u64 = 1;

/// Inlined Admin subsection from `repo/asm/asm-params.json` (only the part
/// needed to construct `AdministrationInitConfig`; the rest is irrelevant since
/// the harness regenerates non-admin subprotocol configs).
const ASM_PARAMS_ADMIN_JSON: &str = r#"{
  "strata_administrator": {
    "keys": [
      "02300dc42e67165c78256d5ef816bad845428841f54f1ecb6da8a3eb1d066f4df7",
      "037f67048bec090ecf441024f5928ac9bd15c45401a92ee8afdf6f653c7fdcd4d7"
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
  "confirmation_depths": {
    "strata_admin_multisig_update": 144,
    "strata_seq_manager_multisig_update": 144,
    "alpen_admin_multisig_update": 144,
    "operator_update": 144,
    "sequencer_update": 144,
    "ol_stf_vk_update": 144,
    "asm_stf_vk_update": 144,
    "ee_stf_vk_update": 144
  },
  "max_seqno_gap": 10
}"#;

#[tokio::test(flavor = "multi_thread")]
async fn e2e_signer_update_enacted_lightweight() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!(
            "Skipping e2e_signer_update_enacted_lightweight: bitcoind is not available in PATH"
        );
        return;
    }

    // 1. Derive A (idx 0), B (idx 1), D (idx 2) from the mnemonic.
    let addrs = signing::list_mnemonic_addresses(MNEMONIC, PASSPHRASE, 3)
        .expect("derive mnemonic addresses");
    let a_hex = addrs[0].public_key_hex.clone();
    let b_hex = addrs[1].public_key_hex.clone();
    let d_hex = addrs[2].public_key_hex.clone();

    // 2. Parse the admin section JSON and assert that A and B match the
    //    canonical strata_administrator keys. This anchors the mnemonic-derived
    //    keys to the production-style config without manual upkeep.
    let admin_section: serde_json::Value =
        serde_json::from_str(ASM_PARAMS_ADMIN_JSON).expect("parse asm-params admin section");
    let json_keys: Vec<String> = admin_section["strata_administrator"]["keys"]
        .as_array()
        .expect("strata_administrator.keys is an array")
        .iter()
        .map(|v| v.as_str().expect("each key is a string").to_string())
        .collect();
    assert_eq!(
        a_hex, json_keys[0],
        "mnemonic m/86'/0'/73'/0/0 must match asm-params strata_administrator.keys[0]"
    );
    assert_eq!(
        b_hex, json_keys[1],
        "mnemonic m/86'/0'/73'/0/1 must match asm-params strata_administrator.keys[1]"
    );

    // 3. Build AdministrationInitConfig directly from the JSON admin section.
    let admin_cfg: AdministrationInitConfig = serde_json::from_value(admin_section.clone())
        .expect("admin section deserializes into AdministrationInitConfig");
    let confirmation_depth: u16 =
        admin_section["confirmation_depths"]["strata_admin_multisig_update"]
            .as_u64()
            .expect("confirmation_depths.strata_admin_multisig_update is u64") as u16;

    // 4. Boot the harness with the configured admin authority.
    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(admin_cfg)
        .build()
        .await
        .expect("harness should build");

    // 5. Build the domain Action that adds signer D and keeps threshold 2.
    let d_bytes: [u8; 33] = hex::decode(&d_hex)
        .expect("d_hex decodes")
        .try_into()
        .expect("d_hex is 33 bytes");
    let action = Action::MultisigUpdate(MultisigUpdate {
        role: Authority::StrataAdmin,
        add_keys: vec![CompressedPubKey::new(d_bytes)],
        remove_keys: vec![],
        new_threshold: NonZeroU8::new(2).expect("non-zero threshold"),
    });
    let action_hex = action_codec::encode_hex(&action).expect("encode action");

    // 6. Sign the sighash with A and B via the desktop mnemonic-signing path.
    let sighash = signing::compute_sighash(SEQ_NO, &action_hex).expect("compute SPS-65 sighash");
    let path_a = format!("{PATH_PREFIX}/0");
    let path_b = format!("{PATH_PREFIX}/1");
    let sig_a =
        signing::sign_with_mnemonic_path(MNEMONIC, PASSPHRASE, &path_a, &sighash.sighash_hex)
            .expect("sign sighash with A");
    let sig_b =
        signing::sign_with_mnemonic_path(MNEMONIC, PASSPHRASE, &path_b, &sighash.sighash_hex)
            .expect("sign sighash with B");
    assert_eq!(sig_a.public_key_hex, a_hex, "A derivation/signing mismatch");
    assert_eq!(sig_b.public_key_hex, b_hex, "B derivation/signing mismatch");

    // 7. Sanity check the desktop's threshold verifier accepts both signatures.
    let verify = signing::verify_threshold(
        &[a_hex.clone(), b_hex.clone()],
        2,
        &[sig_a.signature_hex.clone(), sig_b.signature_hex.clone()],
        &sighash.sighash_hex,
    )
    .expect("verify_threshold");
    assert!(verify.valid, "threshold-of-2 must validate both signatures");

    // 8. Build the SSZ-encoded SignedPayload via the desktop broadcast helper.
    //    canonical_pubkeys_hex MUST match the JSON's order so signer indices
    //    line up with the on-chain ThresholdConfig.
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
    let sighash_bytes: [u8; 32] = hex::decode(&sighash.sighash_hex)
        .expect("sighash_hex decode")
        .try_into()
        .expect("sighash_hex is 32 bytes");
    let payload_bytes = broadcast_tx::build_signed_payload_bytes(
        SEQ_NO,
        &action_hex,
        &proposal_sigs,
        &json_keys,
        &sighash_bytes,
    )
    .expect("build_signed_payload_bytes");

    // 9. Derive the commit address, fund it from regtest, and mine the funding
    //    transaction into a confirmed block.
    let secp = Secp256k1::new();
    let envelope_keypair = UntweakedKeypair::new(&secp, &mut OsRng);
    let (commit_address, reveal_script, taproot_spend_info) = broadcast_tx::derive_commit_address(
        &envelope_keypair,
        &payload_bytes,
        bitcoin::Network::Regtest,
    )
    .expect("derive commit address");

    let commit_txid_str = harness
        .bitcoind
        .client
        .send_to_address(&commit_address, Amount::from_sat(10_000))
        .expect("send commit funding tx")
        .0;
    let commit_txid: bitcoin::Txid = commit_txid_str.parse().expect("parse commit txid");

    // Fetch the commit raw tx BEFORE mining (it's still in the mempool here).
    // Without `-txindex` enabled on bitcoind, `getrawtransaction` cannot find
    // a confirmed transaction by txid alone — see `AsmTestHarness::create_funding_utxo`
    // which uses the same fetch-then-mine ordering.
    let commit_tx = harness
        .client
        .get_raw_transaction_verbosity_zero(&commit_txid)
        .await
        .expect("fetch commit raw tx from mempool")
        .0;

    let _ = harness
        .mine_block(None)
        .await
        .expect("mine commit confirmation block");

    // 10. Build and broadcast the reveal transaction.
    let action_strata =
        MultisigAction::from_ssz_bytes(&hex::decode(&action_hex).expect("hex decode action"))
            .expect("decode strata action");
    let reveal_tx = broadcast_tx::build_reveal_tx(
        &envelope_keypair,
        &reveal_script,
        &taproot_spend_info,
        &commit_tx,
        &commit_address.script_pubkey(),
        &action_strata,
        harness.asm_params.magic,
        bitcoin::Network::Regtest,
        1_000,
    )
    .expect("build reveal tx");

    let _ = harness
        .submit_and_mine_tx(&reveal_tx)
        .await
        .expect("submit and mine reveal tx");

    // 11. Mine `confirmation_depth + 1` additional blocks so the queued admin
    //     update has time to reach its enactment height inside the ASM STF.
    let _ = harness
        .mine_blocks(confirmation_depth as usize + 1)
        .await
        .expect("mine enough blocks for enactment");

    // 12. Read the latest ASM state and assert that D is now in the canonical
    //     signer set, A and B are still present, and `last_seqno` advanced.
    let (_, asm_state) = harness
        .get_latest_asm_state()
        .expect("get_latest_asm_state")
        .expect("ASM state must be present");
    let admin = decode_admin_state(&asm_state).expect("admin section must be present");
    let authority = admin
        .authority(Role::StrataAdministrator)
        .expect("strata administrator authority present");
    let canonical_hex: Vec<String> = authority
        .config()
        .keys()
        .iter()
        .map(|k: &CompressedPublicKey| hex::encode(k.serialize()))
        .collect();
    assert!(
        canonical_hex.contains(&d_hex),
        "newly added signer D must be in the enacted canonical key set: {canonical_hex:?}"
    );
    assert!(
        canonical_hex.contains(&a_hex) && canonical_hex.contains(&b_hex),
        "original signers A and B must remain in canonical set: {canonical_hex:?}"
    );
    assert_eq!(
        authority.last_seqno(),
        SEQ_NO,
        "last_seqno must advance after enactment"
    );
}

/// Inline mirror of `desktop_app::infrastructure::asm_role_membership::decode_admin_state`.
/// Kept private to this test so the production module surface stays unchanged.
fn decode_admin_state(asm_state: &AsmState) -> Option<AdministrationSubprotoState> {
    asm_state
        .state()
        .find_section(AdministrationSubprotocol::ID)
        .and_then(|section| section.try_to_state::<AdministrationSubprotocol>().ok())
}
