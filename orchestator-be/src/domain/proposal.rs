use crate::domain::authority::Authority;
use serde::{Deserialize, Serialize};

/// Sequence number for a multisig action. Protocol allows gaps (skipped values).
pub type SeqNo = u64;

/// Deterministic proposal identity: sha256(seq_no_be_bytes || action_hex_bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

/// Lifecycle state of a proposal, aligned with ASM state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Offchain, collecting signatures. Expires after 7 days.
    Pending,
    /// Threshold reached, broadcast to Bitcoin. Onchain, awaiting enactment (~2016 blocks).
    Approved,
    /// Enacted onchain.
    Enacted,
    /// Canceled during the approved window.
    Canceled,
    /// Expired before reaching threshold.
    Expired,
}

/// A multisig proposal stored by the coordination backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub action_id: ActionId,
    pub seq_no: SeqNo,
    pub authority: Authority,
    pub status: ProposalStatus,
    /// Hex-encoded MultisigAction payload (opaque to backend).
    pub action_hex: String,
    pub signatures: Vec<ProposalSignature>,
}

/// A signature submitted for a proposal by a signer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalSignature {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// Quorum progress for a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumStatus {
    pub collected: u32,
    pub required: u32,
    pub is_reached: bool,
}
