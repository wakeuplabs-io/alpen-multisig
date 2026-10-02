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
use crate::application::tx_broadcaster::{
    broadcast_pair_with_fallback, broadcast_single_with_fallback, TxBroadcaster, TxOutcome,
};
use crate::application::tx_settle::{
    look_up, AbsenceTracker, AbsenceWindow, Presence, TrackedTx, TxLookup, Verdict,
};
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
    /// Any orchestrator call in the broadcast flow: fetching the proposal, claiming the
    /// coordination, or reporting progress afterwards. Named for the boundary rather than for one
    /// of its callers — it used to read "failed to fetch proposal", which sent a reader of the
    /// logs looking at the read path for a failure that happened while reporting.
    #[error("orchestrator request failed: {0}")]
    Orchestrator(#[from] OrchestratorError),
    #[error("broadcast setup error: {0}")]
    Setup(String),
    #[error("bitcoin RPC error: {0}")]
    BitcoinRpc(String),
    #[error("confirmation timeout for txid {txid}")]
    Timeout { txid: String },
    #[error("no pending reveal found for action_id: {action_id}")]
    NoPendingReveal { action_id: String },
    /// No broadcaster (Electrum + node) could even be reached: nothing was sent. Carries the
    /// raw tx hexes for manual copy-and-broadcast as an escape hatch (spec §8.3 M3); the
    /// commit's coins stay reserved for exactly that bundle (#516).
    #[error("all broadcasters failed: {errors:?}")]
    AllBroadcastersFailed {
        commit_tx_hex: String,
        reveal_tx_hex: String,
        errors: Vec<(String, String)>,
    },
    /// Every source that answered refused the commit and none may hold it (#516): its coins
    /// are free again and `failed` is reported, so the send can be retried from scratch.
    #[error("the commit was rejected: {errors:?}")]
    BroadcastRejected { errors: Vec<(String, String)> },
    /// No broadcaster confirmed or refused the commit, and one may hold it (#516). Its coins
    /// stay reserved, the signed reveal is kept, and `failed` is not reported: the bundle may be
    /// live, so a fresh send could fund a second commit.
    #[error("the commit {commit_txid} may have been broadcast: {errors:?}")]
    BroadcastUncertain {
        commit_txid: String,
        errors: Vec<(String, String)>,
    },
    /// The commit is on the network but its reveal was not accepted (#516). The commit is
    /// recorded in the wallet, the signed reveal kept for a resubmit, and `failed` is not
    /// reported (the commit is live).
    #[error(
        "the commit {commit_txid} is on the network but its reveal {reveal_txid} was not accepted: {errors:?}"
    )]
    RevealNotBroadcast {
        commit_txid: String,
        reveal_txid: String,
        errors: Vec<(String, String)>,
    },
    /// A bundle for the same manual proposal is still stored, so its commit may be live (#516):
    /// a second send would fund a second commit. Nothing was built.
    #[error("a bundle for this proposal may already be on the network (commit {commit_txid})")]
    BundleInFlight { commit_txid: String },
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

/// Attempts per progress report around the broadcast (#516). Small and bounded: the report is
/// idempotent. Giving up on the pre-registration aborts the send before anything is broadcast;
/// giving up after the broadcast leaves the row at `commit_broadcasted`, which already holds
/// both txids — never `failed`.
const REPORT_ATTEMPTS: u32 = 3;
const REPORT_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

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

/// Reports broadcast progress, retrying up to [`REPORT_ATTEMPTS`] times (#516).
async fn report_with_retry(
    client: &dyn OrchestratorClient,
    action_id: &str,
    broadcast_status: &str,
    commit_txid: &str,
    reveal_txid: Option<&str>,
) -> Result<(), BroadcastError> {
    let mut attempt = 1;
    loop {
        let reported = report_broadcast(
            client,
            action_id,
            broadcast_status,
            None,
            Some(commit_txid),
            reveal_txid,
            None,
        )
        .await;
        match reported {
            Err(e) if attempt < REPORT_ATTEMPTS => {
                tracing::warn!(action_id, broadcast_status, attempt, error = %e, "progress report failed; retrying");
                tokio::time::sleep(REPORT_RETRY_DELAY).await;
                attempt += 1;
            }
            other => return other,
        }
    }
}

/// A bundle broadcast that did not fully land: the error to return, and whether the proposal
/// may be reported `failed` — only when the commit is proven not to be live (#516).
struct BundleFailure {
    error: BroadcastError,
    report_failed: bool,
}

/// Broadcasts the bundle (spec §8: Electrum first, node fallback; the first source that lands
/// both wins) and settles the commit from the broadcasters' own answer (#516):
///
/// | commit outcome | coins | pending reveal | error | `failed` |
/// |---|---|---|---|---|
/// | accepted, reveal too | recorded (both) | kept | — | — |
/// | accepted, reveal not | commit recorded | kept | `RevealNotBroadcast` | no |
/// | rejected | released | removed | `BroadcastRejected` | yes |
/// | not delivered anywhere | reserved | kept | `AllBroadcastersFailed` (hexes) | no |
/// | ambiguous | reserved | kept | `BroadcastUncertain` | no |
async fn broadcast_bundle(
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    commit_funding: &dyn CommitFunding,
    pending: &PendingReveals,
    pending_key: &str,
    commit_tx: &bitcoin::Transaction,
    reveal_tx: &bitcoin::Transaction,
) -> Result<(), BundleFailure> {
    let commit_hex = broadcast_tx::tx_to_hex(commit_tx);
    let reveal_hex = broadcast_tx::tx_to_hex(reveal_tx);
    let Err(failure) = broadcast_pair_with_fallback(broadcasters, &commit_hex, &reveal_hex).await
    else {
        // On the network: the wallet keeps both txs even if no sync sees them (#516). The
        // reveal's change is what a CPFP bump spends.
        commit_funding.record_broadcast(commit_tx).await;
        commit_funding.record_broadcast(reveal_tx).await;
        return Ok(());
    };

    let commit_txid = commit_tx.compute_txid();
    let errors = failure.by_source();
    let (error, report_failed) = match failure.outcome {
        TxOutcome::Accepted => {
            commit_funding.record_broadcast(commit_tx).await;
            let error = BroadcastError::RevealNotBroadcast {
                commit_txid: commit_txid.to_string(),
                reveal_txid: reveal_tx.compute_txid().to_string(),
                errors,
            };
            (error, false)
        }
        TxOutcome::Rejected => {
            commit_funding.release(commit_txid);
            crate::infrastructure::pending_reveals_store::remove_and_persist(pending, pending_key);
            (BroadcastError::BroadcastRejected { errors }, true)
        }
        TxOutcome::NotDelivered => {
            let error = BroadcastError::AllBroadcastersFailed {
                commit_tx_hex: commit_hex,
                reveal_tx_hex: reveal_hex,
                errors,
            };
            (error, false)
        }
        TxOutcome::Ambiguous => {
            let error = BroadcastError::BroadcastUncertain {
                commit_txid: commit_txid.to_string(),
                errors,
            };
            (error, false)
        }
    };
    Err(BundleFailure {
        error,
        report_failed,
    })
}

/// Outcome of awaiting the reveal confirmation.
///
/// `Confirmed` means the reveal reached at least one confirmation and the orchestrator was
/// promoted to `reveal_confirmed`. `PendingConfirmation` means the wait timed out before the
/// bundle settled — the reveal may still be in the mempool and confirm later: no `failed` is
/// reported and the `PendingReveals` entry is retained, so the settle loop carries on. `Dropped`
/// means the bundle was absent from every source across the absence window and was settled as
/// `failed` (#516).
#[derive(Debug, PartialEq, Eq)]
pub enum ConfirmOutcome {
    /// Reveal reached >= 1 confirmation; orchestrator reported `reveal_confirmed`.
    Confirmed,
    /// Timed out unsettled; the bundle stays tracked (`reveal_broadcasted`).
    PendingConfirmation,
    /// The bundle left the network: coins released, `failed` reported.
    Dropped,
}

/// Pre-sign commit+reveal, store in PendingReveals, broadcast, and report up to
/// `reveal_broadcasted`. Returns `(commit_txid, reveal_txid)` **without** waiting for any
/// confirmation — the caller awaits confirmation separately (see [`await_reveal_confirmation`]).
///
/// Flow: claim → build_signed_commit → build_reveal_tx → drop keypair → insert pending →
/// pre-register both txids (`commit_broadcasted`) → broadcasters (Electrum first, node
/// fallback) → report reveal_broadcasted → return txids.
///
/// If the pre-registration fails, nothing is broadcast: the coins are released, the pending
/// reveal dropped and `failed` reported (#516). A commit every answering source rejected also
/// reports `failed`; a commit that may be live — accepted, ambiguous, or never delivered and
/// handed over for a manual broadcast — does not (see `broadcast_bundle`). Once the bundle is on
/// the network the call succeeds: a failing `reveal_broadcasted` report is retried, logged, and
/// never reported as `failed` — that would reopen the claim while the bundle is live — and the
/// txids are still returned, so the caller starts the confirmation watcher; the orchestrator
/// already holds both txids and can promote the row on its own. The broadcast NEVER advances the chain:
/// confirmation is driven by the dev faucet/harness on regtest and by real miners on
/// testnet/mainnet. `get_raw_transaction` is NEVER called.
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
            BroadcastError::Setup(claim_conflict_message(message))
        } else {
            BroadcastError::Orchestrator(e)
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

    // What the error path must undo (#516): a signed commit that never reached the broadcast
    // gives its reserved inputs back; a commit that may be live is never reported `failed`.
    let mut unbroadcast_commit: Option<bitcoin::Txid> = None;
    let mut report_failed = true;

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
        unbroadcast_commit = Some(commit_tx.compute_txid());

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
        let reveal_hex = broadcast_tx::tx_to_hex(&reveal_tx);

        // Step 5: Insert into PendingReveals BEFORE any broadcast.
        crate::infrastructure::pending_reveals_store::insert_and_persist(
            pending,
            action_id.to_string(),
            crate::application::pending_reveals::PendingReveal {
                reveal_tx_hex: reveal_hex.clone(),
                reveal_txid: reveal_txid.clone(),
                commit_txid: commit_txid.clone(),
                commit_tx_hex: Some(broadcast_tx::tx_to_hex(&commit_tx)),
            },
        );

        // Step 5b: Pre-register both txids with the orchestrator BEFORE any broadcast (#516).
        // Same status the claim set; the backend keeps the txids (COALESCE). A bundle the
        // orchestrator cannot track would be unrecoverable if the later reports never land, so
        // if this fails nothing is broadcast: the coins are released and `failed` reported by
        // the error path below.
        if let Err(e) = report_with_retry(
            client,
            action_id,
            "commit_broadcasted",
            &commit_txid,
            Some(&reveal_txid),
        )
        .await
        {
            crate::infrastructure::pending_reveals_store::remove_and_persist(pending, action_id);
            return Err(e);
        }

        // Step 6: Broadcast — Electrum first, node fallback. From here the broadcasters' answer
        // settles the commit's reservation and whether `failed` may be reported (#516).
        unbroadcast_commit = None;
        if let Err(failure) = broadcast_bundle(
            broadcasters,
            commit_funding,
            pending,
            action_id,
            &commit_tx,
            &reveal_tx,
        )
        .await
        {
            report_failed = failure.report_failed;
            return Err(failure.error);
        }

        // Step 7: Report reveal_broadcasted (no commit_confirmed). From here nothing fails the
        // call: `failed` would reopen the claim while the bundle is live, and an error would
        // skip the confirmation watcher. The txids are already registered (step 5b), so the
        // orchestrator can reconcile the row even if this report never lands.
        if let Err(e) = report_with_retry(
            client,
            action_id,
            "reveal_broadcasted",
            &commit_txid,
            Some(&reveal_txid),
        )
        .await
        {
            tracing::error!(action_id, error = %e, "bundle is on the network but its reveal_broadcasted report failed; the txids are already registered");
        }

        Ok((commit_txid, reveal_txid))
    }
    .await;

    if let Err(ref e) = broadcast_result {
        if let Some(txid) = unbroadcast_commit {
            commit_funding.release(txid);
        }
        if report_failed {
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
        } else {
            tracing::error!(action_id, error = %e, "broadcast failed but the commit may be live; not reporting failed");
        }
    }

    broadcast_result
}

/// What the settle rule needs to settle a stored bundle (#516).
pub struct BundleSettleContext<'a> {
    /// Every source that can say whether it holds a tx (Electrum, the node).
    pub lookups: &'a [std::sync::Arc<dyn TxLookup>],
    /// Used to resubmit a stored reveal.
    pub broadcasters: &'a [std::sync::Arc<dyn TxBroadcaster>],
    /// `None` when no orchestrator session is available: nothing is reported, and a bundle whose
    /// outcome must be reported stays tracked until it can be.
    pub orchestrator: Option<&'a dyn OrchestratorClient>,
    /// The session wallet, to record found txs and release a dropped commit's coins.
    pub funding: Option<&'a dyn CommitFunding>,
    pub pending: &'a PendingReveals,
}

/// Where one stored bundle stands after a settle check (#516).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleSettlement {
    /// The reveal is mined (or the orchestrator already closed the proposal): no longer tracked.
    Confirmed,
    /// On the network, reveal not mined yet — resubmitted if it was missing.
    Live,
    /// Gone from every source across the absence window: coins released, `failed` reported, no
    /// longer tracked.
    Dropped,
    /// Not decided this check.
    Open,
    /// No such stored bundle.
    NotTracked,
}

/// `broadcast_error` for a bundle the settle rule found gone (#516); the UI reads it to tell a
/// dropped bundle from one that was never sent.
pub const DROPPED_BROADCAST_ERROR: &str =
    "dropped: the bundle is no longer in the mempool or the chain";

/// The orchestrator row of a stored bundle, as far as the settle rule is concerned.
enum BundleRow {
    /// A `manual-<sighash>` bundle: no orchestrator row.
    Manual,
    /// The row still tracks this very bundle (same reveal txid).
    Ours(Box<Proposal>),
    /// The row moved on (another bundle, or none): only the coins are settled.
    NotOurs,
    /// No orchestrator session, or it could not be read: nothing can be reported this check.
    Unreachable,
}

async fn bundle_row(ctx: &BundleSettleContext<'_>, key: &str, reveal_txid: &str) -> BundleRow {
    if key.starts_with("manual-") {
        return BundleRow::Manual;
    }
    let Some(client) = ctx.orchestrator else {
        return BundleRow::Unreachable;
    };
    match client.get_proposal(key).await {
        Ok(row) if row.reveal_txid.as_deref() == Some(reveal_txid) => {
            BundleRow::Ours(Box::new(row))
        }
        Ok(_) => BundleRow::NotOurs,
        Err(e) => {
            tracing::warn!(action_id = key, error = %e, "settle: proposal row unreadable");
            BundleRow::Unreachable
        }
    }
}

/// The row has not caught up with a reveal that is on the network.
fn reveal_is_behind(row: &Proposal) -> bool {
    row.broadcast_status != "reveal_broadcasted" && row.broadcast_status != "reveal_confirmed"
}

fn decode_tx(hex_str: &str) -> Option<bitcoin::Transaction> {
    let bytes = hex::decode(hex_str).ok()?;
    bitcoin::consensus::deserialize(&bytes).ok()
}

/// Applies the settle rule (`tx_settle`) once to the bundle stored under `key` (#516):
///
/// | Reveal | Commit | Action |
/// |---|---|---|
/// | mined | — | record both; report `reveal_confirmed`; stop tracking |
/// | found | — | record both; report `reveal_broadcasted` if the row is behind |
/// | absent | found | record the commit; resubmit the stored reveal; report `reveal_broadcasted` |
/// | not found | gone (absent across the window) | release the commit's coins; report `failed` ("dropped"); stop tracking |
/// | anything else | | nothing |
///
/// Reports go only to a row that still carries this bundle's reveal txid, and only while the
/// proposal is approved; a row that is closed (`reveal_confirmed`, or no longer approved) ends
/// the tracking. A report that does not land keeps the bundle tracked, so the next check retries
/// it — never given up silently.
pub async fn settle_bundle(
    ctx: &BundleSettleContext<'_>,
    tracker: &mut AbsenceTracker,
    key: &str,
    now: std::time::Instant,
) -> BundleSettlement {
    let Some(stored) = ctx
        .pending
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(key)
        .cloned()
    else {
        return BundleSettlement::NotTracked;
    };
    let row = bundle_row(ctx, key, &stored.reveal_txid).await;
    if let BundleRow::Ours(proposal) = &row {
        if proposal.broadcast_status == "reveal_confirmed" || proposal.status != "approved" {
            crate::infrastructure::pending_reveals_store::remove_and_persist(ctx.pending, key);
            return BundleSettlement::Confirmed;
        }
    }
    let ours = match &row {
        BundleRow::Ours(proposal) => Some(proposal.as_ref()),
        _ => None,
    };
    let commit_tx = stored.commit_tx_hex.as_deref().and_then(decode_tx);
    let reveal_tx = decode_tx(&stored.reveal_tx_hex);
    let (Ok(commit_txid), Ok(reveal_txid)) = (
        stored.commit_txid.parse::<bitcoin::Txid>(),
        stored.reveal_txid.parse::<bitcoin::Txid>(),
    ) else {
        tracing::error!(action_id = key, "settle: stored bundle has malformed txids");
        return BundleSettlement::Open;
    };
    let report = |status: &'static str| report_bundle(ctx, key, &stored, status, None);
    let record = |tx: Option<bitcoin::Transaction>| async move {
        if let (Some(funding), Some(tx)) = (ctx.funding, tx) {
            funding.record_broadcast(&tx).await;
        }
    };

    let reveal = look_up(
        ctx.lookups,
        &TrackedTx {
            txid: reveal_txid,
            tx: reveal_tx.clone(),
        },
    )
    .await;
    if let Presence::Found { confirmed } = reveal {
        tracker.forget(&commit_txid);
        record(commit_tx).await;
        record(reveal_tx).await;
        if !confirmed {
            if ours.is_some_and(reveal_is_behind) {
                if let Err(e) = report("reveal_broadcasted").await {
                    tracing::warn!(action_id = key, error = %e, "settle: reveal_broadcasted report failed; retried next check");
                }
            }
            return BundleSettlement::Live;
        }
        if ours.is_some() {
            if let Err(e) = report("reveal_confirmed").await {
                tracing::warn!(action_id = key, error = %e, "settle: reveal_confirmed report failed; retried next check");
                return BundleSettlement::Live;
            }
        }
        crate::infrastructure::pending_reveals_store::remove_and_persist(ctx.pending, key);
        return BundleSettlement::Confirmed;
    }

    let commit = look_up(
        ctx.lookups,
        &TrackedTx {
            txid: commit_txid,
            tx: commit_tx.clone(),
        },
    )
    .await;
    match tracker.observe(commit_txid, commit, now) {
        Verdict::Found { .. } => {
            record(commit_tx).await;
            if reveal != Presence::Absent {
                return BundleSettlement::Live;
            }
            // The commit is live and every source answered that the reveal is not: send the
            // stored reveal again (no key needed; "already known" is a success).
            match resubmit_reveal(ctx.pending, ctx.broadcasters, key).await {
                Ok(_) => {
                    tracing::info!(action_id = key, "settle: missing reveal resubmitted");
                    record(reveal_tx).await;
                    if ours.is_some_and(reveal_is_behind) {
                        if let Err(e) = report("reveal_broadcasted").await {
                            tracing::warn!(action_id = key, error = %e, "settle: reveal_broadcasted report failed; retried next check");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(action_id = key, error = %e, "settle: reveal resubmit failed; retried next check");
                }
            }
            BundleSettlement::Live
        }
        Verdict::Gone => {
            // The commit is not, and will not be, on the network: its coins are free again.
            if let Some(funding) = ctx.funding {
                funding.release(commit_txid);
            }
            match &row {
                BundleRow::Unreachable => {
                    tracing::warn!(action_id = key, "settle: bundle dropped but the orchestrator is unreachable; `failed` is reported next check");
                    return BundleSettlement::Open;
                }
                BundleRow::Ours(proposal) if proposal.broadcast_status != "failed" => {
                    if let Err(e) = report_failed_with_retry(ctx, key, &stored).await {
                        tracing::warn!(action_id = key, error = %e, "settle: `failed` report did not land; retried next check");
                        return BundleSettlement::Open;
                    }
                }
                _ => {}
            }
            tracing::warn!(action_id = key, commit_txid = %commit_txid, "settle: bundle dropped from the network");
            tracker.forget(&commit_txid);
            crate::infrastructure::pending_reveals_store::remove_and_persist(ctx.pending, key);
            BundleSettlement::Dropped
        }
        Verdict::Open => BundleSettlement::Open,
    }
}

/// Reports `status` for a stored bundle, with both its txids.
async fn report_bundle(
    ctx: &BundleSettleContext<'_>,
    key: &str,
    stored: &crate::application::pending_reveals::PendingReveal,
    status: &str,
    broadcast_error: Option<&str>,
) -> Result<(), BroadcastError> {
    let client = ctx
        .orchestrator
        .ok_or_else(|| BroadcastError::Setup("no orchestrator session".to_string()))?;
    report_broadcast(
        client,
        key,
        status,
        None,
        Some(&stored.commit_txid),
        Some(&stored.reveal_txid),
        broadcast_error,
    )
    .await
}

/// Reports a dropped bundle as `failed`, retrying up to [`REPORT_ATTEMPTS`] times.
async fn report_failed_with_retry(
    ctx: &BundleSettleContext<'_>,
    key: &str,
    stored: &crate::application::pending_reveals::PendingReveal,
) -> Result<(), BroadcastError> {
    let mut attempt = 1;
    loop {
        let reported =
            report_bundle(ctx, key, stored, "failed", Some(DROPPED_BROADCAST_ERROR)).await;
        match reported {
            Err(e) if attempt < REPORT_ATTEMPTS => {
                tracing::warn!(action_id = key, attempt, error = %e, "`failed` report failed; retrying");
                tokio::time::sleep(REPORT_RETRY_DELAY).await;
                attempt += 1;
            }
            other => return other,
        }
    }
}

/// One settle pass over everything left open (#516): the session wallet's unsettled sends and fee
/// bumps, then every stored bundle. Run by the desktop's settle loop.
pub async fn settle_in_flight(
    ctx: &BundleSettleContext<'_>,
    wallet: Option<&crate::application::wallet_service::WalletService>,
    tracker: &mut AbsenceTracker,
    now: std::time::Instant,
) {
    if let Some(wallet) = wallet {
        wallet.settle_unsettled(ctx.lookups, tracker, now).await;
    }
    let keys: Vec<String> = ctx
        .pending
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .keys()
        .cloned()
        .collect();
    for key in keys {
        let settlement = settle_bundle(ctx, tracker, &key, now).await;
        tracing::debug!(action_id = %key, ?settlement, "settle pass");
    }
}

/// Watches a just-broadcast bundle until it settles, polling the settle rule every
/// `confirm_poll_interval_ms` (#516):
/// - reveal mined → `reveal_confirmed` reported, entry removed → [`ConfirmOutcome::Confirmed`];
/// - bundle gone across `window` → coins released, `failed` reported → [`ConfirmOutcome::Dropped`];
/// - a missing reveal whose commit is live is resubmitted on the way;
/// - a source that does not answer is just another open check — never an error;
/// - timeout → [`ConfirmOutcome::PendingConfirmation`]: nothing reported, entry retained, and the
///   settle loop carries on. A slow block is never a failure.
///
/// Intended to run in the background after [`submit_commit_then_reveal`] returns.
pub async fn await_reveal_confirmation(
    ctx: &BundleSettleContext<'_>,
    action_id: &str,
    window: AbsenceWindow,
    confirm_poll_interval_ms: u64,
    confirm_timeout_ms: u64,
) -> ConfirmOutcome {
    let start = std::time::Instant::now();
    let mut tracker = AbsenceTracker::new(window);
    loop {
        match settle_bundle(ctx, &mut tracker, action_id, std::time::Instant::now()).await {
            BundleSettlement::Confirmed => return ConfirmOutcome::Confirmed,
            BundleSettlement::Dropped => return ConfirmOutcome::Dropped,
            // Settled by someone else (the settle loop, another watcher).
            BundleSettlement::NotTracked => return ConfirmOutcome::PendingConfirmation,
            BundleSettlement::Live | BundleSettlement::Open => {}
        }
        if start.elapsed().as_millis() as u64 >= confirm_timeout_ms {
            return ConfirmOutcome::PendingConfirmation;
        }
        tokio::time::sleep(std::time::Duration::from_millis(confirm_poll_interval_ms)).await;
    }
}

/// Synchronous submit + await-confirmation composition (retained for tests and any sequential
/// caller). A confirmation timeout is **not** reported as `failed` — it leaves the proposal at
/// `reveal_broadcasted`.
#[allow(clippy::too_many_arguments)]
pub async fn broadcast_commit_then_reveal(
    client: &dyn OrchestratorClient,
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    lookups: &[std::sync::Arc<dyn TxLookup>],
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

    let ctx = BundleSettleContext {
        lookups,
        broadcasters,
        orchestrator: Some(client),
        funding: Some(commit_funding),
        pending,
    };
    await_reveal_confirmation(
        &ctx,
        action_id,
        AbsenceWindow::from_env(),
        confirm_poll_interval_ms,
        confirm_timeout_ms,
    )
    .await;

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

    // A stored bundle for this proposal may be live (#516): never replace it with a second one.
    let in_flight = pending
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&pending_key)
        .map(|stored| stored.commit_txid.clone());
    if let Some(commit_txid) = in_flight {
        return Err(BroadcastError::BundleInFlight { commit_txid });
    }

    // A signed commit that never reaches the broadcast gives its reserved inputs back (#516).
    let mut unbroadcast_commit: Option<bitcoin::Txid> = None;

    let broadcast_result: Result<(String, String), BroadcastError> = async {
        let commit_tx = commit_funding
            .build_signed_commit(
                &commit_address.to_string(),
                commit_amount_sats,
                fee_rate.to_bdk(),
            )
            .await
            .map_err(|e| BroadcastError::Setup(e.to_string()))?;
        unbroadcast_commit = Some(commit_tx.compute_txid());

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
        let reveal_hex = broadcast_tx::tx_to_hex(&reveal_tx);

        crate::infrastructure::pending_reveals_store::insert_and_persist(
            pending,
            pending_key.clone(),
            crate::application::pending_reveals::PendingReveal {
                reveal_tx_hex: reveal_hex.clone(),
                reveal_txid: reveal_txid.clone(),
                commit_txid: commit_txid.clone(),
                commit_tx_hex: Some(broadcast_tx::tx_to_hex(&commit_tx)),
            },
        );

        unbroadcast_commit = None;
        broadcast_bundle(
            broadcasters,
            commit_funding,
            pending,
            &pending_key,
            &commit_tx,
            &reveal_tx,
        )
        .await
        .map_err(|failure| failure.error)?;

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

    if let (Err(_), Some(txid)) = (&broadcast_result, unbroadcast_commit) {
        commit_funding.release(txid);
    }

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

/// Re-broadcast a stored reveal transaction for a given action_id (Electrum first, node
/// fallback). Idempotent: "already known" is a success. Returns the reveal txid.
///
/// Does NOT remove the entry — removal happens once the bundle settles.
pub async fn resubmit_reveal(
    pending: &PendingReveals,
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    action_id: &str,
) -> Result<String, BroadcastError> {
    let stored = pending
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(action_id)
        .cloned()
        .ok_or_else(|| BroadcastError::NoPendingReveal {
            action_id: action_id.to_string(),
        })?;
    broadcast_single_with_fallback(broadcasters, &stored.reveal_tx_hex)
        .await
        .map_err(|failure| BroadcastError::BitcoinRpc(failure.message()))?;
    Ok(stored.reveal_txid)
}

/// A 409 from the authority gate is one sentence. Any other 409 keeps its body:
/// "not approved" and "sequence already used" are different refusals.
fn claim_conflict_message(body: &str) -> String {
    const GATE: &str = "a broadcast for this authority is already in flight";
    if body.contains(GATE) {
        "A broadcast for this authority is already in flight.".to_string()
    } else {
        format!("broadcast already in progress: {body}")
    }
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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

    #[derive(Default)]
    struct SpyCommitFunding {
        build_signed_commit_called: Mutex<bool>,
        captured_commit_address: Mutex<Option<String>>,
        /// Txid of the commit handed back, so tests can match the reservation calls.
        built_txid: Mutex<Option<bitcoin::Txid>>,
        released: Mutex<Vec<bitcoin::Txid>>,
        recorded: Mutex<Vec<bitcoin::Txid>>,
        /// The signer refuses (e.g. rejected on device): no commit is produced.
        signer_rejects: bool,
        /// The commit pays somewhere else, so the reveal cannot be built on top of it.
        pays_elsewhere: bool,
    }

    impl SpyCommitFunding {
        fn new(_txid: &str) -> Self {
            Self::default()
        }

        fn rejecting() -> Self {
            Self {
                signer_rejects: true,
                ..Self::default()
            }
        }

        fn paying_elsewhere() -> Self {
            Self {
                pays_elsewhere: true,
                ..Self::default()
            }
        }

        fn built_txid(&self) -> bitcoin::Txid {
            self.built_txid.lock().unwrap().expect("a commit was built")
        }

        fn released(&self) -> Vec<bitcoin::Txid> {
            self.released.lock().unwrap().clone()
        }

        fn recorded(&self) -> Vec<bitcoin::Txid> {
            self.recorded.lock().unwrap().clone()
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
            if self.signer_rejects {
                return Err(
                    crate::application::commit_funding::CommitFundingError::AdminWallet(
                        "Request rejected on device".to_string(),
                    ),
                );
            }
            use bitcoin::{
                absolute::LockTime, transaction::Version, Address, Transaction, TxIn, TxOut,
            };
            use std::str::FromStr;
            // Parse the commit address to get the correct script_pubkey so build_reveal_tx can find the vout.
            let addr = Address::from_str(commit_address)
                .expect("valid commit address")
                .assume_checked();
            let script_pubkey = if self.pays_elsewhere {
                ScriptBuf::new_op_return([0u8; 4])
            } else {
                addr.script_pubkey()
            };
            let tx = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![TxIn::default()],
                output: vec![TxOut {
                    value: bitcoin::Amount::from_sat(10_000),
                    script_pubkey,
                }],
            };
            *self.built_txid.lock().unwrap() = Some(tx.compute_txid());
            Ok(tx)
        }

        fn release(&self, commit_txid: bitcoin::Txid) {
            self.released.lock().unwrap().push(commit_txid);
        }

        async fn record_broadcast(&self, tx: &bitcoin::Transaction) {
            self.recorded.lock().unwrap().push(tx.compute_txid());
        }
    }

    /// MockBitcoinRpcClient: configurable submit_package result and call counters.
    struct MockBtcRpc {
        submit_package_result: Result<(), crate::infrastructure::bitcoin_rpc::RpcError>,
        /// Answer of every `sendrawtransaction` when set (default: accepted).
        send_error: Option<crate::infrastructure::bitcoin_rpc::RpcError>,
        send_raw_transaction_call_count: Mutex<u32>,
        get_raw_transaction_call_count: Mutex<u32>,
        /// Confirmations returned by `get_transaction_confirmations` (default 1).
        confirmations: u32,
    }

    impl MockBtcRpc {
        fn new(_commit_txid: &str) -> Self {
            Self {
                submit_package_result: Ok(()),
                send_error: None,
                send_raw_transaction_call_count: Mutex::new(0),
                get_raw_transaction_call_count: Mutex::new(0),
                confirmations: 1,
            }
        }

        fn with_submit_package_error(err: &str) -> Self {
            Self {
                submit_package_result: Err(crate::infrastructure::bitcoin_rpc::RpcError::answered(
                    None, err,
                )),
                send_error: None,
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
        async fn send_raw_transaction(
            &self,
            _: &str,
        ) -> Result<String, crate::infrastructure::bitcoin_rpc::RpcError> {
            *self.send_raw_transaction_call_count.lock().unwrap() += 1;
            match &self.send_error {
                Some(e) => Err(e.clone()),
                None => Ok("reveal-txid-mock".to_string()),
            }
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

        async fn submit_package(
            &self,
            _: &[String],
        ) -> Result<(), crate::infrastructure::bitcoin_rpc::RpcError> {
            self.submit_package_result.clone()
        }

        async fn get_block_count(&self) -> Result<u64, String> {
            Ok(0)
        }

        async fn get_transaction_depth(
            &self,
            _: &str,
        ) -> Result<u32, crate::infrastructure::bitcoin_rpc::RpcError> {
            Ok(self.confirmations)
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
        /// Progress reports with these statuses are recorded and then refused.
        failing_statuses: Vec<&'static str>,
        /// Shared with a probe broadcaster to assert ordering across both ports.
        events: Option<std::sync::Arc<Mutex<Vec<String>>>>,
        /// `get_proposal` answers with this `(broadcast_status, reveal_txid)` when set.
        row: Option<(&'static str, String)>,
    }

    impl MockOrchestratorClientLargeAction {
        fn new() -> Self {
            Self::default()
        }

        /// Every progress report is refused (orchestrator unreachable).
        fn with_failing_reports() -> Self {
            Self::with_failing_report_of(&["commit_broadcasted", "reveal_broadcasted", "failed"])
        }

        fn with_failing_report_of(statuses: &[&'static str]) -> Self {
            Self {
                failing_statuses: statuses.to_vec(),
                ..Self::default()
            }
        }

        fn reports(
            &self,
        ) -> Vec<crate::application::orchestrator_client::ReportBroadcastProgressRequest> {
            self.reports.lock().unwrap().clone()
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
            let (broadcast_status, reveal_txid) = match &self.row {
                Some((status, reveal_txid)) => (status.to_string(), Some(reveal_txid.clone())),
                None => ("idle".to_string(), None),
            };
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: large_demo_action_hex(),
                title: None,
                status: "approved".to_string(),
                required_signatures: 2,
                signatures: vec![],
                broadcast_status,
                commit_txid: None,
                reveal_txid,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: 0,
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
            })
        }
        async fn report_broadcast_progress(
            &self,
            action_id: &str,
            request: crate::application::orchestrator_client::ReportBroadcastProgressRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            self.reports.lock().unwrap().push(request.clone());
            if let Some(events) = &self.events {
                events
                    .lock()
                    .unwrap()
                    .push(format!("report:{}", request.broadcast_status));
            }
            if self
                .failing_statuses
                .contains(&request.broadcast_status.as_str())
            {
                return Err(OrchestratorError::Backend {
                    status: 503,
                    message: "orchestrator unavailable".to_string(),
                });
            }
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
                updated_at: 0,
                expires_at: None,
                cancel_proposal: None,
                is_cancelable: false,
                broadcast_claim_stale: false,
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
            &mined_lookups(),
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
            &mined_lookups(),
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
            &mined_lookups(),
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
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();

        let result = broadcast_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            &mined_lookups(),
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
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();

        let result = broadcast_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            &mined_lookups(),
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

    /// Submits through a single `broadcaster`, with `pending` as the reveal store.
    async fn submit_through(
        client: &MockOrchestratorClientLargeAction,
        spy: &SpyCommitFunding,
        pending: &PendingReveals,
        broadcaster: MockBroadcaster,
    ) -> Result<(String, String), BroadcastError> {
        let chain: Vec<std::sync::Arc<dyn crate::application::tx_broadcaster::TxBroadcaster>> =
            vec![std::sync::Arc::new(broadcaster)];
        submit_commit_then_reveal(
            client,
            &chain,
            "mock://asm-membership",
            strata_l1_txfmt::MagicBytes::new([0x62, 0x74, 0x00, 0x00]),
            bitcoin::Network::Regtest,
            "action-outcome",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            spy,
            ScriptBuf::new(),
            pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await
    }

    /// What a failed bundle broadcast leaves behind, per commit outcome (#516).
    #[derive(Debug, PartialEq)]
    struct Aftermath {
        reported_failed: bool,
        released: bool,
        recorded_commit: bool,
        pending_kept: bool,
    }

    /// #516: the commit's outcome drives the proposal. Only an explicit rejection releases the
    /// coins, drops the signed reveal and reports `failed`; a commit that may be live keeps
    /// all three, and each case returns an error the UI can tell apart.
    #[tokio::test]
    async fn failed_bundle_is_settled_by_the_commit_outcome() {
        use crate::application::tx_broadcaster::FailureKind;

        let kept = |recorded_commit| Aftermath {
            reported_failed: false,
            released: false,
            recorded_commit,
            pending_kept: true,
        };
        type ErrorCheck = fn(&BroadcastError) -> bool;
        let cases: Vec<(MockBroadcaster, ErrorCheck, Aftermath)> = vec![
            (
                MockBroadcaster::failing("node", "bad-txns-inputs-missingorspent"),
                |e| matches!(e, BroadcastError::BroadcastRejected { .. }),
                Aftermath {
                    reported_failed: true,
                    released: true,
                    recorded_commit: false,
                    pending_kept: false,
                },
            ),
            (
                MockBroadcaster::unreachable("node"),
                |e| {
                    matches!(e, BroadcastError::AllBroadcastersFailed { commit_tx_hex, reveal_tx_hex, .. }
                        if !commit_tx_hex.is_empty() && !reveal_tx_hex.is_empty())
                },
                kept(false),
            ),
            (
                MockBroadcaster::ambiguous("node"),
                |e| matches!(e, BroadcastError::BroadcastUncertain { .. }),
                kept(false),
            ),
            (
                MockBroadcaster::landing_commit_then(
                    "node",
                    FailureKind::Rejected,
                    "reveal rejected",
                ),
                |e| matches!(e, BroadcastError::RevealNotBroadcast { .. }),
                kept(true),
            ),
        ];
        for (broadcaster, expected_error, expected) in cases {
            let spy = SpyCommitFunding::new("ignored");
            let client = MockOrchestratorClientLargeAction::new();
            let pending = crate::application::pending_reveals::new();

            let error = submit_through(&client, &spy, &pending, broadcaster)
                .await
                .unwrap_err();

            assert!(expected_error(&error), "got: {error:?}");
            let aftermath = Aftermath {
                reported_failed: client.reported_statuses().iter().any(|s| s == "failed"),
                released: spy.released() == vec![spy.built_txid()],
                recorded_commit: spy.recorded() == vec![spy.built_txid()],
                pending_kept: pending.lock().unwrap().contains_key("action-outcome"),
            };
            assert_eq!(aftermath, expected, "{error:?}");
        }
    }

    /// #516: the manual path has no orchestrator but the same rule — a rejected commit frees
    /// its coins and drops its `manual-<sighash>` reveal, so nothing offers to send it again.
    #[tokio::test]
    async fn manual_rejected_commit_releases_its_coins_and_drops_the_reveal() {
        let spy = SpyCommitFunding::new("ignored");
        let pending = crate::application::pending_reveals::new();
        let chain: Vec<std::sync::Arc<dyn crate::application::tx_broadcaster::TxBroadcaster>> =
            vec![std::sync::Arc::new(MockBroadcaster::failing(
                "node",
                "bad-txns-inputs-missingorspent",
            ))];

        let result = broadcast_manual(
            &chain,
            &MockBtcRpc::new("ignored"),
            "mock://asm-membership",
            strata_l1_txfmt::MagicBytes::new([0x62, 0x74, 0x00, 0x00]),
            bitcoin::Network::Regtest,
            &large_demo_action_hex(),
            1,
            "strata_admin",
            &[],
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            10,
            5_000,
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(
            matches!(result, Err(BroadcastError::BroadcastRejected { .. })),
            "got: {result:?}"
        );
        assert_eq!(spy.released(), vec![spy.built_txid()]);
        assert!(pending.lock().unwrap().is_empty(), "the reveal is dropped");
    }

    /// #516: a `manual-<sighash>` reveal still stored means that bundle's commit may be live —
    /// kept after an ambiguous or undelivered broadcast, or still waiting for its reveal. A second
    /// manual send of the same proposal would fund a second commit, so it is refused before
    /// anything is built, and the stored bundle is left as it was.
    #[tokio::test]
    async fn manual_send_is_refused_while_the_same_bundle_may_be_live() {
        use crate::application::pending_reveals::PendingReveal;

        let spy = SpyCommitFunding::new("ignored");
        let pending = crate::application::pending_reveals::new();
        let action_hex = large_demo_action_hex();
        let sighash = hex::encode(broadcast_tx::compute_sighash(1, &action_hex).unwrap());
        let key = format!("manual-{}", &sighash[..16]);
        let stored = PendingReveal {
            reveal_tx_hex: "aa".to_string(),
            reveal_txid: "reveal-live".to_string(),
            commit_txid: "commit-live".to_string(),
            commit_tx_hex: None,
        };
        pending.lock().unwrap().insert(key.clone(), stored);

        let result = broadcast_manual(
            &ok_broadcasters(),
            &MockBtcRpc::new("ignored"),
            "mock://asm-membership",
            strata_l1_txfmt::MagicBytes::new([0x62, 0x74, 0x00, 0x00]),
            bitcoin::Network::Regtest,
            &action_hex,
            1,
            "strata_admin",
            &[],
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            10,
            5_000,
            &spy,
            ScriptBuf::new(),
            &pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(
            matches!(&result, Err(BroadcastError::BundleInFlight { commit_txid }) if commit_txid == "commit-live"),
            "got: {result:?}"
        );
        assert!(!spy.was_called(), "nothing is built or signed");
        assert_eq!(
            pending
                .lock()
                .unwrap()
                .get(&key)
                .map(|p| p.reveal_txid.clone()),
            Some("reveal-live".to_string()),
            "the stored bundle is untouched"
        );
    }

    async fn submit_with(
        client: &MockOrchestratorClientLargeAction,
        funding: &SpyCommitFunding,
        action_id: &str,
    ) -> Result<(String, String), BroadcastError> {
        submit_commit_then_reveal(
            client,
            &ok_broadcasters(),
            "mock://asm-membership",
            strata_l1_txfmt::MagicBytes::new([0x62, 0x74, 0x00, 0x00]),
            bitcoin::Network::Regtest,
            action_id,
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            funding,
            ScriptBuf::new(),
            &crate::application::pending_reveals::new(),
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await
    }

    /// #516: once the bundle is on the network, reporting `failed` would reopen the claim
    /// while the bundle is live, and failing the call would skip the confirmation watcher —
    /// the only thing left that reports `reveal_confirmed`. A failing `reveal_broadcasted`
    /// report is retried, logged, and the call still succeeds; the orchestrator already holds
    /// both txids from the pre-registration, so it can reconcile the row on its own.
    #[tokio::test]
    async fn reveal_report_failure_after_broadcast_never_fails_the_send() {
        let spy = SpyCommitFunding::new("ignored");
        let client =
            MockOrchestratorClientLargeAction::with_failing_report_of(&["reveal_broadcasted"]);

        let result = submit_with(&client, &spy, "action-report-down").await;

        let (commit_txid, reveal_txid) =
            result.expect("the bundle is on the network: the send succeeded");
        let mut expected = vec!["commit_broadcasted"];
        expected.extend(vec!["reveal_broadcasted"; REPORT_ATTEMPTS as usize]);
        assert_eq!(
            client.reported_statuses(),
            expected,
            "the reveal report is retried a bounded number of times and `failed` is never sent"
        );
        let registered = &client.reports()[0];
        assert_eq!(
            (
                registered.commit_txid.as_deref(),
                registered.reveal_txid.as_deref()
            ),
            (Some(commit_txid.as_str()), Some(reveal_txid.as_str())),
            "the orchestrator holds both txids from before the broadcast"
        );
        assert!(spy.released().is_empty(), "the commit is on the network");
        assert_eq!(
            spy.recorded()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![commit_txid, reveal_txid],
            "both broadcast txs are recorded in the wallet so their coins stay spent"
        );
    }

    /// Broadcaster that only logs when it is called, next to the orchestrator's reports.
    struct ProbeBroadcaster {
        events: std::sync::Arc<Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl crate::application::tx_broadcaster::TxBroadcaster for ProbeBroadcaster {
        fn name(&self) -> &'static str {
            "probe"
        }
        async fn broadcast_pair(
            &self,
            _: &str,
            _: &str,
        ) -> Result<(), crate::application::tx_broadcaster::PairBroadcastError> {
            self.events.lock().unwrap().push("broadcast".to_string());
            Ok(())
        }
        async fn broadcast_one(
            &self,
            _: &str,
        ) -> Result<(), crate::application::tx_broadcaster::TxBroadcastError> {
            unimplemented!("pairs only")
        }
    }

    async fn submit_through_probe(
        client: &MockOrchestratorClientLargeAction,
        events: &std::sync::Arc<Mutex<Vec<String>>>,
        spy: &SpyCommitFunding,
        pending: &PendingReveals,
    ) -> Result<(String, String), BroadcastError> {
        let probe: Vec<std::sync::Arc<dyn crate::application::tx_broadcaster::TxBroadcaster>> =
            vec![std::sync::Arc::new(ProbeBroadcaster {
                events: std::sync::Arc::clone(events),
            })];
        submit_commit_then_reveal(
            client,
            &probe,
            "mock://asm-membership",
            strata_l1_txfmt::MagicBytes::new([0x62, 0x74, 0x00, 0x00]),
            bitcoin::Network::Regtest,
            "action-pre-register",
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            spy,
            ScriptBuf::new(),
            pending,
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await
    }

    /// #516: both txids reach the orchestrator before anything reaches the network, so a
    /// bundle whose later reports never land can still be reconciled from the orchestrator row.
    #[tokio::test]
    async fn both_txids_are_registered_before_any_broadcaster_is_called() {
        let events = std::sync::Arc::new(Mutex::new(Vec::new()));
        let client = MockOrchestratorClientLargeAction {
            events: Some(std::sync::Arc::clone(&events)),
            ..MockOrchestratorClientLargeAction::default()
        };
        let spy = SpyCommitFunding::new("ignored");

        submit_through_probe(
            &client,
            &events,
            &spy,
            &crate::application::pending_reveals::new(),
        )
        .await
        .expect("submit ok");

        assert_eq!(
            events.lock().unwrap().clone(),
            vec![
                "report:commit_broadcasted",
                "broadcast",
                "report:reveal_broadcasted"
            ]
        );
        let registered = &client.reports()[0];
        assert!(registered.commit_txid.is_some() && registered.reveal_txid.is_some());
    }

    /// #516: if the orchestrator never takes the txids, nothing is broadcast — a bundle it
    /// cannot track would be unrecoverable. The coins are released, the pending reveal dropped,
    /// and `failed` reported (best effort) so the claim reopens.
    #[tokio::test]
    async fn pre_registration_failure_aborts_before_any_broadcast() {
        let events = std::sync::Arc::new(Mutex::new(Vec::new()));
        let client = MockOrchestratorClientLargeAction {
            events: Some(std::sync::Arc::clone(&events)),
            ..MockOrchestratorClientLargeAction::with_failing_reports()
        };
        let spy = SpyCommitFunding::new("ignored");
        let pending = crate::application::pending_reveals::new();

        let result = submit_through_probe(&client, &events, &spy, &pending).await;

        assert!(
            matches!(result, Err(BroadcastError::Orchestrator(_))),
            "{result:?}"
        );
        assert!(
            !events.lock().unwrap().iter().any(|e| e == "broadcast"),
            "nothing may be broadcast: {:?}",
            events.lock().unwrap()
        );
        let mut expected = vec!["commit_broadcasted"; REPORT_ATTEMPTS as usize];
        expected.push("failed");
        assert_eq!(client.reported_statuses(), expected);
        assert_eq!(spy.released(), vec![spy.built_txid()]);
        assert!(
            pending.lock().unwrap().get("action-pre-register").is_none(),
            "the pending reveal of a bundle that was never sent is dropped"
        );
    }

    /// Before the broadcast nothing is on the network: a signer error still reports `failed`
    /// (so another signer can retry) and there is no commit to mark.
    #[tokio::test]
    async fn signer_error_before_broadcast_still_reports_failed() {
        let spy = SpyCommitFunding::rejecting();
        let client = MockOrchestratorClientLargeAction::new();

        let result = submit_with(&client, &spy, "action-signer-rejects").await;

        assert!(matches!(result, Err(BroadcastError::Setup(_))));
        assert_eq!(client.reported_statuses(), vec!["failed"]);
        assert!(spy.recorded().is_empty(), "nothing was broadcast");
    }

    /// A signed commit that fails before the broadcast (here: its reveal cannot be built)
    /// hands its inputs back and reports `failed`.
    #[tokio::test]
    async fn setup_error_after_the_commit_is_signed_releases_its_inputs() {
        let spy = SpyCommitFunding::paying_elsewhere();
        let client = MockOrchestratorClientLargeAction::new();

        let result = submit_with(&client, &spy, "action-reveal-fails").await;

        assert!(
            matches!(result, Err(BroadcastError::Setup(_))),
            "got: {result:?}"
        );
        assert_eq!(spy.released(), vec![spy.built_txid()]);
        assert!(spy.recorded().is_empty(), "nothing was broadcast");
        assert_eq!(client.reported_statuses(), vec!["failed"]);
    }

    /// Manual broadcast has no orchestrator, but the same reservation rule: a signed commit
    /// that never reaches the broadcast is released.
    #[tokio::test]
    async fn manual_setup_error_after_the_commit_is_signed_releases_its_inputs() {
        let spy = SpyCommitFunding::paying_elsewhere();
        let rpc = MockBtcRpc::new("ignored");

        let result = broadcast_manual(
            &ok_broadcasters(),
            &rpc,
            "mock://asm-membership",
            strata_l1_txfmt::MagicBytes::new([0x62, 0x74, 0x00, 0x00]),
            bitcoin::Network::Regtest,
            &large_demo_action_hex(),
            1,
            "strata_admin",
            &[],
            crate::domain::fee_rate::FeeRate::from_raw_clamped(1_000),
            10,
            5_000,
            &spy,
            ScriptBuf::new(),
            &crate::application::pending_reveals::new(),
            &crate::infrastructure::admin_wallet::EnvelopeKeyCache::default(),
        )
        .await;

        assert!(
            matches!(result, Err(BroadcastError::Setup(_))),
            "got: {result:?}"
        );
        assert_eq!(spy.released(), vec![spy.built_txid()]);
        assert!(spy.recorded().is_empty(), "nothing was broadcast");
    }

    // ─── settle rule (#516) ─────────────────────────────────────────────────

    use crate::application::tx_settle::tests::StubLookup;
    use crate::application::tx_settle::Lookup;

    const MINED: Lookup = Lookup::Found { confirmed: true };
    const IN_MEMPOOL: Lookup = Lookup::Found { confirmed: false };

    fn stub_lookups(answer: Lookup) -> Vec<std::sync::Arc<dyn TxLookup>> {
        vec![std::sync::Arc::new(StubLookup::always(answer))]
    }

    /// Every source holds both txs, mined.
    fn mined_lookups() -> Vec<std::sync::Arc<dyn TxLookup>> {
        stub_lookups(MINED)
    }

    /// Every source holds both txs, unconfirmed.
    fn mempool_lookups() -> Vec<std::sync::Arc<dyn TxLookup>> {
        stub_lookups(IN_MEMPOOL)
    }

    /// A signed-looking commit and the reveal that spends it, stored as `PendingReveals` holds
    /// them. Returns `(stored, commit_txid, reveal_txid)`.
    fn stored_bundle() -> (
        crate::application::pending_reveals::PendingReveal,
        bitcoin::Txid,
        bitcoin::Txid,
    ) {
        use bitcoin::{
            absolute::LockTime, transaction::Version, OutPoint, Transaction, TxIn, TxOut,
        };
        let output = |sats| TxOut {
            value: bitcoin::Amount::from_sat(sats),
            script_pubkey: ScriptBuf::from_bytes(vec![0x51, 0x20, 0x07]),
        };
        let commit = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn::default()],
            output: vec![output(10_000)],
        };
        let reveal = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::new(commit.compute_txid(), 0),
                ..TxIn::default()
            }],
            output: vec![output(9_000)],
        };
        let stored = crate::application::pending_reveals::PendingReveal {
            reveal_tx_hex: broadcast_tx::tx_to_hex(&reveal),
            reveal_txid: reveal.compute_txid().to_string(),
            commit_txid: commit.compute_txid().to_string(),
            commit_tx_hex: Some(broadcast_tx::tx_to_hex(&commit)),
        };
        (stored, commit.compute_txid(), reveal.compute_txid())
    }

    /// What the orchestrator knows about the stored bundle in a settle case.
    #[derive(Clone, Copy, Debug)]
    enum RowFixture {
        /// The row carries this bundle's reveal txid, at this broadcast status.
        Ours(&'static str),
        /// The row carries another bundle (a retry by another signer).
        NotOurs,
        /// No orchestrator session.
        Unreachable,
    }

    /// One settle case: each source's answer for `(reveal, commit)`, the minutes of the checks,
    /// the orchestrator row, and what the last check must leave behind.
    struct SettleCase {
        sources: Vec<(Lookup, Lookup)>,
        minutes: Vec<u64>,
        row: RowFixture,
        settlement: BundleSettlement,
        reported: Vec<&'static str>,
        resubmitted: bool,
        released: bool,
        recorded: usize,
        tracked: bool,
    }

    /// #516: the settle rule, bundle state by bundle state.
    #[tokio::test]
    async fn settle_rule_decides_each_stored_bundle() {
        use Lookup::{NotFound, Unanswered};
        let ours = RowFixture::Ours;
        let case = |sources, minutes, row, settlement, reported: Vec<&'static str>| SettleCase {
            sources,
            minutes,
            row,
            settlement,
            reported,
            resubmitted: false,
            released: false,
            recorded: 0,
            tracked: true,
        };
        let cases = vec![
            // Reveal mined: reported and no longer tracked.
            SettleCase {
                recorded: 2,
                tracked: false,
                ..case(
                    vec![(MINED, MINED)],
                    vec![0],
                    ours("reveal_broadcasted"),
                    BundleSettlement::Confirmed,
                    vec!["reveal_confirmed"],
                )
            },
            // Reveal in the mempool: a row still behind catches up; one that is not is left alone.
            SettleCase {
                recorded: 2,
                ..case(
                    vec![(IN_MEMPOOL, IN_MEMPOOL)],
                    vec![0],
                    ours("commit_broadcasted"),
                    BundleSettlement::Live,
                    vec!["reveal_broadcasted"],
                )
            },
            SettleCase {
                recorded: 2,
                ..case(
                    vec![(IN_MEMPOOL, IN_MEMPOOL)],
                    vec![0],
                    ours("reveal_broadcasted"),
                    BundleSettlement::Live,
                    vec![],
                )
            },
            // A mistaken `failed` whose bundle is live is set right.
            SettleCase {
                recorded: 2,
                ..case(
                    vec![(IN_MEMPOOL, IN_MEMPOOL)],
                    vec![0],
                    ours("failed"),
                    BundleSettlement::Live,
                    vec!["reveal_broadcasted"],
                )
            },
            // Commit live, reveal absent everywhere: the stored reveal is sent again.
            SettleCase {
                resubmitted: true,
                recorded: 2,
                ..case(
                    vec![(NotFound, IN_MEMPOOL)],
                    vec![0],
                    ours("commit_broadcasted"),
                    BundleSettlement::Live,
                    vec!["reveal_broadcasted"],
                )
            },
            // ...but not on a guess: one source that did not answer about the reveal is enough to wait.
            SettleCase {
                recorded: 1,
                ..case(
                    vec![(NotFound, IN_MEMPOOL), (Unanswered, Unanswered)],
                    vec![0],
                    ours("commit_broadcasted"),
                    BundleSettlement::Live,
                    vec![],
                )
            },
            // Absent everywhere across the whole window: dropped.
            SettleCase {
                released: true,
                tracked: false,
                ..case(
                    vec![(NotFound, NotFound)],
                    vec![0, 5, 10],
                    ours("reveal_broadcasted"),
                    BundleSettlement::Dropped,
                    vec!["failed"],
                )
            },
            // Not yet the whole window.
            case(
                vec![(NotFound, NotFound)],
                vec![0, 5],
                ours("reveal_broadcasted"),
                BundleSettlement::Open,
                vec![],
            ),
            // A source that never answers keeps it open, however long.
            case(
                vec![(NotFound, NotFound), (Unanswered, Unanswered)],
                vec![0, 5, 10, 15],
                ours("reveal_broadcasted"),
                BundleSettlement::Open,
                vec![],
            ),
            // A row that moved on to another bundle: coins settled, nothing reported.
            SettleCase {
                released: true,
                tracked: false,
                ..case(
                    vec![(NotFound, NotFound)],
                    vec![0, 5, 10],
                    RowFixture::NotOurs,
                    BundleSettlement::Dropped,
                    vec![],
                )
            },
            // No orchestrator: the coins come back, but the bundle stays tracked until `failed` lands.
            SettleCase {
                released: true,
                ..case(
                    vec![(NotFound, NotFound)],
                    vec![0, 5, 10],
                    RowFixture::Unreachable,
                    BundleSettlement::Open,
                    vec![],
                )
            },
        ];

        for c in cases {
            let (stored, commit_txid, reveal_txid) = stored_bundle();
            let pending = crate::application::pending_reveals::new();
            pending
                .lock()
                .unwrap()
                .insert("action-settle".to_string(), stored.clone());
            let lookups: Vec<std::sync::Arc<dyn TxLookup>> = c
                .sources
                .iter()
                .map(|(reveal, commit)| {
                    let stub = StubLookup::always(Lookup::Unanswered);
                    stub.answer(reveal_txid, *reveal);
                    stub.answer(commit_txid, *commit);
                    std::sync::Arc::new(stub) as std::sync::Arc<dyn TxLookup>
                })
                .collect();
            let broadcaster = std::sync::Arc::new(MockBroadcaster::ok("node"));
            let broadcasters: Vec<std::sync::Arc<dyn TxBroadcaster>> = vec![broadcaster.clone()];
            let client = MockOrchestratorClientLargeAction {
                row: Some(match c.row {
                    RowFixture::Ours(status) => (status, stored.reveal_txid.clone()),
                    _ => ("reveal_broadcasted", "another-bundle".to_string()),
                }),
                ..MockOrchestratorClientLargeAction::default()
            };
            let spy = SpyCommitFunding::new("ignored");
            let ctx = BundleSettleContext {
                lookups: &lookups,
                broadcasters: &broadcasters,
                orchestrator: match c.row {
                    RowFixture::Unreachable => None,
                    _ => Some(&client),
                },
                funding: Some(&spy),
                pending: &pending,
            };
            let mut tracker = AbsenceTracker::new(AbsenceWindow::DEFAULT);
            let start = std::time::Instant::now();

            let mut settlement = BundleSettlement::NotTracked;
            for minute in &c.minutes {
                let at = start + std::time::Duration::from_secs(60 * minute);
                settlement = settle_bundle(&ctx, &mut tracker, "action-settle", at).await;
            }

            let label = format!("{:?} / {:?} at {:?}", c.sources, c.row, c.minutes);
            assert_eq!(settlement, c.settlement, "{label}");
            assert_eq!(client.reported_statuses(), c.reported, "{label}");
            if c.reported == vec!["failed"] {
                let report = &client.reports()[0];
                assert_eq!(
                    report.broadcast_error.as_deref(),
                    Some(DROPPED_BROADCAST_ERROR)
                );
                assert_eq!(
                    report.reveal_txid.as_deref(),
                    Some(stored.reveal_txid.as_str())
                );
            }
            assert_eq!(
                broadcaster.sent_single() == vec![stored.reveal_tx_hex.clone()],
                c.resubmitted,
                "{label}"
            );
            assert_eq!(spy.released() == vec![commit_txid], c.released, "{label}");
            assert_eq!(spy.recorded().len(), c.recorded, "{label}");
            assert_eq!(
                pending.lock().unwrap().contains_key("action-settle"),
                c.tracked,
                "{label}"
            );
        }
    }

    /// #516: a dropped bundle's `failed` report that does not land is never given up: the bundle
    /// stays tracked, and the next check reports it again.
    #[tokio::test]
    async fn dropped_bundle_failed_report_is_retried_on_the_next_check() {
        let (stored, _, _) = stored_bundle();
        let pending = crate::application::pending_reveals::new();
        pending
            .lock()
            .unwrap()
            .insert("action-retry".to_string(), stored.clone());
        let lookups = stub_lookups(Lookup::NotFound);
        let row = Some(("reveal_broadcasted", stored.reveal_txid.clone()));
        let down = MockOrchestratorClientLargeAction {
            row: row.clone(),
            ..MockOrchestratorClientLargeAction::with_failing_report_of(&["failed"])
        };
        let up = MockOrchestratorClientLargeAction {
            row,
            ..MockOrchestratorClientLargeAction::default()
        };
        let spy = SpyCommitFunding::new("ignored");
        let mut tracker = AbsenceTracker::new(AbsenceWindow::DEFAULT);
        let start = std::time::Instant::now();
        let minutes = |m: u64| start + std::time::Duration::from_secs(60 * m);
        let broadcasters = ok_broadcasters();
        let ctx_down = BundleSettleContext {
            lookups: &lookups,
            broadcasters: &broadcasters,
            orchestrator: Some(&down),
            funding: Some(&spy),
            pending: &pending,
        };
        let ctx_up = BundleSettleContext {
            orchestrator: Some(&up),
            ..ctx_down
        };

        for m in [0, 5, 10] {
            let settled = settle_bundle(&ctx_down, &mut tracker, "action-retry", minutes(m)).await;
            assert_eq!(settled, BundleSettlement::Open);
        }
        assert_eq!(
            down.reported_statuses(),
            vec!["failed"; REPORT_ATTEMPTS as usize],
            "retried within the check"
        );
        assert!(pending.lock().unwrap().contains_key("action-retry"));

        let settled = settle_bundle(&ctx_up, &mut tracker, "action-retry", minutes(11)).await;
        assert_eq!(settled, BundleSettlement::Dropped);
        assert_eq!(up.reported_statuses(), vec!["failed"]);
        assert!(pending.lock().unwrap().is_empty());
    }

    /// #516: the watcher started after a broadcast applies the same rule until the bundle
    /// settles or the timeout hands over to the settle loop. A source that does not answer is
    /// never an error, and a timeout never reports `failed`.
    #[tokio::test]
    async fn reveal_watcher_stops_when_the_bundle_settles() {
        let short_window = AbsenceWindow {
            checks: 1,
            span: std::time::Duration::ZERO,
        };
        let cases = [
            (
                MINED,
                ConfirmOutcome::Confirmed,
                vec!["reveal_confirmed"],
                false,
            ),
            (
                IN_MEMPOOL,
                ConfirmOutcome::PendingConfirmation,
                vec![],
                true,
            ),
            (
                Lookup::Unanswered,
                ConfirmOutcome::PendingConfirmation,
                vec![],
                true,
            ),
            (
                Lookup::NotFound,
                ConfirmOutcome::Dropped,
                vec!["failed"],
                false,
            ),
        ];
        for (answer, expected, reported, tracked) in cases {
            let (stored, _, _) = stored_bundle();
            let pending = crate::application::pending_reveals::new();
            pending
                .lock()
                .unwrap()
                .insert("action-watch".to_string(), stored.clone());
            let lookups = stub_lookups(answer);
            let broadcasters = ok_broadcasters();
            let client = MockOrchestratorClientLargeAction {
                row: Some(("reveal_broadcasted", stored.reveal_txid.clone())),
                ..MockOrchestratorClientLargeAction::default()
            };
            let spy = SpyCommitFunding::new("ignored");
            let ctx = BundleSettleContext {
                lookups: &lookups,
                broadcasters: &broadcasters,
                orchestrator: Some(&client),
                funding: Some(&spy),
                pending: &pending,
            };

            let outcome = await_reveal_confirmation(&ctx, "action-watch", short_window, 1, 5).await;

            assert_eq!(outcome, expected, "{answer:?}");
            assert_eq!(client.reported_statuses(), reported, "{answer:?}");
            assert_eq!(
                pending.lock().unwrap().contains_key("action-watch"),
                tracked,
                "{answer:?}"
            );
        }
    }

    /// The retained sequential wrapper must NOT report `failed` when confirmation times out.
    #[tokio::test]
    async fn broadcast_wrapper_timeout_does_not_report_failed() {
        use bitcoin::{Network, ScriptBuf};
        use strata_l1_txfmt::MagicBytes;

        let spy = SpyCommitFunding::new("ignored");
        let mock_client = MockOrchestratorClientLargeAction::new();
        let magic_bytes = MagicBytes::new([0x62, 0x74, 0x00, 0x00]);
        let pending = crate::application::pending_reveals::new();

        let result = broadcast_commit_then_reveal(
            &mock_client,
            &ok_broadcasters(),
            &mempool_lookups(),
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
        let result = resubmit_reveal(&pending, &ok_broadcasters(), "action-missing").await;
        assert!(matches!(
            result,
            Err(BroadcastError::NoPendingReveal { .. })
        ));
    }

    /// #516: resubmitting a stored reveal needs no key and is idempotent — a node that already
    /// holds it ("already in mempool", -27) is a success, and the stored reveal's txid is returned.
    #[tokio::test]
    async fn resubmit_reveal_sends_the_stored_hex_and_already_known_is_success() {
        let (stored, _, reveal_txid) = stored_bundle();
        let pending = crate::application::pending_reveals::new();
        pending
            .lock()
            .unwrap()
            .insert("action-1".to_string(), stored.clone());
        let rpc = Arc::new(MockBtcRpc {
            send_error: Some(crate::infrastructure::bitcoin_rpc::RpcError::answered(
                Some(-27),
                "txn-already-in-mempool",
            )),
            ..MockBtcRpc::new("ignored")
        });

        let resubmitted = resubmit_reveal(&pending, &node_broadcasters(rpc.clone()), "action-1")
            .await
            .expect("already known is a success");

        assert_eq!(resubmitted, reveal_txid.to_string());
        assert_eq!(rpc.send_raw_transaction_call_count(), 1);
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

    #[test]
    fn claim_conflict_message_hides_the_json_only_for_the_authority_gate() {
        let gate = r#"{"error":"conflict: a broadcast for this authority is already in flight","errorCode":"conflict"}"#;
        assert_eq!(
            super::claim_conflict_message(gate),
            "A broadcast for this authority is already in flight."
        );
        let other =
            r#"{"error":"conflict: proposal must be in 'approved' state","errorCode":"conflict"}"#;
        assert!(super::claim_conflict_message(other).contains("approved"));
    }
}
