use serde::{Deserialize, Serialize};

// ─── Wire types (serialized to/from orchestrator) ────────────────────────────

/// Request to create a proposal with initial signature.
#[derive(Debug, Serialize)]
pub struct CreateProposalRequest {
    pub authority: String,
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// Request to approve (add signature to) an existing proposal.
#[derive(Debug, Serialize)]
pub struct ApproveActionRequest {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// A proposal as returned by the orchestrator.
#[derive(Debug, Clone, Deserialize)]
pub struct Proposal {
    pub action_id: String,
    pub seq_no: u64,
    pub authority: String,
    pub status: String,
    pub action_hex: String,
    pub signatures: Vec<ProposalSignature>,
}

/// A signature on a proposal.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProposalSignature {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

// ─── Domain types ─────────────────────────────────────────────────────────────

/// A cryptographic signature from a signer.
#[derive(Debug, Clone)]
pub struct Signature {
    pub signer_pubkey: String,
    pub signature_hex: String,
}
