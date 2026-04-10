//! Transport DTOs for the orchestrator API contract.
//!
//! These types define the JSON shapes exchanged between the desktop app
//! and the orchestrator backend. The orchestrator (Step 2) must produce
//! compatible responses.

use serde::{Deserialize, Serialize};

// ─── Requests ───────────────────────────────────────────────────────────────

/// Request to create a proposal with initial signature.
#[derive(Debug, Serialize)]
pub(crate) struct CreateProposalRequest {
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Request to submit a signature for an existing proposal.
#[derive(Debug, Serialize)]
pub(crate) struct SubmitSignatureRequest {
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

// ─── Responses ──────────────────────────────────────────────────────────────

/// Response from creating a proposal.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProposalResponse {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<SignatureInfo>,
}

/// Summary of a proposal for list views.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProposalSummary {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) status: String,
    pub(crate) signature_count: u32,
    pub(crate) threshold: u32,
}

/// Full proposal detail including all signatures.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProposalDetail {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<SignatureInfo>,
    pub(crate) threshold: u32,
}

/// A single signature on a proposal.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct SignatureInfo {
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Response from submitting a signature.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SignatureResponse {
    pub(crate) quorum_reached: bool,
    pub(crate) signatures_count: u32,
    pub(crate) threshold: u32,
}
