//! Client-side proposal domain types — pure data, no framework dependencies.

use serde::{Deserialize, Serialize};

/// A proposal as seen by the desktop client.
#[derive(Debug, Clone, Deserialize)]
pub struct Proposal {
    pub action_id: String,
    pub seq_no: u64,
    pub authority: String,
    pub status: String,
    pub action_hex: String,
    pub signatures: Vec<ProposalSignature>,
}

/// A signature attached to a proposal.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProposalSignature {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// A cryptographic signature produced by a signer, awaiting submission.
#[derive(Debug, Clone)]
pub struct Signature {
    pub signer_pubkey: String,
    pub signature_hex: String,
}
