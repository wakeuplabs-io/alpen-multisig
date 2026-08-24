//! Proposal management — application layer entry point for the desktop app.
//!
//! Public API mirrors the PRD's `MultisigBackend` trait semantics:
//! - `create_update_action(action_hex, seq_no, signature)` — propose + first signature
//! - `approve_action(action_id, signature)` — add approval signature
//! - `create_cancel_action(target_action_id, action_hex, seq_no, signature)` — propose a cancel
//! - `get_update_action(action_id)` — fetch proposal detail
//!
//! Authority is implicit — bound to the authenticated session, not passed per call.
//! Signing and action encoding happen before reaching this layer.

use bitcoin::{Network, ScriptBuf};
use ssz::Decode;
use strata_asm_txs_admin::actions::MultisigAction;
use strata_l1_txfmt::MagicBytes;

use crate::application::commit_funding::CommitFunding;
use crate::application::orchestrator_client::{
    ApproveActionRequest, CreateCancelProposalRequest, CreateProposalRequest, OrchestratorClient,
    OrchestratorError, ReportBroadcastProgressRequest, TransitionProposalRequest,
};
use crate::application::pending_reveals::PendingReveals;
use crate::application::tx_broadcaster::TxBroadcaster;
use crate::domain::proposal::{Proposal, Signature};
use crate::infrastructure::asm_role_membership;
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;
use crate::infrastructure::broadcast_tx;

/// Errors that can occur during proposal operations.
#[derive(Debug, thiserror::Error)]
pub enum ProposalError {
    #[error("Orchestrator error: {0}")]
    Orchestrator(#[from] OrchestratorError),
}

/// Errors that can occur during direct broadcast from Tauri.
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("failed to fetch proposal: {0}")]
    ProposalFetch(#[from] OrchestratorError),
    #[error("broadcast setup error: {0}")]
    Setup(String),
    #[error("bitcoin RPC error: {0}")]
    BitcoinRpc(String),
    #[error("confirmation timeout for txid {txid}")]
    Timeout { txid: String },
    #[error("no pending reveal found for action_id: {action_id}")]
    NoPendingReveal { action_id: String },
    /// All broadcasters (Electrum + node) failed. Carries the raw tx hexes for manual
    /// copy-and-broadcast as an escape hatch (spec §8.3 M3).
    #[error("all broadcasters failed: {errors:?}")]
    AllBroadcastersFailed {
        commit_tx_hex: String,
        reveal_tx_hex: String,
        errors: Vec<(String, String)>,
    },
}

use crate::domain::fee_constants::{COMMIT_DUST_SATS, REVEAL_TX_VBYTES};
use crate::domain::fee_rate::FeeRate;
use crate::infrastructure::admin_wallet::EnvelopeKeyCache;
use crate::infrastructure::hw_wallet::hw_psbt_signer::HwDeviceType;

/// Assemble commit/reveal artifacts for an approved proposal without submitting to the network.
///
/// Returns `(commit_address, commit_amount_sats, estimated_fee_sats)`.
#[allow(clippy::too_many_arguments)]
pub async fn prepare_broadcast_bundle(
    client: &dyn OrchestratorClient,
    asm_rpc_url: &str,
    network: Network,
    action_id: &str,
    fee_rate: FeeRate,
    envelope_cache: &EnvelopeKeyCache,
    hw_device: Option<HwDeviceType>,
) -> Result<(String, u64, u64), BroadcastError> {
    let proposal = client.get_proposal(action_id).await?;

    if proposal.status != "approved" {
        return Err(BroadcastError::Setup(format!(
            "proposal must be in 'approved' state to broadcast (current: {})",
            proposal.status
        )));
    }

    let canonical_keys =
        asm_role_membership::ordered_keys_for_authority(asm_rpc_url, proposal.authority)
            .await
            .map_err(BroadcastError::Setup)?;

    let sighash = broadcast_tx::compute_sighash(proposal.seq_no, &proposal.action_hex)
        .map_err(BroadcastError::Setup)?;

    let payload = broadcast_tx::build_signed_payload_bytes(
        proposal.seq_no,
        &proposal.action_hex,
        &proposal.signatures,
        &canonical_keys,
        &sighash,
    )
    .map_err(BroadcastError::Setup)?;

    let envelope_keypair = envelope_cache.get_or_generate(&payload);
    let (commit_address, _, _) =
        broadcast_tx::derive_commit_address(&envelope_keypair, &payload, network)
            .map_err(BroadcastError::Setup)?;

    let estimated_fee_sats = fee_rate.fee_sats(REVEAL_TX_VBYTES);
    let commit_amount_sats = COMMIT_DUST_SATS + estimated_fee_sats;

    Ok((
        broadcast_tx::device_facing_commit_address(&commit_address, network, hw_device),
        commit_amount_sats,
        estimated_fee_sats,
    ))
}

async fn report_broadcast(
    client: &dyn OrchestratorClient,
    action_id: &str,
    broadcast_status: &str,
    proposal_status: Option<&str>,
    commit_txid: Option<&str>,
    reveal_txid: Option<&str>,
    broadcast_error: Option<&str>,
) -> Result<(), BroadcastError> {
    client
        .report_broadcast_progress(
            action_id,
            ReportBroadcastProgressRequest {
                broadcast_status: broadcast_status.to_string(),
                proposal_status: proposal_status.map(str::to_string),
                commit_txid: commit_txid.map(str::to_string),
                reveal_txid: reveal_txid.map(str::to_string),
                broadcast_error: broadcast_error.map(str::to_string),
            },
        )
        .await?;
    Ok(())
}

/// Try each broadcaster in order; the first success wins (spec §8: Electrum first,
/// node fallback). When every broadcaster fails, returns [`BroadcastError::AllBroadcastersFailed`]
/// carrying both raw tx hexes so the UI can offer manual copy-and-broadcast.
async fn broadcast_via(
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    commit_hex: &str,
    reveal_hex: &str,
) -> Result<(), BroadcastError> {
    let mut errors: Vec<(String, String)> = Vec::new();
    for b in broadcasters {
        match b.broadcast_pair(commit_hex, reveal_hex).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!(broadcaster = b.name(), error = %e, "broadcaster failed");
                errors.push((b.name().to_string(), e.message));
            }
        }
    }
    Err(BroadcastError::AllBroadcastersFailed {
        commit_tx_hex: commit_hex.to_string(),
        reveal_tx_hex: reveal_hex.to_string(),
        errors,
    })
}

/// Outcome of awaiting the reveal confirmation.
///
/// `Confirmed` means the reveal reached at least one confirmation and the orchestrator was
/// promoted to `reveal_confirmed`. `PendingConfirmation` means the confirmation wait timed out
/// with zero confirmations — the reveal is still in the mempool and may confirm later. A
/// `PendingConfirmation` is **not** a failure: no `failed` status is reported and the
/// `PendingReveals` entry is retained so resubmit/reconcile remain possible.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfirmOutcome {
    /// Reveal reached >= 1 confirmation; orchestrator reported `reveal_confirmed`.
    Confirmed,
    /// Timed out with 0 confirmations; reveal remains in mempool (`reveal_broadcasted`).
    PendingConfirmation,
}

/// Pre-sign commit+reveal, store in PendingReveals, broadcast, and report up to
/// `reveal_broadcasted`. Returns `(commit_txid, reveal_txid)` **without** waiting for any
/// confirmation — the caller awaits confirmation separately (see [`await_reveal_confirmation`]).
///
/// Flow: claim → build_signed_commit → build_reveal_tx → drop keypair → insert pending →
/// broadcasters (Electrum first, node fallback) → report commit_broadcasted → report
/// reveal_broadcasted → return txids.
///
/// On any error during the broadcast stage (a genuine submission error), the proposal is
/// reported as `failed`. The broadcast NEVER advances the chain: confirmation is driven by the
/// dev faucet/harness on regtest and by real miners on testnet/mainnet. `get_raw_transaction`
/// is NEVER called.
#[allow(clippy::too_many_arguments)]
pub async fn submit_commit_then_reveal(
    client: &dyn OrchestratorClient,
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    asm_rpc_url: &str,
    magic_bytes: MagicBytes,
    network: Network,
    action_id: &str,
    fee_rate: FeeRate,
    commit_funding: &dyn CommitFunding,
    reveal_change_spk: ScriptBuf,
    pending: &PendingReveals,
    envelope_cache: &EnvelopeKeyCache,
) -> Result<(String, String), BroadcastError> {
    let proposal = client.claim_broadcast(action_id).await.map_err(|e| {
        if let OrchestratorError::Backend {
            status: 409,
            message,
        } = &e
        {
            BroadcastError::Setup(format!("broadcast already in progress: {message}"))
        } else {
            BroadcastError::ProposalFetch(e)
        }
    })?;

    if proposal.status != "approved" {
        return Err(BroadcastError::Setup(format!(
            "proposal must be in 'approved' state to broadcast (current: {})",
            proposal.status
        )));
    }

    let canonical_keys =
        asm_role_membership::ordered_keys_for_authority(asm_rpc_url, proposal.authority)
            .await
            .map_err(BroadcastError::Setup)?;

    let sighash = broadcast_tx::compute_sighash(proposal.seq_no, &proposal.action_hex)
        .map_err(BroadcastError::Setup)?;

    let payload = broadcast_tx::build_signed_payload_bytes(
        proposal.seq_no,
        &proposal.action_hex,
        &proposal.signatures,
        &canonical_keys,
        &sighash,
    )
    .map_err(BroadcastError::Setup)?;

    // Reuse the ephemeral keypair derived for this exact payload during the preview, so the
    // commit address the signer confirmed on device matches what we actually fund (issue #382).
    let envelope_keypair = envelope_cache.get_or_generate(&payload);
    let (commit_address, reveal_script, taproot_spend_info) =
        broadcast_tx::derive_commit_address(&envelope_keypair, &payload, network)
            .map_err(BroadcastError::Setup)?;

    let reveal_fee_sats = fee_rate.fee_sats(REVEAL_TX_VBYTES);
    let commit_amount_sats = COMMIT_DUST_SATS + reveal_fee_sats;

    let broadcast_result: Result<(String, String), BroadcastError> = async {
        // Step 1: Pre-sign commit tx.
        let commit_tx = commit_funding
            .build_signed_commit(
                &commit_address.to_string(),
                commit_amount_sats,
                fee_rate.to_bdk(),
            )
            .await
            .map_err(|e| BroadcastError::Setup(e.to_string()))?;

        // Step 2: Pre-sign reveal tx using the local commit tx (no get_raw_transaction).
        let commit_address_script = commit_address.script_pubkey();

        let action_bytes = hex::decode(&proposal.action_hex)
            .map_err(|e| BroadcastError::Setup(format!("invalid action hex: {e}")))?;
        let action = MultisigAction::from_ssz_bytes(&action_bytes)
            .map_err(|e| BroadcastError::Setup(format!("invalid SSZ action: {e:?}")))?;

        let reveal_tx = broadcast_tx::build_reveal_tx(
            &envelope_keypair,
            &reveal_script,
            &taproot_spend_info,
            &commit_tx,
            &commit_address_script,
            &action,
            magic_bytes,
            reveal_change_spk.clone(),
            reveal_fee_sats,
        )
        .map_err(BroadcastError::Setup)?;

        // Step 3: DROP ephemeral keypair — both txs are signed, key no longer needed.
        let _ = envelope_keypair;
        envelope_cache.evict(&payload);

        // Step 4: Serialize both transactions.
        let commit_txid = commit_tx.compute_txid().to_string();
        let reveal_txid = reveal_tx.compute_txid().to_string();
        let commit_hex = broadcast_tx::tx_to_hex(&commit_tx);
        let reveal_hex = broadcast_tx::tx_to_hex(&reveal_tx);

        // Step 5: Insert into PendingReveals BEFORE any broadcast.
        crate::infrastructure::pending_reveals_store::insert_and_persist(
            pending,
            action_id.to_string(),
            crate::application::pending_reveals::PendingReveal {
                reveal_tx_hex: reveal_hex.clone(),
                reveal_txid: reveal_txid.clone(),
                commit_txid: commit_txid.clone(),
            },
        );

        // Step 6: Broadcast — Electrum first, node fallback.
        broadcast_via(broadcasters, &commit_hex, &reveal_hex).await?;

        // Step 7: Report commit_broadcasted then reveal_broadcasted (no commit_confirmed).
        report_broadcast(
            client,
            action_id,
            "commit_broadcasted",
            None,
            Some(&commit_txid),
            None,
            None,
        )
        .await?;

        report_broadcast(
            client,
            action_id,
            "reveal_broadcasted",
            None,
            Some(&commit_txid),
            Some(&reveal_txid),
            None,
        )
        .await?;

        Ok((commit_txid, reveal_txid))
    }
    .await;

    if let Err(ref e) = broadcast_result {
        let _ = report_broadcast(
            client,
            action_id,
            "failed",
            None,
            None,
            None,
            Some(&e.to_string()),
        )
        .await;
    }

    broadcast_result
}

/// Await a single confirmation of the reveal tx, then promote the orchestrator.
///
/// Polls `get_transaction_confirmations(reveal_txid)`:
/// - On `>= 1` confirmation → reports `reveal_confirmed`, removes the `PendingReveals` entry,
///   and returns [`ConfirmOutcome::Confirmed`].
/// - On timeout with `0` confirmations → returns [`ConfirmOutcome::PendingConfirmation`]: it
///   does **not** report `failed`, keeps the last status at `reveal_broadcasted`, and retains
///   the `PendingReveals` entry. A slow block is never a failure.
/// - On a genuine RPC error while polling → returns `Err(BroadcastError::BitcoinRpc)` without
///   reporting `failed` (the tx is already broadcast; an on-open reconcile can recover later).
///
/// Intended to run in the background after [`submit_commit_then_reveal`] returns.
#[allow(clippy::too_many_arguments)]
pub async fn await_reveal_confirmation(
    client: &dyn OrchestratorClient,
    btc_rpc: &dyn BitcoinRpcClient,
    action_id: &str,
    commit_txid: &str,
    reveal_txid: &str,
    confirm_poll_interval_ms: u64,
    confirm_timeout_ms: u64,
    pending: &PendingReveals,
) -> Result<ConfirmOutcome, BroadcastError> {
    if !wait_for_confirmation(
        btc_rpc,
        reveal_txid,
        confirm_poll_interval_ms,
        confirm_timeout_ms,
    )
    .await?
    {
        // Timed out with 0 confirmations: stay reveal_broadcasted (mempool-pending).
        return Ok(ConfirmOutcome::PendingConfirmation);
    }

    report_broadcast(
        client,
        action_id,
        "reveal_confirmed",
        None,
        Some(commit_txid),
        Some(reveal_txid),
        None,
    )
    .await?;

    crate::infrastructure::pending_reveals_store::remove_and_persist(pending, action_id);

    Ok(ConfirmOutcome::Confirmed)
}

/// Synchronous submit + await-confirmation composition (retained for tests and any sequential
/// caller). Unlike the previous implementation, a confirmation timeout is **not** reported as
/// `failed` — it leaves the proposal at `reveal_broadcasted`.
#[allow(clippy::too_many_arguments)]
pub async fn broadcast_commit_then_reveal(
    client: &dyn OrchestratorClient,
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    btc_rpc: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
    magic_bytes: MagicBytes,
    network: Network,
    action_id: &str,
    fee_rate: FeeRate,
    confirm_poll_interval_ms: u64,
    confirm_timeout_ms: u64,
    commit_funding: &dyn CommitFunding,
    reveal_change_spk: ScriptBuf,
    pending: &PendingReveals,
    envelope_cache: &EnvelopeKeyCache,
) -> Result<(String, String), BroadcastError> {
    let (commit_txid, reveal_txid) = submit_commit_then_reveal(
        client,
        broadcasters,
        asm_rpc_url,
        magic_bytes,
        network,
        action_id,
        fee_rate,
        commit_funding,
        reveal_change_spk,
        pending,
        envelope_cache,
    )
    .await?;

    await_reveal_confirmation(
        client,
        btc_rpc,
        action_id,
        &commit_txid,
        &reveal_txid,
        confirm_poll_interval_ms,
        confirm_timeout_ms,
        pending,
    )
    .await?;

    Ok((commit_txid, reveal_txid))
}

/// Poll until the tx reaches >= 1 confirmation (`Ok(true)`) or the timeout elapses
/// (`Ok(false)`). A genuine RPC error short-circuits to `Err(BitcoinRpc)`.
async fn wait_for_confirmation(
    btc_rpc: &dyn BitcoinRpcClient,
    txid: &str,
    poll_interval_ms: u64,
    timeout_ms: u64,
) -> Result<bool, BroadcastError> {
    let start = std::time::Instant::now();
    loop {
        let confs = btc_rpc
            .get_transaction_confirmations(txid)
            .await
            .map_err(BroadcastError::BitcoinRpc)?;
        if confs >= 1 {
            return Ok(true);
        }
        if start.elapsed().as_millis() as u64 >= timeout_ms {
            return Ok(false);
        }
        tokio::time::sleep(std::time::Duration::from_millis(poll_interval_ms)).await;
    }
}

/// Assemble commit/reveal fee estimate for a manual proposal (no orchestrator fetch).
///
/// `authority` is the wire-format string (e.g. `"strata_admin"`).
#[allow(clippy::too_many_arguments)]
pub async fn prepare_broadcast_manual(
    asm_rpc_url: &str,
    network: Network,
    action_hex: &str,
    seq_no: u64,
    authority: &str,
    signatures: &[Signature],
    fee_rate: FeeRate,
    envelope_cache: &EnvelopeKeyCache,
    hw_device: Option<HwDeviceType>,
) -> Result<(String, u64, u64), BroadcastError> {
    let auth = crate::domain::authority::Authority::from_wire(authority)
        .map_err(|e| BroadcastError::Setup(e.to_string()))?;
    let canonical_keys = asm_role_membership::ordered_keys_for_authority(asm_rpc_url, auth)
        .await
        .map_err(BroadcastError::Setup)?;

    let proxy_sigs: Vec<crate::domain::proposal::ProposalSignature> = signatures
        .iter()
        .map(|s| crate::domain::proposal::ProposalSignature {
            signer_pubkey: s.signer_pubkey.clone(),
            signature_hex: s.signature_hex.clone(),
        })
        .collect();

    let sighash =
        broadcast_tx::compute_sighash(seq_no, action_hex).map_err(BroadcastError::Setup)?;

    let payload = broadcast_tx::build_signed_payload_bytes(
        seq_no,
        action_hex,
        &proxy_sigs,
        &canonical_keys,
        &sighash,
    )
    .map_err(BroadcastError::Setup)?;

    let envelope_keypair = envelope_cache.get_or_generate(&payload);
    let (commit_address, _, _) =
        broadcast_tx::derive_commit_address(&envelope_keypair, &payload, network)
            .map_err(BroadcastError::Setup)?;

    let estimated_fee_sats = fee_rate.fee_sats(REVEAL_TX_VBYTES);
    let commit_amount_sats = COMMIT_DUST_SATS + estimated_fee_sats;

    Ok((
        broadcast_tx::device_facing_commit_address(&commit_address, network, hw_device),
        commit_amount_sats,
        estimated_fee_sats,
    ))
}

/// Execute commit+reveal broadcast for a manual proposal (no orchestrator — no claim, no reporting).
///
/// Uses a derived key `"manual-<first-16-chars-of-sighash>"` as the PendingReveals key.
#[allow(clippy::too_many_arguments)]
pub async fn broadcast_manual(
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    btc_rpc: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
    magic_bytes: MagicBytes,
    network: Network,
    action_hex: &str,
    seq_no: u64,
    authority: &str,
    signatures: &[Signature],
    fee_rate: FeeRate,
    confirm_poll_interval_ms: u64,
    confirm_timeout_ms: u64,
    commit_funding: &dyn CommitFunding,
    reveal_change_spk: ScriptBuf,
    pending: &PendingReveals,
    envelope_cache: &EnvelopeKeyCache,
) -> Result<(String, String), BroadcastError> {
    let auth = crate::domain::authority::Authority::from_wire(authority)
        .map_err(|e| BroadcastError::Setup(e.to_string()))?;
    let canonical_keys = asm_role_membership::ordered_keys_for_authority(asm_rpc_url, auth)
        .await
        .map_err(BroadcastError::Setup)?;

    let proxy_sigs: Vec<crate::domain::proposal::ProposalSignature> = signatures
        .iter()
        .map(|s| crate::domain::proposal::ProposalSignature {
            signer_pubkey: s.signer_pubkey.clone(),
            signature_hex: s.signature_hex.clone(),
        })
        .collect();

    let sighash =
        broadcast_tx::compute_sighash(seq_no, action_hex).map_err(BroadcastError::Setup)?;

    let payload = broadcast_tx::build_signed_payload_bytes(
        seq_no,
        action_hex,
        &proxy_sigs,
        &canonical_keys,
        &sighash,
    )
    .map_err(BroadcastError::Setup)?;

    // Reuse the preview's ephemeral keypair for this payload so the on-device commit address
    // matches the app's "COMMIT TX PREVIEW" (issue #382).
    let envelope_keypair = envelope_cache.get_or_generate(&payload);
    let (commit_address, reveal_script, taproot_spend_info) =
        broadcast_tx::derive_commit_address(&envelope_keypair, &payload, network)
            .map_err(BroadcastError::Setup)?;

    let reveal_fee_sats = fee_rate.fee_sats(REVEAL_TX_VBYTES);
    let commit_amount_sats = COMMIT_DUST_SATS + reveal_fee_sats;

    // Use sighash hex prefix as the PendingReveals key (no orchestrator action_id).
    let sighash_hex = hex::encode(sighash);
    let pending_key = format!("manual-{}", &sighash_hex[..sighash_hex.len().min(16)]);

    let broadcast_result: Result<(String, String), BroadcastError> = async {
        let commit_tx = commit_funding
            .build_signed_commit(
                &commit_address.to_string(),
                commit_amount_sats,
                fee_rate.to_bdk(),
            )
            .await
            .map_err(|e| BroadcastError::Setup(e.to_string()))?;

        let commit_address_script = commit_address.script_pubkey();

        let action_bytes = hex::decode(action_hex)
            .map_err(|e| BroadcastError::Setup(format!("invalid action hex: {e}")))?;
        let action = MultisigAction::from_ssz_bytes(&action_bytes)
            .map_err(|e| BroadcastError::Setup(format!("invalid SSZ action: {e:?}")))?;

        let reveal_tx = broadcast_tx::build_reveal_tx(
            &envelope_keypair,
            &reveal_script,
            &taproot_spend_info,
            &commit_tx,
            &commit_address_script,
            &action,
            magic_bytes,
            reveal_change_spk.clone(),
            reveal_fee_sats,
        )
        .map_err(BroadcastError::Setup)?;

        let _ = envelope_keypair;
        envelope_cache.evict(&payload);

        let commit_txid = commit_tx.compute_txid().to_string();
        let reveal_txid = reveal_tx.compute_txid().to_string();
        let commit_hex = broadcast_tx::tx_to_hex(&commit_tx);
        let reveal_hex = broadcast_tx::tx_to_hex(&reveal_tx);

        crate::infrastructure::pending_reveals_store::insert_and_persist(
            pending,
            pending_key.clone(),
            crate::application::pending_reveals::PendingReveal {
                reveal_tx_hex: reveal_hex.clone(),
                reveal_txid: reveal_txid.clone(),
                commit_txid: commit_txid.clone(),
            },
        );

        broadcast_via(broadcasters, &commit_hex, &reveal_hex).await?;

        wait_for_confirmation(
            btc_rpc,
            &reveal_txid,
            confirm_poll_interval_ms,
            confirm_timeout_ms,
        )
        .await?;

        crate::infrastructure::pending_reveals_store::remove_and_persist(pending, &pending_key);

        Ok((commit_txid, reveal_txid))
    }
    .await;

    broadcast_result
}

/// Create a new action and store the creator's signature.
///
/// Mirrors PRD: `create_update_action(action, seq, sig)`.
///
/// Callers are responsible for encoding the action to SSZ hex before calling this
/// function (`infrastructure::action_codec::encode_hex`).
pub async fn create_update_action(
    client: &dyn OrchestratorClient,
    action_hex: &str,
    seq_no: u64,
    signature: &Signature,
    title: Option<String>,
) -> Result<Proposal, ProposalError> {
    let request = CreateProposalRequest {
        seq_no,
        action_hex: action_hex.to_string(),
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
        title,
    };

    let proposal = client.create_proposal(request).await?;
    if proposal.status == "pending" && orchestrator_quorum_reached(&proposal) {
        return transition_to_approved(client, &proposal.action_id).await;
    }
    Ok(proposal)
}

fn orchestrator_quorum_reached(proposal: &Proposal) -> bool {
    proposal.signatures.len() >= proposal.required_signatures as usize
}

/// Explicit pending → approved after quorum (P-012 / ADR-006).
pub async fn transition_to_approved(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<Proposal, ProposalError> {
    let proposal = client
        .transition_to_approved(
            action_id,
            TransitionProposalRequest {
                proposal_status: "approved".to_string(),
            },
        )
        .await?;
    Ok(proposal)
}

/// Append an approval signature; when quorum is reached, persist `approved` on the orchestrator.
pub async fn approve_action(
    client: &dyn OrchestratorClient,
    action_id: &str,
    signature: &Signature,
) -> Result<Proposal, ProposalError> {
    let request = ApproveActionRequest {
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
    };

    let proposal = client.approve_action(action_id, request).await?;
    if proposal.status == "pending" && orchestrator_quorum_reached(&proposal) {
        return transition_to_approved(client, action_id).await;
    }
    Ok(proposal)
}

/// Create a Cancel proposal for an approved target and store the initiator's signature.
///
/// The orchestrator is idempotent: when a cancel proposal already exists for `target_action_id`
/// it is returned unchanged, without recording another signature.
///
/// Like `create_update_action`, when that first signature already satisfies quorum (effective
/// threshold 1) the explicit pending → approved transition is persisted here (P-012 / ADR-006) —
/// the orchestrator never transitions on its own.
///
/// Callers are responsible for encoding the cancel action to SSZ hex before calling this
/// function (`infrastructure::action_codec::encode_hex`).
pub async fn create_cancel_action(
    client: &dyn OrchestratorClient,
    target_action_id: &str,
    action_hex: &str,
    seq_no: u64,
    signature: &Signature,
) -> Result<Proposal, ProposalError> {
    let request = CreateCancelProposalRequest {
        seq_no,
        action_hex: action_hex.to_string(),
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
    };

    let proposal = client
        .create_cancel_proposal(target_action_id, request)
        .await?;
    // The cancel proposal carries its own action id — transitioning `target_action_id` here
    // would approve the very proposal being cancelled.
    if proposal.status == "pending" && orchestrator_quorum_reached(&proposal) {
        return transition_to_approved(client, &proposal.action_id).await;
    }
    Ok(proposal)
}

/// Fetch the action payload and details.
///
/// Mirrors PRD: `get_update_action(id)`.
pub async fn get_update_action(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<Proposal, ProposalError> {
    let proposal = client.get_proposal(action_id).await?;
    Ok(proposal)
}

/// Pre-broadcast guard for a Cancel proposal: is its target action still queued on the ASM?
pub async fn get_cancel_target_status(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<bool, ProposalError> {
    let status = client.get_cancel_target_status(action_id).await?;
    Ok(status.target_queued)
}

/// List proposals, optionally filtered by status.
pub async fn list_proposals(
    client: &dyn OrchestratorClient,
    status: Option<&str>,
) -> Result<Vec<Proposal>, ProposalError> {
    let proposals = client.list_proposals(status).await?;
    Ok(proposals)
}

/// Prepare commit/reveal fee estimate locally (desktop-owned Bitcoin RPC).
#[allow(clippy::too_many_arguments)]
pub async fn prepare_broadcast_local(
    client: &dyn OrchestratorClient,
    asm_rpc_url: &str,
    network: Network,
    action_id: &str,
    fee_rate: FeeRate,
    envelope_cache: &EnvelopeKeyCache,
    hw_device: Option<HwDeviceType>,
) -> Result<(String, u64, u64), BroadcastError> {
    prepare_broadcast_bundle(
        client,
        asm_rpc_url,
        network,
        action_id,
        fee_rate,
        envelope_cache,
        hw_device,
    )
    .await
}

/// Re-broadcast a stored reveal transaction for a given action_id.
///
/// Looks up the reveal_tx_hex in PendingReveals and calls send_raw_transaction.
/// Does NOT remove the entry — removal happens on reveal_confirmed.
pub async fn resubmit_reveal(
    pending: &PendingReveals,
    btc_rpc: &dyn BitcoinRpcClient,
    _client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<String, BroadcastError> {
    let reveal_tx_hex = {
        let guard = pending.lock().unwrap();
        guard
            .get(action_id)
            .ok_or_else(|| BroadcastError::NoPendingReveal {
                action_id: action_id.to_string(),
            })?
            .reveal_tx_hex
            .clone()
    };
    let txid = btc_rpc
        .send_raw_transaction(&reveal_tx_hex)
        .await
        .map_err(BroadcastError::BitcoinRpc)?;
    Ok(txid)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::orchestrator_client::OrchestratorError;
    use crate::application::orchestrator_client::{
        CompleteOrchestratorAuthRequest, OrchestratorAuthChallenge, OrchestratorAuthSession,
        StartOrchestratorAuthRequest,
    };
    use crate::application::tx_broadcaster::tests::MockBroadcaster;
    use crate::domain::action::{Action, CompressedPubKey, MultisigUpdate};
    use crate::domain::authority::Authority;
    use crate::domain::proposal::{Proposal as OrcProposal, ProposalSignature};
    use crate::infrastructure::action_codec;
    use crate::infrastructure::node_broadcaster::NodeBroadcaster;
    use crate::infrastructure::signing;
    use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
    use rand::rngs::OsRng;
    use std::num::NonZeroU8;
    use std::sync::{Arc, Mutex};

    // Helper: create broadcaster vec from a MockBtcRpc (wraps it in NodeBroadcaster).
    fn node_broadcasters(
        rpc: Arc<MockBtcRpc>,
    ) -> Vec<std::sync::Arc<dyn crate::application::tx_broadcaster::TxBroadcaster>> {
        vec![std::sync::Arc::new(NodeBroadcaster::new(
            rpc as Arc<dyn crate::infrastructure::bitcoin_rpc::BitcoinRpcClient>,
        ))]
    }

    // Helper: single always-ok mock broadcaster for tests that don't need RPC-level assertions.
    fn ok_broadcasters(
    ) -> Vec<std::sync::Arc<dyn crate::application::tx_broadcaster::TxBroadcaster>> {
        vec![std::sync::Arc::new(MockBroadcaster::ok("mock"))]
    }

    // ─── Test helpers ───────────────────────────────────────────────────────

    fn generate_test_keypair() -> (String, String) {
        let sk = SecretKey::new(&mut OsRng);
        let pk = PublicKey::from_secret_key(SECP256K1, &sk);
        (hex::encode(sk.secret_bytes()), hex::encode(pk.serialize()))
    }

    /// Builds a sample `Action::MultisigUpdate` via domain types only.
    fn demo_action() -> Action {
        let demo_bytes = [0x42u8; 32];
        let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
        let new_signer_pk = PublicKey::from_secret_key(SECP256K1, &demo_sk);
        let new_signer = CompressedPubKey::new(new_signer_pk.serialize());
        Action::MultisigUpdate(MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: vec![new_signer],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).expect("non-zero"),
        })
    }

    fn demo_action_hex() -> String {
        action_codec::encode_hex(&demo_action()).expect("encode ok")
    }

    /// Action with 4 keys — produces a payload large enough for taproot envelope (>= 126 bytes).
    fn large_demo_action_hex() -> String {
        let keys: Vec<CompressedPubKey> = (1u8..=4)
            .map(|i| {
                let mut seed = [0x42u8; 32];
                seed[0] = i;
                let sk = SecretKey::from_slice(&seed).expect("valid key");
                let pk = PublicKey::from_secret_key(SECP256K1, &sk);
                CompressedPubKey::new(pk.serialize())
            })
            .collect();
        let action = Action::MultisigUpdate(MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: keys,
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).expect("non-zero"),
        });
        action_codec::encode_hex(&action).expect("encode ok")
    }

    fn sign_action(secret_key_hex: &str, seq_no: u64, action_hex: &str) -> Signature {
        let sighash = signing::compute_sighash(seq_no, action_hex).expect("sighash ok");
        let sig = signing::sign_sighash(secret_key_hex, &sighash.sighash_hex).expect("sign ok");
        Signature {
            signer_pubkey: sig.public_key_hex,
            signature_hex: sig.signature_hex,
        }
    }

    struct MockOrchestratorClient {
        last_create_request: Mutex<Option<CreateProposalRequest>>,
        last_approve_request: Mutex<Option<(String, ApproveActionRequest)>>,
        last_cancel_request: Mutex<Option<(String, CreateCancelProposalRequest)>>,
        transition_called: Mutex<bool>,
        last_transition_action_id: Mutex<Option<String>>,
        approve_signature_count: Mutex<usize>,
        claim_broadcast_called: Mutex<bool>,
        report_broadcast_called: Mutex<bool>,
        last_report_request:
            Mutex<Option<crate::application::orchestrator_client::ReportBroadcastProgressRequest>>,
        should_fail: bool,
        required_signatures: u16,
    }

    impl MockOrchestratorClient {
        fn new() -> Self {
            Self {
                last_create_request: Mutex::new(None),
                last_approve_request: Mutex::new(None),
                last_cancel_request: Mutex::new(None),
                transition_called: Mutex::new(false),
                last_transition_action_id: Mutex::new(None),
                approve_signature_count: Mutex::new(0),
                claim_broadcast_called: Mutex::new(false),
                report_broadcast_called: Mutex::new(false),
                last_report_request: Mutex::new(None),
                should_fail: false,
                required_signatures: 2,
            }
        }

        fn with_required_signatures(required_signatures: u16) -> Self {
            Self {
                required_signatures,
                ..Self::new()
            }
        }

        fn failing() -> Self {
            Self {
                last_create_request: Mutex::new(None),
                last_approve_request: Mutex::new(None),
                last_cancel_request: Mutex::new(None),
                transition_called: Mutex::new(false),
                last_transition_action_id: Mutex::new(None),
                approve_signature_count: Mutex::new(0),
                claim_broadcast_called: Mutex::new(false),
                report_broadcast_called: Mutex::new(false),
                last_report_request: Mutex::new(None),
                should_fail: true,
                required_signatures: 2,
            }
        }

        fn claim_broadcast_called(&self) -> bool {
            *self.claim_broadcast_called.lock().unwrap()
        }

        fn last_create_request(&self) -> Option<CreateProposalRequest> {
            self.last_create_request.lock().unwrap().take()
        }

        fn last_approve_request(&self) -> Option<(String, ApproveActionRequest)> {
            self.last_approve_request.lock().unwrap().take()
        }

        fn last_cancel_request(&self) -> Option<(String, CreateCancelProposalRequest)> {
            self.last_cancel_request.lock().unwrap().take()
        }
    }

    #[async_trait::async_trait]
    impl OrchestratorClient for MockOrchestratorClient {
        async fn auth_challenge(
            &self,
            _request: StartOrchestratorAuthRequest,
        ) -> Result<OrchestratorAuthChallenge, OrchestratorError> {
            Err(OrchestratorError::Request("not used in tests".to_string()))
        }

        async fn auth_verify(
            &self,
            _request: CompleteOrchestratorAuthRequest,
        ) -> Result<OrchestratorAuthSession, OrchestratorError> {
            Err(OrchestratorError::Request("not used in tests".to_string()))
        }

        async fn auth_logout(&self) -> Result<(), OrchestratorError> {
            Err(OrchestratorError::Request("not used in tests".to_string()))
        }

        async fn create_proposal(
            &self,
            request: CreateProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            let response = OrcProposal {
                action_id: format!("action_{}", request.seq_no),
                authority: Authority::StrataAdmin,
                seq_no: request.seq_no,
                action_hex: request.action_hex.clone(),
                title: None,
                status: "pending".to_string(),
                required_signatures: self.required_signatures,
                signatures: vec![ProposalSignature {
                    signer_pubkey: request.signer_pubkey.clone(),
                    signature_hex: request.signature_hex.clone(),
                }],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            };
            *self.last_create_request.lock().unwrap() = Some(request);
            Ok(response)
        }

        async fn create_cancel_proposal(
            &self,
            target_action_id: &str,
            request: CreateCancelProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            // A `cancel_` prefix keeps the cancel proposal's own id distinguishable from the
            // target's `action_` id, so tests can prove which one the transition targets.
            let response = OrcProposal {
                action_id: format!("cancel_{}", request.seq_no),
                authority: Authority::StrataAdmin,
                seq_no: request.seq_no,
                action_hex: request.action_hex.clone(),
                title: None,
                status: "pending".to_string(),
                required_signatures: self.required_signatures,
                signatures: vec![ProposalSignature {
                    signer_pubkey: request.signer_pubkey.clone(),
                    signature_hex: request.signature_hex.clone(),
                }],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: Some(target_action_id.to_string()),
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            };
            *self.last_cancel_request.lock().unwrap() =
                Some((target_action_id.to_string(), request));
            Ok(response)
        }

        async fn get_proposal(&self, action_id: &str) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                title: None,
                status: "pending".to_string(),
                required_signatures: self.required_signatures,
                signatures: vec![],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            })
        }

        async fn get_cancel_target_status(
            &self,
            _action_id: &str,
        ) -> Result<
            crate::application::orchestrator_client::CancelTargetStatusResponse,
            OrchestratorError,
        > {
            Ok(
                crate::application::orchestrator_client::CancelTargetStatusResponse {
                    target_queued: true,
                },
            )
        }

        async fn approve_action(
            &self,
            action_id: &str,
            request: ApproveActionRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.last_approve_request.lock().unwrap() = Some((action_id.to_string(), request));
            let mut count = self.approve_signature_count.lock().unwrap();
            *count += 1;
            let signatures = (0..*count)
                .map(|i| ProposalSignature {
                    signer_pubkey: format!("signer_{i}"),
                    signature_hex: format!("sig_{i}"),
                })
                .collect();
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                title: None,
                status: "pending".to_string(),
                required_signatures: self.required_signatures,
                signatures,
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            })
        }

        async fn transition_to_approved(
            &self,
            action_id: &str,
            _request: TransitionProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            *self.transition_called.lock().unwrap() = true;
            *self.last_transition_action_id.lock().unwrap() = Some(action_id.to_string());
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                title: None,
                status: "approved".to_string(),
                required_signatures: 2,
                signatures: vec![
                    ProposalSignature {
                        signer_pubkey: "signer_0".to_string(),
                        signature_hex: "sig_0".to_string(),
                    },
                    ProposalSignature {
                        signer_pubkey: "signer_1".to_string(),
                        signature_hex: "sig_1".to_string(),
                    },
                ],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            })
        }

        async fn list_proposals(
            &self,
            _status: Option<&str>,
        ) -> Result<Vec<OrcProposal>, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(vec![OrcProposal {
                action_id: "action_1".to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                title: None,
                status: "pending".to_string(),
                required_signatures: self.required_signatures,
                signatures: vec![],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            }])
        }

        async fn get_next_seq_no(&self) -> Result<u64, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(1)
        }

        async fn claim_broadcast(&self, action_id: &str) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.claim_broadcast_called.lock().unwrap() = true;
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                title: None,
                status: "approved".to_string(),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: "commit_broadcasted".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            })
        }

        async fn report_broadcast_progress(
            &self,
            action_id: &str,
            request: crate::application::orchestrator_client::ReportBroadcastProgressRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.report_broadcast_called.lock().unwrap() = true;
            *self.last_report_request.lock().unwrap() = Some(request.clone());
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                title: None,
                status: request
                    .proposal_status
                    .unwrap_or_else(|| "approved".to_string()),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: request.broadcast_status,
                commit_txid: request.commit_txid,
                reveal_txid: request.reveal_txid,
                broadcast_error: request.broadcast_error,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            })
        }
    }

    // ─── Tests ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_update_action() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = create_update_action(&mock, &action_hex, 1, &sig, None)
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.status, "pending");
        assert_eq!(result.signatures.len(), 1);
        assert_eq!(result.signatures[0].signer_pubkey, sig.signer_pubkey);

        let req = mock.last_create_request().expect("request sent");
        assert_eq!(req.seq_no, 1);
        assert_eq!(req.action_hex, action_hex);
    }

    #[tokio::test]
    async fn test_create_at_quorum_calls_transition() {
        let mock = MockOrchestratorClient::with_required_signatures(1);
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = create_update_action(&mock, &action_hex, 1, &sig, None)
            .await
            .expect("should succeed");

        assert_eq!(result.status, "approved");
        assert!(*mock.transition_called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_create_cancel_at_quorum_calls_transition() {
        let mock = MockOrchestratorClient::with_required_signatures(1);
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 7, &action_hex);

        let result = create_cancel_action(&mock, "action_target", &action_hex, 7, &sig)
            .await
            .expect("should succeed");

        assert_eq!(result.status, "approved");
        assert!(*mock.transition_called.lock().unwrap());
        // The transition must target the cancel proposal, never the proposal being cancelled.
        assert_eq!(
            mock.last_transition_action_id.lock().unwrap().as_deref(),
            Some("cancel_7")
        );
    }

    #[tokio::test]
    async fn test_create_cancel_below_quorum_stays_pending() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 7, &action_hex);

        let result = create_cancel_action(&mock, "action_target", &action_hex, 7, &sig)
            .await
            .expect("should succeed");

        assert_eq!(result.status, "pending");
        assert_eq!(result.signatures.len(), 1);
        assert_eq!(result.target_action_id.as_deref(), Some("action_target"));
        assert!(!*mock.transition_called.lock().unwrap());

        let (target, req) = mock.last_cancel_request().expect("request sent");
        assert_eq!(target, "action_target");
        assert_eq!(req.seq_no, 7);
        assert_eq!(req.action_hex, action_hex);
        assert_eq!(req.signer_pubkey, sig.signer_pubkey);
    }

    #[tokio::test]
    async fn test_create_cancel_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 7, &action_hex);

        let result = create_cancel_action(&mock, "action_target", &action_hex, 7, &sig).await;

        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }

    #[tokio::test]
    async fn test_approve_action() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = approve_action(&mock, "action_1", &sig)
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.status, "pending");

        let (action_id, req) = mock.last_approve_request().expect("request sent");
        assert_eq!(action_id, "action_1");
        assert_eq!(req.signer_pubkey, sig.signer_pubkey);
        assert!(!*mock.transition_called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_approve_at_quorum_calls_transition() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let _first = approve_action(&mock, "action_1", &sig)
            .await
            .expect("first approve");
        assert!(!*mock.transition_called.lock().unwrap());

        let result = approve_action(&mock, "action_1", &sig)
            .await
            .expect("quorum approve");
        assert_eq!(result.status, "approved");
        assert!(*mock.transition_called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_get_update_action() {
        let mock = MockOrchestratorClient::new();

        let result = get_update_action(&mock, "action_1")
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.authority, Authority::StrataAdmin);
    }

    #[tokio::test]
    async fn test_create_then_get_consistent() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let created = create_update_action(&mock, &action_hex, 1, &sig, None)
            .await
            .expect("should succeed");

        let detail = get_update_action(&mock, &created.action_id)
            .await
            .expect("should succeed");

        assert_eq!(created.authority, detail.authority);
        assert_eq!(created.seq_no, detail.seq_no);
    }

    #[tokio::test]
    async fn test_signature_is_verifiable() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let _result = create_update_action(&mock, &action_hex, 1, &sig, None)
            .await
            .expect("should succeed");

        let req = mock.last_create_request().expect("request sent");
        let sighash = signing::compute_sighash(1, &action_hex).expect("sighash ok");
        let verify = signing::verify_threshold(
            &[req.signer_pubkey],
            1,
            &[req.signature_hex],
            &sighash.sighash_hex,
        )
        .expect("verify ok");

        assert!(verify.valid);
    }

    #[tokio::test]
    async fn test_create_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = create_update_action(&mock, &action_hex, 1, &sig, None).await;

        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }

    #[tokio::test]
    async fn test_approve_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = approve_action(&mock, "action_1", &sig).await;

        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }

    #[tokio::test]
    async fn test_list_proposals() {
        let mock = MockOrchestratorClient::new();
        let proposals = list_proposals(&mock, Some("pending"))
            .await
            .expect("should succeed");
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].action_id, "action_1");
    }

    #[tokio::test]
    async fn test_claim_broadcast_coordination() {
        let mock = MockOrchestratorClient::new();
        let proposal = mock.claim_broadcast("action_42").await.expect("claim ok");
        assert!(mock.claim_broadcast_called());
        assert_eq!(proposal.action_id, "action_42");
        assert_eq!(proposal.status, "approved");
    }

    // ─── Acceptance test: CommitFunding abstraction is used ─────────────────

    struct SpyCommitFunding {
        build_signed_commit_called: Mutex<bool>,
        captured_commit_address: Mutex<Option<String>>,
    }

    impl SpyCommitFunding {
        fn new(_txid: &str) -> Self {
            Self {
                build_signed_commit_called: Mutex::new(false),
                captured_commit_address: Mutex::new(None),
            }
        }

        fn was_called(&self) -> bool {
            *self.build_signed_commit_called.lock().unwrap()
        }

        /// The exact commit address string the broadcast funded (the real, on-network address).
        fn funded_commit_address(&self) -> Option<String> {
            self.captured_commit_address.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl crate::application::commit_funding::CommitFunding for SpyCommitFunding {
        async fn build_signed_commit(
            &self,
            commit_address: &str,
            _amount_sats: u64,
            _fee_rate: bdk_wallet::bitcoin::FeeRate,
        ) -> Result<bitcoin::Transaction, crate::application::commit_funding::CommitFundingError>
        {
            *self.build_signed_commit_called.lock().unwrap() = true;
            *self.captured_commit_address.lock().unwrap() = Some(commit_address.to_string());
            use bitcoin::{
                absolute::LockTime, transaction::Version, Address, Transaction, TxIn, TxOut,
            };
            use std::str::FromStr;
            // Parse the commit address to get the correct script_pubkey so build_reveal_tx can find the vout.
            let addr = Address::from_str(commit_address)
                .expect("valid commit address")
                .assume_checked();
            Ok(Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![TxIn::default()],
                output: vec![TxOut {
                    value: bitcoin::Amount::from_sat(10_000),
                    script_pubkey: addr.script_pubkey(),
                }],
            })
        }
    }

    /// MockBitcoinRpcClient: configurable submit_package result and call counters.
    struct MockBtcRpc {
        submit_package_result: Result<(), String>,
        send_raw_transaction_call_count: Mutex<u32>,
        get_raw_transaction_call_count: Mutex<u32>,
        /// Confirmations returned by `get_transaction_confirmations` (default 1).
        confirmations: u32,
    }

    impl MockBtcRpc {
        fn new(_commit_txid: &str) -> Self {
            Self {
                submit_package_result: Ok(()),
                send_raw_transaction_call_count: Mutex::new(0),
                get_raw_transaction_call_count: Mutex::new(0),
                confirmations: 1,
            }
        }

        /// Submit succeeds but the reveal never confirms (0 confirmations).
        fn with_zero_confirmations() -> Self {
            Self {
                submit_package_result: Ok(()),
                send_raw_transaction_call_count: Mutex::new(0),
                get_raw_transaction_call_count: Mutex::new(0),
                confirmations: 0,
            }
        }

        fn with_submit_package_error(err: &str) -> Self {
            Self {
                submit_package_result: Err(err.to_string()),
                send_raw_transaction_call_count: Mutex::new(0),
                get_raw_transaction_call_count: Mutex::new(0),
                confirmations: 1,
            }
        }

        fn send_raw_transaction_call_count(&self) -> u32 {
            *self.send_raw_transaction_call_count.lock().unwrap()
        }

        fn get_raw_transaction_call_count(&self) -> u32 {
            *self.get_raw_transaction_call_count.lock().unwrap()
        }
    }

    #[async_trait::async_trait]
    impl crate::infrastructure::bitcoin_rpc::BitcoinRpcClient for MockBtcRpc {
        async fn send_raw_transaction(&self, _: &str) -> Result<String, String> {
            *self.send_raw_transaction_call_count.lock().unwrap() += 1;
            Ok("reveal-txid-mock".to_string())
        }

        async fn get_transaction_confirmations(&self, _txid: &str) -> Result<u32, String> {
            Ok(self.confirmations)
        }

        async fn get_raw_transaction(&self, _txid: &str) -> Result<bitcoin::Transaction, String> {
            *self.get_raw_transaction_call_count.lock().unwrap() += 1;
            use bitcoin::{absolute::LockTime, transaction::Version, Transaction, TxIn, TxOut};
            Ok(Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![TxIn::default()],
                output: vec![TxOut {
                    value: bitcoin::Amount::from_sat(10_000),
                    script_pubkey: bitcoin::ScriptBuf::new(),
                }],
            })
        }

        async fn submit_package(&self, _: &[String]) -> Result<(), String> {
            self.submit_package_result.clone()
        }

        async fn get_block_count(&self) -> Result<u64, String> {
            Ok(0)
        }

        async fn estimate_smart_fee_sat_per_kvb(&self, _: u16) -> Result<u64, String> {
            Ok(1_000)
        }

        async fn min_relay_sat_per_kvb(&self) -> Result<u64, String> {
            Ok(1_000)
        }
    }

    /// Minimal orchestrator mock that returns an action large enough for the taproot envelope.
    ///
    /// Records every `report_broadcast_progress` request so tests can assert which broadcast
    /// statuses were (or were not) reported.
    #[derive(Default)]
    struct MockOrchestratorClientLargeAction {
        reports:
            Mutex<Vec<crate::application::orchestrator_client::ReportBroadcastProgressRequest>>,
    }

    impl MockOrchestratorClientLargeAction {
        fn new() -> Self {
            Self::default()
        }

        fn reported_statuses(&self) -> Vec<String> {
            self.reports
                .lock()
                .unwrap()
                .iter()
                .map(|r| r.broadcast_status.clone())
                .collect()
        }
    }

    #[async_trait::async_trait]
    impl OrchestratorClient for MockOrchestratorClientLargeAction {
        async fn auth_challenge(
            &self,
            _: crate::application::orchestrator_client::StartOrchestratorAuthRequest,
        ) -> Result<
            crate::application::orchestrator_client::OrchestratorAuthChallenge,
            OrchestratorError,
        > {
            unimplemented!()
        }
        async fn auth_verify(
            &self,
            _: crate::application::orchestrator_client::CompleteOrchestratorAuthRequest,
        ) -> Result<
            crate::application::orchestrator_client::OrchestratorAuthSession,
            OrchestratorError,
        > {
            unimplemented!()
        }
        async fn auth_logout(&self) -> Result<(), OrchestratorError> {
            unimplemented!()
        }
        async fn create_proposal(
            &self,
            _: CreateProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            unimplemented!()
        }
        async fn get_proposal(&self, action_id: &str) -> Result<OrcProposal, OrchestratorError> {
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: large_demo_action_hex(),
                title: None,
                status: "approved".to_string(),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            })
        }
        async fn get_cancel_target_status(
            &self,
            _action_id: &str,
        ) -> Result<
            crate::application::orchestrator_client::CancelTargetStatusResponse,
            OrchestratorError,
        > {
            Ok(
                crate::application::orchestrator_client::CancelTargetStatusResponse {
                    target_queued: true,
                },
            )
        }
        async fn approve_action(
            &self,
            _: &str,
            _: ApproveActionRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            unimplemented!()
        }
        async fn transition_to_approved(
            &self,
            _: &str,
            _: TransitionProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            unimplemented!()
        }
        async fn list_proposals(
            &self,
            _: Option<&str>,
        ) -> Result<Vec<OrcProposal>, OrchestratorError> {
            unimplemented!()
        }
        async fn get_next_seq_no(&self) -> Result<u64, OrchestratorError> {
            unimplemented!()
        }
        async fn claim_broadcast(&self, action_id: &str) -> Result<OrcProposal, OrchestratorError> {
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: large_demo_action_hex(),
                title: None,
                status: "approved".to_string(),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: "idle".to_string(),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            })
        }
        async fn report_broadcast_progress(
            &self,
            action_id: &str,
            request: crate::application::orchestrator_client::ReportBroadcastProgressRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            self.reports.lock().unwrap().push(request.clone());
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: large_demo_action_hex(),
                title: None,
                status: "approved".to_string(),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status: request.broadcast_status,
                commit_txid: request.commit_txid,
                reveal_txid: request.reveal_txid,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                cancel_proposal: None,
            })
        }
        async fn create_cancel_proposal(
            &self,
            _: &str,
            _: crate::application::orchestrator_client::CreateCancelProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn broadcast_commit_uses_commit_funding_abstraction() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let commit_txid = "spy-commit-txid-abc123";
        let spy = SpyCommitFunding::new(commit_txid);
        let mock_rpc = Arc::new(MockBtcRpc::new(commit_txid));
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let reveal_change_spk = ScriptBuf::new();
        let pending = crate::application::pending_reveals::new();

        let _result = broadcast_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            mock_rpc.as_ref(),
            "mock://asm-membership",
            magic_bytes,
            Network::Regtest,
            "action-1",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            10,
            5000,
            &spy,
            reveal_change_spk,
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(
            spy.was_called(),
            "CommitFunding::build_signed_commit must be called to fund the commit (Admin Wallet path)"
        );
        assert_eq!(
            mock_rpc.get_raw_transaction_call_count(),
            0,
            "get_raw_transaction must never be called in the new pre-sign flow"
        );
    }

    #[tokio::test]
    async fn submit_package_path_never_calls_send_raw_transaction() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let spy = SpyCommitFunding::new("ignored");
        let mock_rpc = Arc::new(MockBtcRpc::new("ignored")); // submit_package returns Ok(())
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();
        let broadcasters = node_broadcasters(Arc::clone(&mock_rpc));

        let result = broadcast_commit_then_reveal(
            &mock_client,
            &broadcasters,
            mock_rpc.as_ref(),
            "mock://asm-membership",
            magic_bytes,
            Network::Regtest,
            "action-submit-package",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            10,
            5000,
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(result.is_ok(), "broadcast should succeed: {result:?}");
        assert_eq!(
            mock_rpc.send_raw_transaction_call_count(),
            0,
            "send_raw_transaction must not be called when submit_package succeeds"
        );
    }

    #[tokio::test]
    async fn sequential_fallback_when_submit_package_returns_unknown_method() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let spy = SpyCommitFunding::new("ignored");
        let mock_rpc = Arc::new(MockBtcRpc::with_submit_package_error("Method not found"));
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();
        // NodeBroadcaster will use the MockBtcRpc — submit_package fails → sequential fallback
        let broadcasters = node_broadcasters(Arc::clone(&mock_rpc));

        let result = broadcast_commit_then_reveal(
            &mock_client,
            &broadcasters,
            mock_rpc.as_ref(),
            "mock://asm-membership",
            magic_bytes,
            Network::Regtest,
            "action-fallback",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            10,
            5000,
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(result.is_ok(), "fallback should succeed: {result:?}");
        assert_eq!(
            mock_rpc.send_raw_transaction_call_count(),
            2,
            "send_raw_transaction must be called exactly twice (commit then reveal) in sequential fallback"
        );
    }

    #[tokio::test]
    async fn pending_reveal_inserted_before_broadcast_and_removed_after_confirm() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let spy = SpyCommitFunding::new("ignored");
        let mock_rpc = Arc::new(MockBtcRpc::new("ignored"));
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();

        let result = broadcast_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            mock_rpc.as_ref(),
            "mock://asm-membership",
            magic_bytes,
            Network::Regtest,
            "action-pending-lifecycle",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            10,
            5000,
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(result.is_ok(), "broadcast should succeed: {result:?}");
        // After reveal_confirmed, the entry must be removed.
        assert!(
            pending
                .lock()
                .unwrap()
                .get("action-pending-lifecycle")
                .is_none(),
            "PendingReveals entry must be removed after reveal_confirmed"
        );
    }

    /// Regression (RCA: getnewaddress "wallet does not exist or is not loaded").
    ///
    /// The production broadcast path must NOT advance the chain and must NOT depend on a
    /// bitcoind Core wallet. Previously Step 8 called `mine_blocks(1)` → `getnewaddress`
    /// on `/wallet/asm-runner`, which fails whenever that wallet is not loaded. Mining is
    /// now delegated to the dev faucet/harness. This test proves the broadcast on regtest
    /// succeeds using ONLY node-level RPCs (the `mine_blocks`/`get_new_address` methods no
    /// longer exist on `BitcoinRpcClient`, so the regression is also compiler-enforced).
    #[tokio::test]
    async fn broadcast_on_regtest_does_not_mine_blocks() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let spy = SpyCommitFunding::new("ignored");
        let mock_rpc = Arc::new(MockBtcRpc::new("ignored"));
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();

        let result = broadcast_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            mock_rpc.as_ref(),
            "mock://asm-membership",
            magic_bytes,
            Network::Regtest,
            "action-reporting",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            10,
            5000,
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(
            result.is_ok(),
            "broadcast on regtest must succeed without mining / wallet RPC: {result:?}"
        );
    }

    // ─── submit / await split tests ──────────────────────────────────────────

    /// `submit_commit_then_reveal` returns after reporting `reveal_broadcasted` and never
    /// waits for or reports a confirmation. The PendingReveals entry must remain.
    #[tokio::test]
    async fn submit_returns_at_reveal_broadcasted_without_confirming() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let spy = SpyCommitFunding::new("ignored");
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();

        let result = submit_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            "mock://asm-membership",
            magic_bytes,
            Network::Regtest,
            "action-submit-only",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(result.is_ok(), "submit should succeed: {result:?}");
        let statuses = mock_client.reported_statuses();
        assert_eq!(statuses, vec!["commit_broadcasted", "reveal_broadcasted"]);
        assert!(
            !statuses
                .iter()
                .any(|s| s == "reveal_confirmed" || s == "failed"),
            "submit must not report reveal_confirmed or failed: {statuses:?}"
        );
        assert!(
            pending.lock().unwrap().get("action-submit-only").is_some(),
            "PendingReveals entry must be present after submit (awaiting confirmation)"
        );
    }

    /// REGRESSION (issue #382): the commit address shown in the broadcast preview must equal
    /// the commit address actually funded/signed. Before the shared `EnvelopeKeyCache`, the
    /// preview and the broadcast each minted their own random ephemeral keypair, so the address
    /// the signer confirmed on a hardware wallet never matched the app — defeating on-device
    /// verification. With one cache across both calls (same payload → same keypair), the two
    /// addresses are identical (modulo the HRP the device renders — see issue #401).
    #[tokio::test]
    async fn preview_and_broadcast_commit_address_match() {
        use crate::infrastructure::hw_wallet::hw_psbt_signer::HwDeviceType;
        use bitcoin::{Address, Network, ScriptBuf};
        use std::str::FromStr;
        use strata_l1_txfmt::MagicBytes;

        let cache = crate::infrastructure::admin_wallet::EnvelopeKeyCache::default();
        let mock_client = MockOrchestratorClientLargeAction::new();
        let fee_rate = crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000);

        // 1. Preview — what the UI shows as "COMMIT TX PREVIEW" (device-facing HRP).
        let (preview_address, _, _) = prepare_broadcast_bundle(
            &mock_client,
            "mock://asm-membership",
            Network::Regtest,
            "action-match",
            fee_rate,
            &cache,
            Some(HwDeviceType::Trezor),
        )
        .await
        .expect("preview ok");

        // 2. Broadcast — the spy captures the real regtest address actually funded/signed.
        let spy = SpyCommitFunding::new("ignored");
        let pending = crate::application::pending_reveals::new();
        submit_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            "mock://asm-membership",
            MagicBytes::new([0x62, 0x74, 0x00, 0x00]),
            Network::Regtest,
            "action-match",
            fee_rate,
            &spy,
            ScriptBuf::new(),
            &pending,
            &cache,
        )
        .await
        .expect("broadcast ok");

        let funded = spy.funded_commit_address().expect("commit was funded");
        let funded_addr = Address::from_str(&funded).unwrap().assume_checked();

        // The funded address is the real regtest (bcrt1) address; rendering it the way the
        // device would must reproduce the preview string exactly.
        assert_eq!(
            broadcast_tx::device_facing_commit_address(
                &funded_addr,
                Network::Regtest,
                Some(HwDeviceType::Trezor)
            ),
            preview_address,
            "preview and broadcast must derive the same commit address"
        );
        assert!(
            funded.starts_with("bcrt1p") && preview_address.starts_with("bc1p"),
            "sanity: funded is regtest, preview is the mainnet HRP a Trezor shows for coin 0'"
        );
    }

    /// A genuine submission error (all broadcasters fail) reports `failed`.
    #[tokio::test]
    async fn submit_reports_failed_on_real_submission_error() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let spy = SpyCommitFunding::new("ignored");
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();
        // Failing broadcaster simulates all broadcasters down
        let failing: Vec<std::sync::Arc<dyn crate::application::tx_broadcaster::TxBroadcaster>> =
            vec![std::sync::Arc::new(MockBroadcaster::failing(
                "mock",
                "node rejected",
            ))];

        let result = submit_commit_then_reveal(
            &mock_client,
            &failing,
            "mock://asm-membership",
            magic_bytes,
            Network::Regtest,
            "action-submit-error",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(matches!(
            result,
            Err(BroadcastError::AllBroadcastersFailed { .. })
        ));
        assert!(
            mock_client
                .reported_statuses()
                .iter()
                .any(|s| s == "failed"),
            "real submission error must report failed"
        );
    }

    /// `await_reveal_confirmation` with 1 confirmation reports `reveal_confirmed`, removes the
    /// pending entry, and returns `Confirmed`.
    #[tokio::test]
    async fn await_confirmation_confirms_and_removes_pending() {
        use crate::application::pending_reveals::{new as new_pending, PendingReveal};

        let mock_rpc = MockBtcRpc::new("ignored"); // 1 confirmation
        let mock_client = MockOrchestratorClientLargeAction::new();
        let pending = new_pending();
        pending.lock().unwrap().insert(
            "action-confirm".to_string(),
            PendingReveal {
                reveal_tx_hex: "deadbeef".to_string(),
                reveal_txid: "reveal-1".to_string(),
                commit_txid: "commit-1".to_string(),
            },
        );

        let outcome = await_reveal_confirmation(
            &mock_client,
            &mock_rpc,
            "action-confirm",
            "commit-1",
            "reveal-1",
            10,
            5000,
            &pending,
        )
        .await
        .expect("await ok");

        assert_eq!(outcome, ConfirmOutcome::Confirmed);
        assert_eq!(mock_client.reported_statuses(), vec!["reveal_confirmed"]);
        assert!(
            pending.lock().unwrap().get("action-confirm").is_none(),
            "pending entry must be removed after reveal_confirmed"
        );
    }

    /// `await_reveal_confirmation` that times out with 0 confirmations returns
    /// `PendingConfirmation`, reports NOTHING (no `failed`, no `reveal_confirmed`), and keeps
    /// the pending entry. A slow block must never become a false failure.
    #[tokio::test]
    async fn await_confirmation_timeout_stays_pending_without_failed() {
        use crate::application::pending_reveals::{new as new_pending, PendingReveal};

        let mock_rpc = MockBtcRpc::with_zero_confirmations();
        let mock_client = MockOrchestratorClientLargeAction::new();
        let pending = new_pending();
        pending.lock().unwrap().insert(
            "action-pending".to_string(),
            PendingReveal {
                reveal_tx_hex: "deadbeef".to_string(),
                reveal_txid: "reveal-2".to_string(),
                commit_txid: "commit-2".to_string(),
            },
        );

        let outcome = await_reveal_confirmation(
            &mock_client,
            &mock_rpc,
            "action-pending",
            "commit-2",
            "reveal-2",
            1,
            5, // tiny timeout → returns PendingConfirmation quickly
            &pending,
        )
        .await
        .expect("await ok");

        assert_eq!(outcome, ConfirmOutcome::PendingConfirmation);
        let statuses = mock_client.reported_statuses();
        assert!(
            !statuses
                .iter()
                .any(|s| s == "failed" || s == "reveal_confirmed"),
            "timeout must not report failed or reveal_confirmed: {statuses:?}"
        );
        assert!(
            pending.lock().unwrap().get("action-pending").is_some(),
            "pending entry must be retained on PendingConfirmation"
        );
    }

    /// The retained sequential wrapper must NOT report `failed` when confirmation times out.
    #[tokio::test]
    async fn broadcast_wrapper_timeout_does_not_report_failed() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let spy = SpyCommitFunding::new("ignored");
        let mock_rpc = Arc::new(MockBtcRpc::with_zero_confirmations());
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();

        let result = broadcast_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            mock_rpc.as_ref(),
            "mock://asm-membership",
            magic_bytes,
            Network::Regtest,
            "action-wrapper-timeout",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            1,
            5,
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(
            result.is_ok(),
            "wrapper should succeed (pending, not failed): {result:?}"
        );
        assert!(
            !mock_client
                .reported_statuses()
                .iter()
                .any(|s| s == "failed"),
            "confirmation timeout must not report failed"
        );
    }

    // ─── resubmit_reveal tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn resubmit_reveal_returns_no_pending_reveal_when_absent() {
        let pending = crate::application::pending_reveals::new();
        let mock_rpc = MockBtcRpc::new("ignored");
        let mock_client = MockOrchestratorClient::new();
        let result = resubmit_reveal(&pending, &mock_rpc, &mock_client, "action-missing").await;
        assert!(matches!(
            result,
            Err(BroadcastError::NoPendingReveal { .. })
        ));
    }

    #[tokio::test]
    async fn resubmit_reveal_broadcasts_stored_reveal_hex() {
        use crate::application::pending_reveals::{new as new_pending, PendingReveal};
        let pending = new_pending();
        pending.lock().unwrap().insert(
            "action-1".to_string(),
            PendingReveal {
                reveal_tx_hex: "deadbeef".to_string(),
                reveal_txid: "reveal-txid-123".to_string(),
                commit_txid: "commit-txid-456".to_string(),
            },
        );
        let mock_rpc = MockBtcRpc::new("action-1");
        let mock_client = MockOrchestratorClient::new();
        let result = resubmit_reveal(&pending, &mock_rpc, &mock_client, "action-1").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "reveal-txid-mock");
    }

    #[tokio::test]
    async fn reveal_confirmed_report_keeps_proposal_approved() {
        let mock = MockOrchestratorClient::new();
        super::report_broadcast(
            &mock,
            "action-1",
            "reveal_confirmed",
            None,
            Some("commit-txid"),
            Some("reveal-txid"),
            None,
        )
        .await
        .expect("report ok");

        let req = mock
            .last_report_request
            .lock()
            .unwrap()
            .clone()
            .expect("report captured");
        assert_eq!(req.broadcast_status, "reveal_confirmed");
        assert_eq!(req.proposal_status, None);
    }
}
