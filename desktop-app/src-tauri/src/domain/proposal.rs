//! Client-side proposal domain types — pure data, no framework dependencies.

use serde::{Deserialize, Serialize};

use crate::domain::authority::Authority;

/// A proposal as seen by the desktop client.
#[derive(Debug, Clone, Deserialize)]
pub struct Proposal {
    pub action_id: String,
    pub seq_no: u64,
    pub authority: Authority,
    pub status: String,
    pub required_signatures: u16,
    pub action_hex: String,
    pub signatures: Vec<ProposalSignature>,
    pub broadcast_status: String,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
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
