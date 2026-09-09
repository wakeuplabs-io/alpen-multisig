//! Protocol regression e2e for the Safe Harbour address update (SPS-50 tx type 14).
//!
//! Three paths, and the third is why this file exists: once the safe harbour is activated the
//! bridge **refuses** an address change and the subprotocol discards the refusal, so the action is
//! accepted on chain, consumes its sequence number, leaves the queue — and changes nothing. Nothing
//! upstream covers that, and it is the shape every rotation attempted after an incident would take.
//!
//! Administrator and council keys are disjoint on purpose: the rotation is authorized by the
//! administrator and the Defcon that freezes the harbour by the council, and mixing them would hide
//! a mistake in either gate.

use std::num::NonZero;
use std::process::Command;

use alpen_multisig_e2e_tests::fixtures::{decode_administration_subproto, decode_bridge_subproto};
use alpen_multisig_e2e_tests::test_harness::{AsmTestHarness, AsmTestHarnessBuilder};
use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
use bitcoind_async_client::traits::Reader;
use ssz::Encode;
use strata_asm_params::{AdministrationInitConfig, ConfirmationDepths, Role, UpdateTxType};
use strata_asm_proto_admin::AdministrationSubprotoState;
use strata_asm_txs_admin::actions::updates::Defcon1Update;
use strata_asm_txs_admin::actions::{CancelAction, MultisigAction, UpdateAction};
use strata_asm_txs_admin::parser::SignedPayload;
use strata_asm_txs_admin::test_utils::create_signature_set;
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::ThresholdConfig;

use desktop_app::domain::action::{Action, SafeHarbourDescriptor};
use desktop_app::infrastructure::action_codec;

const HARBOUR_UPDATE_DEPTH: u16 = 5;
/// Deliberately different from the safe harbour depth: a per-authority mapping would resolve both
/// administrator actions to one number, and the queue assertions would still pass.
const ADMIN_UPDATE_DEPTH: u16 = 9;
const ROTATION_SEQNO: u64 = 7;
const CANCEL_SEQNO: u64 = 8;
const DEFCON_SEQNO: u64 = 1;
const FIRST_UPDATE_ID: u32 = 0;

/// The destination the harness starts on is **not** fixed: `SafeHarbourAddress::arbitrary` derives
/// a P2TR descriptor from a fresh keypair, so every run begins somewhere different. Each path reads
/// it and compares against what it read — which is the stronger assertion anyway, since a hardcoded
/// value would also pass on a chain that never applied anything.
///
/// A different destination: the taproot output for the x-only key `0x0202…02`.
const NEW_DESCRIPTOR_HEX: &str =
    "040202020202020202020202020202020202020202020202020202020202020202";

struct HarbourFixture {
    admin_keys: Vec<SecretKey>,
    council_keys: Vec<SecretKey>,
    admin_config: AdministrationInitConfig,
}

impl HarbourFixture {
    fn new() -> Self {
        let admin_keys = vec![fixed_key(1), fixed_key(2)];
        let council_keys = vec![fixed_key(3), fixed_key(4)];
        let unrelated_config = threshold_config(&[fixed_key(7)], 1);

        let admin_config = AdministrationInitConfig {
            strata_administrator: threshold_config(&admin_keys, 2),
            strata_sequencer_manager: unrelated_config.clone(),
            alpen_administrator: unrelated_config,
            strata_security_council: threshold_config(&council_keys, 2),
            confirmation_depths: ConfirmationDepths {
                strata_admin_multisig_update: ADMIN_UPDATE_DEPTH,
                strata_seq_manager_multisig_update: HARBOUR_UPDATE_DEPTH,
                alpen_admin_multisig_update: HARBOUR_UPDATE_DEPTH,
                operator_update: HARBOUR_UPDATE_DEPTH,
                sequencer_update: HARBOUR_UPDATE_DEPTH,
                ol_stf_vk_update: HARBOUR_UPDATE_DEPTH,
                asm_stf_vk_update: HARBOUR_UPDATE_DEPTH,
                ee_stf_vk_update: HARBOUR_UPDATE_DEPTH,
                strata_security_council_multisig_update: HARBOUR_UPDATE_DEPTH,
                defcon3: HARBOUR_UPDATE_DEPTH,
                safe_harbour_address_update: HARBOUR_UPDATE_DEPTH,
            },
            max_seqno_gap: NonZero::new(10).expect("non-zero sequence gap"),
        };

        Self {
            admin_keys,
            council_keys,
            admin_config,
        }
    }

    /// Composed through the desktop codec, like the council rotation: that wire mapping is part of
    /// what is under test, and it keeps this crate free of the protocol crates that build the
    /// payload.
    fn rotation_action(&self) -> anyhow::Result<MultisigAction> {
        let destination = SafeHarbourDescriptor::from_hex(NEW_DESCRIPTOR_HEX)?;
        let encoded = action_codec::encode_hex(&Action::SafeHarbourAddressUpdate(destination))?;
        ssz::Decode::from_ssz_bytes(&hex::decode(encoded)?).map_err(|e| anyhow::anyhow!("{e:?}"))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_safe_harbour_address_enacts_at_its_depth() {
    if skip_without_bitcoind("e2e_safe_harbour_address_enacts_at_its_depth") {
        return;
    }
    run_enacted_rotation()
        .await
        .expect("the destination must change at the configured depth");
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_safe_harbour_address_cancelled_never_changes_the_destination() {
    if skip_without_bitcoind("e2e_safe_harbour_address_cancelled_never_changes_the_destination") {
        return;
    }
    run_cancelled_rotation()
        .await
        .expect("a cancelled rotation must leave the destination untouched");
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_safe_harbour_address_is_swallowed_once_the_harbour_is_active() {
    if skip_without_bitcoind("e2e_safe_harbour_address_is_swallowed_once_the_harbour_is_active") {
        return;
    }
    run_swallowed_rotation()
        .await
        .expect("an activated harbour must freeze the destination while still accepting the tx");
}

async fn run_enacted_rotation() -> anyhow::Result<()> {
    let fixture = HarbourFixture::new();
    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(fixture.admin_config.clone())
        .build()
        .await?;

    let initial = administration_state(&harness)?;
    assert_fixture_depths(&initial)?;
    let initial_destination = safe_harbour_descriptor_hex(&harness)?;
    anyhow::ensure!(
        initial_destination != NEW_DESCRIPTOR_HEX,
        "the fixture destination must differ from the one the rotation installs"
    );
    anyhow::ensure!(
        !safe_harbour_activated(&harness)?,
        "the harness must start with the safe harbour down"
    );
    let (admin_config, initial_admin_seqno) =
        authority_snapshot(&initial, Role::StrataAdministrator)?;

    let rotation = fixture.rotation_action()?;
    let admin_indices = signer_indices(&admin_config, &fixture.admin_keys)?;
    let reveal_height = submit_action(
        &harness,
        &rotation,
        ROTATION_SEQNO,
        &fixture.admin_keys,
        &admin_indices,
    )
    .await?;
    let activation_height = reveal_height + u64::from(HARBOUR_UPDATE_DEPTH);

    // Queued, and nothing moved yet — the seqno is spent at acceptance, the destination is not.
    anyhow::ensure!(
        queued_matches(&harness, &rotation)? == 1,
        "the rotation must sit in the admin queue before its activation height"
    );
    anyhow::ensure!(
        safe_harbour_descriptor_hex(&harness)? == initial_destination,
        "the destination must not change while the update is queued"
    );
    anyhow::ensure!(
        authority_snapshot(&administration_state(&harness)?, Role::StrataAdministrator)?.1
            == ROTATION_SEQNO,
        "administrator sequence must advance to {ROTATION_SEQNO} at acceptance"
    );

    // One block short is still short: the boundary is `activation_height <= tip`.
    mine_to(&harness, activation_height - 1).await?;
    anyhow::ensure!(
        safe_harbour_descriptor_hex(&harness)? == initial_destination,
        "the destination must not change one block before activation"
    );

    mine_to(&harness, activation_height).await?;
    anyhow::ensure!(
        safe_harbour_descriptor_hex(&harness)? == NEW_DESCRIPTOR_HEX,
        "the destination must equal the proposed one at exact activation"
    );
    anyhow::ensure!(
        administration_state(&harness)?.queued().is_empty(),
        "the update must leave the queue at activation"
    );
    // AC 7a: a rotation moves the destination and never touches activation.
    anyhow::ensure!(
        !safe_harbour_activated(&harness)?,
        "a rotation must not activate the safe harbour"
    );
    anyhow::ensure!(
        initial_admin_seqno < ROTATION_SEQNO,
        "the fixture must start below the proposal's sequence number"
    );
    Ok(())
}

async fn run_cancelled_rotation() -> anyhow::Result<()> {
    let fixture = HarbourFixture::new();
    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(fixture.admin_config.clone())
        .build()
        .await?;

    let initial = administration_state(&harness)?;
    let initial_destination = safe_harbour_descriptor_hex(&harness)?;
    let (admin_config, _) = authority_snapshot(&initial, Role::StrataAdministrator)?;
    let admin_indices = signer_indices(&admin_config, &fixture.admin_keys)?;

    let rotation = fixture.rotation_action()?;
    let reveal_height = submit_action(
        &harness,
        &rotation,
        ROTATION_SEQNO,
        &fixture.admin_keys,
        &admin_indices,
    )
    .await?;
    let activation_height = reveal_height + u64::from(HARBOUR_UPDATE_DEPTH);

    let queued_state = administration_state(&harness)?;
    let MultisigAction::Update(expected_update) = &rotation else {
        anyhow::bail!("a rotation is an update, not a cancel");
    };
    let queued = queued_state
        .queued()
        .iter()
        .find(|entry| entry.action() == expected_update)
        .ok_or_else(|| anyhow::anyhow!("the rotation must be queued before it can be cancelled"))?;
    anyhow::ensure!(
        *queued.id() == FIRST_UPDATE_ID,
        "a fresh harness gives the first update id 0"
    );

    let cancel = MultisigAction::Cancel(CancelAction::new(*queued.id(), queued.action().clone()));
    let cancel_height = submit_action(
        &harness,
        &cancel,
        CANCEL_SEQNO,
        &fixture.admin_keys,
        &admin_indices,
    )
    .await?;
    anyhow::ensure!(
        cancel_height < activation_height,
        "the cancel must land inside the window it is meant to close"
    );

    // Past the height the rotation would have matured at: the destination stays put because the
    // entry is gone, not because it has not come due yet.
    mine_to(&harness, activation_height + 1).await?;
    anyhow::ensure!(
        administration_state(&harness)?.queued().is_empty(),
        "the cancel must drain the queue entry"
    );
    anyhow::ensure!(
        safe_harbour_descriptor_hex(&harness)? == initial_destination,
        "a cancelled rotation must never install its destination"
    );
    Ok(())
}

/// Constraint 1, against a real chain. `SafeHarbour::update_address` returns `false` while the
/// harbour is activated and the bridge subprotocol drops that boolean — so everything else about
/// this rotation succeeds and the destination does not move.
async fn run_swallowed_rotation() -> anyhow::Result<()> {
    let fixture = HarbourFixture::new();
    let harness = AsmTestHarnessBuilder::default()
        .with_admin_config(fixture.admin_config.clone())
        .build()
        .await?;

    let initial = administration_state(&harness)?;
    let initial_destination = safe_harbour_descriptor_hex(&harness)?;
    let (admin_config, _) = authority_snapshot(&initial, Role::StrataAdministrator)?;
    let (council_config, _) = authority_snapshot(&initial, Role::StrataSecurityCouncil)?;

    // Defcon 1 has depth 0: it activates the harbour in its own reveal block. The rotation has to
    // be submitted after that, or it would be queued against a deactivated harbour and this test
    // would prove the enacted path instead.
    let defcon = MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update));
    let council_indices = signer_indices(&council_config, &fixture.council_keys)?;
    submit_action(
        &harness,
        &defcon,
        DEFCON_SEQNO,
        &fixture.council_keys,
        &council_indices,
    )
    .await?;
    anyhow::ensure!(
        safe_harbour_activated(&harness)?,
        "Defcon 1 must activate the safe harbour in its own block"
    );

    let rotation = fixture.rotation_action()?;
    let admin_indices = signer_indices(&admin_config, &fixture.admin_keys)?;
    let reveal_height = submit_action(
        &harness,
        &rotation,
        ROTATION_SEQNO,
        &fixture.admin_keys,
        &admin_indices,
    )
    .await?;
    let activation_height = reveal_height + u64::from(HARBOUR_UPDATE_DEPTH);

    // Accepted, exactly as it would be with the harbour down: the signature check passed and the
    // sequence number is spent.
    anyhow::ensure!(
        authority_snapshot(&administration_state(&harness)?, Role::StrataAdministrator)?.1
            == ROTATION_SEQNO,
        "the rotation must be accepted and consume the administrator's sequence number"
    );
    anyhow::ensure!(
        queued_matches(&harness, &rotation)? == 1,
        "the rotation must be queued like any other"
    );

    mine_to(&harness, activation_height).await?;

    anyhow::ensure!(
        administration_state(&harness)?.queued().is_empty(),
        "the queue entry must drain at activation, as it would for a rotation that applied"
    );
    anyhow::ensure!(
        safe_harbour_descriptor_hex(&harness)? == initial_destination,
        "the destination is frozen once the harbour is active: the rotation must change nothing"
    );
    anyhow::ensure!(
        safe_harbour_activated(&harness)?,
        "the harbour must still be active after the swallowed rotation"
    );
    Ok(())
}

fn skip_without_bitcoind(test_name: &str) -> bool {
    if Command::new("bitcoind").arg("--version").output().is_err() {
        eprintln!("Skipping {test_name}: bitcoind is not available in PATH");
        return true;
    }
    false
}

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

fn queued_matches(harness: &AsmTestHarness, action: &MultisigAction) -> anyhow::Result<usize> {
    let MultisigAction::Update(expected) = action else {
        anyhow::bail!("only an update is ever queued");
    };
    Ok(administration_state(harness)?
        .queued()
        .iter()
        .filter(|entry| entry.action() == expected)
        .count())
}

/// The BOSD wire form of the installed destination — the same bytes the signing message renders.
fn safe_harbour_descriptor_hex(harness: &AsmTestHarness) -> anyhow::Result<String> {
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let bridge = decode_bridge_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("bridge section must be present"))?;
    Ok(hex::encode(
        bridge.safe_harbour().address().as_descriptor().to_bytes(),
    ))
}

fn safe_harbour_activated(harness: &AsmTestHarness) -> anyhow::Result<bool> {
    let (_, asm_state) = harness
        .get_latest_asm_state()?
        .ok_or_else(|| anyhow::anyhow!("ASM state must be present"))?;
    let bridge = decode_bridge_subproto(&asm_state)
        .ok_or_else(|| anyhow::anyhow!("bridge section must be present"))?;
    Ok(bridge.safe_harbour().is_activated())
}

fn assert_fixture_depths(state: &AdministrationSubprotoState) -> anyhow::Result<()> {
    anyhow::ensure!(
        state.confirmation_depth(UpdateTxType::SafeHarbourAddressUpdate)
            == Some(HARBOUR_UPDATE_DEPTH),
        "tx14 must use the safe harbour depth"
    );
    anyhow::ensure!(
        state.confirmation_depth(UpdateTxType::StrataAdminMultisigUpdate)
            == Some(ADMIN_UPDATE_DEPTH),
        "the administrator's own update must keep a distinct depth"
    );
    Ok(())
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
        .map(|secret_key| {
            let public_key = compressed_key(secret_key);
            config
                .keys()
                .iter()
                .position(|candidate| candidate == &public_key)
                .ok_or_else(|| anyhow::anyhow!("signer is absent from live authority config"))?
                .try_into()
                .map_err(|_| anyhow::anyhow!("signer index exceeds u8"))
        })
        .collect()
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
