//! Persistence contracts defined by the application layer.
//!
//! Concrete implementations live in `crate::infrastructure`.

use crate::domain::authority::Authority;
use crate::domain::proposal::{
    ActionId, BroadcastAttempt, BroadcastStatus, Proposal, ProposalStatus,
};
use crate::error::AppError;

/// Persistence contract for proposals.
#[async_trait::async_trait]
pub(crate) trait ProposalRepository: Send + Sync {
    /// Store a new proposal. Fails if ActionId already exists.
    async fn save_proposal(&self, proposal: Proposal) -> Result<(), AppError>;

    /// Find a proposal by ActionId.
    async fn find_by_action_id(&self, action_id: &ActionId) -> Result<Option<Proposal>, AppError>;

    /// Append one signature to an existing proposal.
    async fn add_signature(
        &self,
        action_id: &ActionId,
        signer_pubkey: &str,
        signature_hex: &str,
    ) -> Result<Option<Proposal>, AppError>;

    /// List proposals for one authority, optionally filtered by status.
    async fn list_by_status(
        &self,
        authority: Authority,
        status: Option<ProposalStatus>,
    ) -> Result<Vec<Proposal>, AppError>;

    /// Take broadcast coordination for `action_id`.
    ///
    /// Succeeds from `idle` or `failed`, and from a `commit_broadcasted` row that
    /// has no txids and a claim older than `CLAIM_STALE_AFTER`. Refuses when
    /// another approved proposal of the same authority is already in flight.
    /// A proposal that is no longer approved does not count.
    /// `Err(Conflict)` carries `AUTHORITY_IN_FLIGHT`.
    async fn claim_broadcast(&self, action_id: &ActionId) -> Result<Proposal, AppError>;

    /// Register another signer's pair beside the one in flight and make it the pair the row
    /// tracks (`commit_broadcasted`, claim time now, error cleared) (#605).
    ///
    /// Decided under the repository's lock with `decide_additional_pair`; a refusal is
    /// `Err(Conflict)` carrying its text. `abandoned` names this signer's earlier pairs the node
    /// finds nowhere. Never touches another proposal: the authority's in-flight bundle is already
    /// this one.
    async fn register_additional_pair(
        &self,
        action_id: &ActionId,
        attempt: BroadcastAttempt,
        abandoned: &[String],
    ) -> Result<Proposal, AppError>;

    /// Update broadcast sub-status and related txids/error for an approved proposal.
    async fn update_broadcast_status(
        &self,
        action_id: &ActionId,
        status: BroadcastStatus,
        proposal_status: Option<ProposalStatus>,
        commit_txid: Option<&str>,
        reveal_txid: Option<&str>,
        error: Option<&str>,
    ) -> Result<Option<Proposal>, AppError>;

    /// `update_broadcast_status` for a desktop's report about its own pair (#605): applied only
    /// while the row is untagged or still tracks the reported reveal. A report that names no
    /// reveal does not touch a tagged row. Decided in the write itself, so a pair registered
    /// after the report read the row is never overwritten; otherwise the row comes back as it is.
    ///
    /// `attempt` is recorded in that same write, and only when the update applies. A report that
    /// loses the race is not listed, so a reveal from another action cannot later confirm this one.
    ///
    /// `live` lists the reveals of pairs the node still has; a `failed` report for the tracked
    /// pair puts the row back on the newest of them (see `broadcast_write_for_report`).
    #[allow(clippy::too_many_arguments)]
    async fn update_reported_pair_status(
        &self,
        action_id: &ActionId,
        status: BroadcastStatus,
        proposal_status: Option<ProposalStatus>,
        commit_txid: Option<&str>,
        reveal_txid: Option<&str>,
        error: Option<&str>,
        attempt: Option<BroadcastAttempt>,
        live: &[String],
    ) -> Result<Option<Proposal>, AppError>;

    /// Mark the proposal `reveal_confirmed` on this commit + reveal, but only while it still
    /// tracks `observed_reveal` and the broadcast is still unconfirmed (#605).
    ///
    /// `Ok(None)` when the row is gone or has moved on: the next reconcile reads it again and
    /// decides from what it finds. The winner's txids are written outright, never left to COALESCE.
    async fn confirm_tracked_reveal(
        &self,
        action_id: &ActionId,
        observed_reveal: Option<&str>,
        commit_txid: &str,
        reveal_txid: &str,
    ) -> Result<Option<Proposal>, AppError>;

    /// Find the cancel proposal that targets the given action_id, if one exists.
    async fn find_cancel_for_target(&self, target: &ActionId)
        -> Result<Option<Proposal>, AppError>;

    /// Persist the computed activation height on a proposal row.
    #[allow(dead_code)]
    async fn update_activation_height(
        &self,
        action_id: &ActionId,
        height: u64,
    ) -> Result<(), AppError>;

    /// Persist the ASM queue UpdateId on a proposal row. Set after RevealConfirmed.
    #[allow(dead_code)]
    async fn update_update_id_in_queue(
        &self,
        action_id: &ActionId,
        update_id: u32,
    ) -> Result<(), AppError>;

    /// Atomically mark a cancel proposal as `Enacted` and its target as `Canceled`,
    /// but only if the target is still `Approved`.
    ///
    /// Returns `Ok(true)` if the target was `Approved` and both transitions were applied.
    /// Returns `Ok(false)` without mutating anything if the target was no longer `Approved`
    /// (e.g. it was already `Enacted`, `Canceled`, or `Expired` via another path) — in that
    /// case the ASM would have rejected the cancel tx, so the cancel proposal did not actually
    /// take effect and the caller must decide how to reflect that.
    async fn enact_cancel(
        &self,
        cancel_action_id: &ActionId,
        target_action_id: &ActionId,
    ) -> Result<bool, AppError>;
}
