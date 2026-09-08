//! Protocol regression e2e for Strata Security Council signer rotation.
//!
//! The fixture deliberately keeps administrator and council keys disjoint. Actions travel through
//! real SPS-50 envelopes and a live regtest ASM; only the rotation is composed through the desktop
//! codec because that wire mapping is part of the behavior under test.

use std::num::{NonZero, NonZeroU8};
use std::process::Command;

use alpen_multisig_e2e_tests::fixtures::{decode_administration_subproto, decode_bridge_subproto};
use alpen_multisig_e2e_tests::test_harness::{AsmTestHarness, AsmTestHarnessBuilder};
use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
use bitcoind_async_client::traits::Reader;
use ssz::{Decode, Encode};
use strata_asm_params::{AdministrationInitConfig, ConfirmationDepths, Role};
use strata_asm_proto_admin::AdministrationSubprotoState;
use strata_asm_txs_admin::actions::updates::Defcon1Update;
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_asm_txs_admin::parser::SignedPayload;
use strata_asm_txs_admin::test_utils::create_signature_set;
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::ThresholdConfig;

use desktop_app::domain::action::{Action, CompressedPubKey, MultisigUpdate};
use desktop_app::domain::authority::Authority;
use desktop_app::infrastructure::action_codec;

const COUNCIL_UPDATE_DEPTH: u16 = 5;
const ADMIN_UPDATE_DEPTH: u16 = 9;
const ROTATION_SEQNO: u64 = 7;
const COUNCIL_ACTION_SEQNO: u64 = 1;
const FIRST_UPDATE_ID: u32 = 0;

struct RotationFixture {
    admin_keys: Vec<SecretKey>,
    council_keys: Vec<SecretKey>,
    replacement_key: SecretKey,
    initial_council_config: ThresholdConfig,
    rotated_council_config: ThresholdConfig,
    admin_config: AdministrationInitConfig,
}

impl RotationFixture {
    fn new() -> Self {
        let admin_keys = vec![fixed_key(1), fixed_key(2)];
        let council_keys = vec![fixed_key(3), fixed_key(4), fixed_key(5)];
        let replacement_key = fixed_key(6);
        let initial_council_config = threshold_config(&council_keys, 2);
        let rotated_council_config =
            threshold_config(&[council_keys[0], council_keys[2], replacement_key], 2);
        let unrelated_config = threshold_config(&[fixed_key(7)], 1);

        let admin_config = AdministrationInitConfig {
            strata_administrator: threshold_config(&admin_keys, 2),
            strata_sequencer_manager: unrelated_config.clone(),
            alpen_administrator: unrelated_config.clone(),
            strata_security_council: initial_council_config.clone(),
            confirmation_depths: ConfirmationDepths {
                strata_admin_multisig_update: ADMIN_UPDATE_DEPTH,
                strata_seq_manager_multisig_update: COUNCIL_UPDATE_DEPTH,
                alpen_admin_multisig_update: COUNCIL_UPDATE_DEPTH,
                operator_update: COUNCIL_UPDATE_DEPTH,
                sequencer_update: COUNCIL_UPDATE_DEPTH,
                ol_stf_vk_update: COUNCIL_UPDATE_DEPTH,
                asm_stf_vk_update: COUNCIL_UPDATE_DEPTH,
                ee_stf_vk_update: COUNCIL_UPDATE_DEPTH,
                strata_security_council_multisig_update: COUNCIL_UPDATE_DEPTH,
                defcon3: COUNCIL_UPDATE_DEPTH,
                safe_harbour_address_update: COUNCIL_UPDATE_DEPTH,
            },
            max_seqno_gap: NonZero::new(10).expect("non-zero sequence gap"),
        };

        Self {
            admin_keys,
            council_keys,
            replacement_key,
            initial_council_config,
            rotated_council_config,
            admin_config,
        }
    }

    fn rotation_action(&self) -> anyhow::Result<MultisigAction> {
        let removed = compressed_key(&self.council_keys[1]);
        let added = compressed_key(&self.replacement_key);
        let action = Action::MultisigUpdate(MultisigUpdate {
            role: Authority::SecurityCouncil,
            add_keys: vec![domain_key(&added)],
            remove_keys: vec![domain_key(&removed)],
            new_threshold: NonZeroU8::new(2).expect("non-zero threshold"),
        });
        let encoded = action_codec::encode_hex(&action)?;
        Ok(MultisigAction::from_ssz_bytes(&hex::decode(encoded)?)?)
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_council_rotation_enacts_and_changes_who_can_trigger_defcon() {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!(
            "Skipping e2e_council_rotation_enacts_and_changes_who_can_trigger_defcon: \
             bitcoind is not available in PATH"
        );
        return;
    }

    run_enacted_rotation()
        .await
        .expect("council rotation must enact at its configured depth");
}

async fn run_enacted_rotation() -> anyhow::Result<()> {
    let fixture = RotationFixture::new();
    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(fixture.admin_config.clone())
        .build()
        .await?;
    let initial = administration_state(&harness)?;
    let (initial_admin_config, initial_admin_seqno) =
        authority_snapshot(&initial, Role::StrataAdministrator)?;
    let (initial_council_config, initial_council_seqno) =
        authority_snapshot(&initial, Role::StrataSecurityCouncil)?;
    anyhow::ensure!(
        initial_council_config == fixture.initial_council_config,
        "fixture council config must be installed verbatim"
    );

    let rotation = fixture.rotation_action()?;
    let admin_indices = signer_indices(&initial_admin_config, &fixture.admin_keys)?;
    let reveal_height = submit_action(
        &harness,
        &rotation,
        ROTATION_SEQNO,
        &fixture.admin_keys,
        &admin_indices,
    )
    .await?;
    let activation_height = reveal_height + u64::from(COUNCIL_UPDATE_DEPTH);

    assert_rotation_queued(
        &harness,
        &rotation,
        activation_height,
        &fixture.initial_council_config,
    )?;
    let queued = administration_state(&harness)?;
    anyhow::ensure!(
        authority_snapshot(&queued, Role::StrataAdministrator)?.1 == ROTATION_SEQNO,
        "administrator sequence must advance to {ROTATION_SEQNO}"
    );
    anyhow::ensure!(
        authority_snapshot(&queued, Role::StrataSecurityCouncil)?.1 == initial_council_seqno,
        "council sequence must not advance for an administrator-authorized rotation"
    );

    mine_to(&harness, activation_height - 1).await?;
    assert_rotation_queued(
        &harness,
        &rotation,
        activation_height,
        &fixture.initial_council_config,
    )?;
    mine_to(&harness, activation_height).await?;

    let enacted = administration_state(&harness)?;
    anyhow::ensure!(
        enacted.queued().is_empty(),
        "rotation must leave the queue at exact activation"
    );
    anyhow::ensure!(
        authority_snapshot(&enacted, Role::StrataSecurityCouncil)?.0
            == fixture.rotated_council_config,
        "council config must contain C0, C2 and C3 at threshold 2"
    );
    let (enacted_admin_config, enacted_admin_seqno) =
        authority_snapshot(&enacted, Role::StrataAdministrator)?;
    anyhow::ensure!(
        enacted_admin_config == initial_admin_config,
        "rotation must not change administrator membership"
    );
    anyhow::ensure!(
        enacted_admin_seqno == ROTATION_SEQNO && initial_admin_seqno < ROTATION_SEQNO,
        "administrator sequence must remain {ROTATION_SEQNO} after enactment"
    );
    anyhow::ensure!(
        authority_snapshot(&enacted, Role::StrataSecurityCouncil)?.1 == initial_council_seqno,
        "council sequence must remain unchanged after enactment"
    );

    prove_rotated_membership_controls_defcon(&harness, &fixture, initial_council_seqno).await?;

    Ok(())
}

async fn prove_rotated_membership_controls_defcon(
    harness: &AsmTestHarness,
    fixture: &RotationFixture,
    initial_council_seqno: u64,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        initial_council_seqno == 0,
        "fresh council sequence must begin at zero"
    );
    anyhow::ensure!(
        !bridge_safe_harbour_activated(harness)?,
        "safe harbour must start deactivated"
    );

    let live = administration_state(harness)?;
    let (live_council, _) = authority_snapshot(&live, Role::StrataSecurityCouncil)?;
    let c0_index = signer_index(&live_council, &fixture.council_keys[0])?;
    let c3_index = signer_index(&live_council, &fixture.replacement_key)?;
    let mut live_indices = vec![c0_index, c3_index];
    live_indices.sort_unstable();

    let defcon = MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update));

    // Put removed C1's signature in the canonical slot now occupied by C3. The envelope and
    // SignatureSet are structurally valid, so rejection occurs inside ASM threshold verification.
    let mut invalid_old_quorum = vec![fixed_key(8); live_council.len()];
    invalid_old_quorum[c0_index as usize] = fixture.council_keys[0];
    invalid_old_quorum[c3_index as usize] = fixture.council_keys[1];
    let _ = submit_action(
        harness,
        &defcon,
        COUNCIL_ACTION_SEQNO,
        &invalid_old_quorum,
        &live_indices,
    )
    .await?;

    let after_rejected = administration_state(harness)?;
    anyhow::ensure!(
        !bridge_safe_harbour_activated(harness)?,
        "removed-signer quorum must not activate safe harbour"
    );
    anyhow::ensure!(
        !after_rejected
            .queued()
            .iter()
            .any(|queued| matches!(queued.action(), UpdateAction::Defcon1(_))),
        "rejected Defcon 1 must not appear in the queue"
    );
    anyhow::ensure!(
        authority_snapshot(&after_rejected, Role::StrataSecurityCouncil)?.1
            == initial_council_seqno,
        "rejected old quorum must not consume council sequence 1"
    );

    let mut valid_new_quorum = vec![fixed_key(8); live_council.len()];
    valid_new_quorum[c0_index as usize] = fixture.council_keys[0];
    valid_new_quorum[c3_index as usize] = fixture.replacement_key;
    let _ = submit_action(
        harness,
        &defcon,
        COUNCIL_ACTION_SEQNO,
        &valid_new_quorum,
        &live_indices,
    )
    .await?;

    anyhow::ensure!(
        bridge_safe_harbour_activated(harness)?,
        "valid new quorum must activate safe harbour at the same sequence"
    );
    let after_accepted = administration_state(harness)?;
    anyhow::ensure!(
        authority_snapshot(&after_accepted, Role::StrataSecurityCouncil)?.1 == COUNCIL_ACTION_SEQNO,
        "valid new quorum must advance council sequence to 1"
    );

    Ok(())
}

fn assert_rotation_queued(
    harness: &AsmTestHarness,
    rotation: &MultisigAction,
    activation_height: u64,
    expected_council: &ThresholdConfig,
) -> anyhow::Result<()> {
    let state = administration_state(harness)?;
    let expected_action = match rotation {
        MultisigAction::Update(update) => update,
        MultisigAction::Cancel(_) => anyhow::bail!("rotation must be an update"),
    };
    let matches: Vec<_> = state
        .queued()
        .iter()
        .filter(|queued| queued.action() == expected_action)
        .collect();
    anyhow::ensure!(
        matches.len() == 1,
        "exactly one matching council rotation must be queued"
    );
    anyhow::ensure!(
        *matches[0].id() == FIRST_UPDATE_ID,
        "administrator seqno {ROTATION_SEQNO} must be distinct from global UpdateId 0"
    );
    anyhow::ensure!(
        u64::from(matches[0].activation_height()) == activation_height,
        "queued activation height must equal measured reveal height plus depth"
    );
    anyhow::ensure!(
        authority_snapshot(&state, Role::StrataSecurityCouncil)?.0 == *expected_council,
        "council config must remain unchanged before activation"
    );
    Ok(())
}

/// Submit a real admin envelope and return the measured reveal block height.
///
/// Key material is separate from signature indices because membership-effect coverage deliberately
/// signs a live canonical slot with a removed key, allowing the malformed quorum to reach ASM.
async fn submit_action(
    harness: &AsmTestHarness,
    action: &MultisigAction,
    seqno: u64,
    signer_keys: &[SecretKey],
    signer_indices: &[u8],
) -> anyhow::Result<u64> {
    let signatures = create_signature_set(signer_keys, signer_indices, action, seqno);
    let payload = SignedPayload::new(seqno, action.clone(), signatures).as_ssz_bytes();
    let reveal = harness.build_envelope_tx(action.tag(), payload).await?;
    let block_hash = harness.submit_and_mine_tx(&reveal).await?;
    Ok(harness.client.get_block_height(&block_hash).await?)
}

async fn mine_to(harness: &AsmTestHarness, target_height: u64) -> anyhow::Result<()> {
    let tip = harness.get_chain_tip().await?;
    let _ = harness
        .mine_blocks(target_height.saturating_sub(tip) as usize)
        .await?;
    anyhow::ensure!(
        harness.get_chain_tip().await? == target_height.max(tip),
        "mining must end at the requested measured height"
    );
    Ok(())
}

fn administration_state(harness: &AsmTestHarness) -> anyhow::Result<AdministrationSubprotoState> {
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    decode_administration_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("administration section must be present"))
}

fn authority_snapshot(
    state: &AdministrationSubprotoState,
    role: Role,
) -> anyhow::Result<(ThresholdConfig, u64)> {
    let authority = state
        .authority(role)
        .ok_or_else(|| anyhow::anyhow!("{role:?} authority must be present"))?;
    Ok((authority.config().clone(), authority.last_seqno()))
}

fn signer_indices(config: &ThresholdConfig, signer_keys: &[SecretKey]) -> anyhow::Result<Vec<u8>> {
    signer_keys
        .iter()
        .map(|secret_key| signer_index(config, secret_key))
        .collect()
}

fn signer_index(config: &ThresholdConfig, secret_key: &SecretKey) -> anyhow::Result<u8> {
    let public_key = compressed_key(secret_key);
    config
        .keys()
        .iter()
        .position(|candidate| candidate == &public_key)
        .ok_or_else(|| anyhow::anyhow!("signer is absent from live authority config"))?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signer index exceeds u8"))
}

fn bridge_safe_harbour_activated(harness: &AsmTestHarness) -> anyhow::Result<bool> {
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let bridge = decode_bridge_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("bridge section must be present"))?;
    Ok(bridge.safe_harbour().is_activated())
}

fn fixed_key(byte: u8) -> SecretKey {
    SecretKey::from_slice(&[byte; 32]).expect("fixed test key must be valid")
}

fn threshold_config(keys: &[SecretKey], threshold: u8) -> ThresholdConfig {
    ThresholdConfig::try_new(
        keys.iter().map(compressed_key).collect(),
        NonZero::new(threshold).expect("non-zero threshold"),
    )
    .expect("valid threshold config")
}

fn compressed_key(secret_key: &SecretKey) -> CompressedPublicKey {
    CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, secret_key))
}

fn domain_key(key: &CompressedPublicKey) -> CompressedPubKey {
    CompressedPubKey::new(key.serialize())
}
