//! Application layer — business logic for auth, proposals, and signatures.
//!
//! Handlers delegate here. This module uses domain types directly and will
//! grow into `application/` with separate files per area when it exceeds ~300 lines.
//! See ADR-002 for the evolution strategy.

use crate::domain::authority::Authority;
use crate::domain::proposal::{ActionId, Proposal, ProposalSignature, ProposalStatus, SeqNo};
use crate::domain::session::AuthChallenge;
use crate::error::Result;

/// Input for creating a new session after attestation verification.
pub(crate) struct CreateSessionInput {
    pub(crate) ephemeral_pubkey: String,
    pub(crate) nonce: String,
    pub(crate) attestation_signature: String,
    pub(crate) signer_pubkey: String,
    pub(crate) authority: String,
}

/// Result of submitting a signature to a proposal.
pub(crate) struct SubmitSignatureResult {
    pub(crate) signature_id: String,
    pub(crate) quorum_reached: bool,
}

/// Result of creating a new session.
pub(crate) struct SessionResult {
    pub(crate) session_id: String,
    pub(crate) expires_at: String,
}

// ─── Auth ────────────────────────────────────────────────────────────────────

pub(crate) fn get_challenge(_pubkey: &str, _authority: &str) -> Result<AuthChallenge> {
    todo!("issue nonce challenge for signer")
}

pub(crate) fn create_session(_input: CreateSessionInput) -> Result<SessionResult> {
    todo!("verify attestation against ASM signer set, create session")
}

pub(crate) fn delete_session(_session_id: &str) -> Result<()> {
    todo!("invalidate session")
}

// ─── Proposals ───────────────────────────────────────────────────────────────

pub(crate) fn list_proposals(
    _authority: Authority,
    _status: Option<ProposalStatus>,
) -> Result<Vec<Proposal>> {
    todo!("list proposals scoped to authority")
}

pub(crate) fn create_proposal(
    _authority: Authority,
    _seq_no: SeqNo,
    _action_payload: serde_json::Value,
) -> Result<(ActionId, Proposal)> {
    todo!("derive ActionId, prevent duplicates, store proposal")
}

pub(crate) fn get_proposal(_action_id: &str) -> Result<Proposal> {
    todo!("fetch proposal by ActionId, scoped to session authority")
}

// ─── Signatures ──────────────────────────────────────────────────────────────

pub(crate) fn submit_signature(
    _action_id: &str,
    _signer_pubkey: &str,
    _signature: &str,
) -> Result<SubmitSignatureResult> {
    todo!("validate signature, prevent duplicates, check quorum")
}

pub(crate) fn list_signatures(_action_id: &str) -> Result<Vec<ProposalSignature>> {
    todo!("list signatures for proposal, scoped to session authority")
}
