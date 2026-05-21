//! Persistence contracts defined by the application layer.
//!
//! Concrete implementations live in `crate::infrastructure`.

use crate::domain::authority::Authority;
use crate::domain::proposal::{ActionId, BroadcastStatus, Proposal, ProposalStatus};
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

    /// Atomically transition broadcast_status from `Idle` to `CommitBroadcasted`.
    ///
    /// Returns `Ok(proposal)` on success, `Err(Conflict)` if the proposal is not
    /// in the `Idle` state (i.e. another caller already claimed it).
    async fn claim_broadcast(&self, action_id: &ActionId) -> Result<Proposal, AppError>;

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

    /// Find the cancel proposal that targets the given action_id, if one exists.
    async fn find_cancel_for_target(
        &self,
        target: &ActionId,
    ) -> Result<Option<Proposal>, AppError>;

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
}
