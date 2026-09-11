//! Client-side proposal domain types — pure data, no framework dependencies.

use serde::{Deserialize, Serialize};

use crate::domain::authority::Authority;

/// Summary of a cancel proposal attached to a target proposal's detail response.
#[derive(Debug, Clone, Deserialize)]
pub struct CancelProposalSummary {
    pub action_id: String,
    pub status: String,
    pub signatures: Vec<ProposalSignature>,
    pub required_signatures: u16,
}

/// A proposal as seen by the desktop client.
#[derive(Debug, Clone, Deserialize)]
pub struct Proposal {
    pub action_id: String,
    pub seq_no: u64,
    pub authority: Authority,
    pub status: String,
    pub required_signatures: u16,
    pub action_hex: String,
    /// Human-written label from the proposal author. Coordination metadata: not signed, not part
    /// of `action_hex`. `None` for proposals created before the field existed.
    #[serde(default)]
    pub title: Option<String>,
    pub signatures: Vec<ProposalSignature>,
    pub broadcast_status: String,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
    pub target_action_id: Option<String>,
    pub activation_height: Option<u64>,
    /// ASM queue UpdateId assigned when the reveal tx confirmed. Used to build CancelAction.
    pub update_id_in_queue: Option<u32>,
    pub cancel_proposal: Option<CancelProposalSummary>,
    /// Unix epoch milliseconds when the proposal was created. Used to derive expiry.
    pub created_at: i64,
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
