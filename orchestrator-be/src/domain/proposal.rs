//! Proposal domain types — pure value objects, no framework dependencies.

use crate::domain::authority::Authority;
use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Sequence number for a multisig action. Protocol allows gaps (skipped values).
pub type SeqNo = u64;

/// Deterministic proposal identity: sha256(seq_no_be_bytes || action_hex_bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

/// Broadcast sub-status tracking the commit/reveal sequence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadcastStatus {
    #[default]
    Idle,
    CommitBroadcasted,
    CommitConfirmed,
    RevealBroadcasted,
    RevealConfirmed,
    Failed,
}

impl std::fmt::Display for BroadcastStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Idle => "idle",
            Self::CommitBroadcasted => "commit_broadcasted",
            Self::CommitConfirmed => "commit_confirmed",
            Self::RevealBroadcasted => "reveal_broadcasted",
            Self::RevealConfirmed => "reveal_confirmed",
            Self::Failed => "failed",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for BroadcastStatus {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(Self::Idle),
            "commit_broadcasted" => Ok(Self::CommitBroadcasted),
            "commit_confirmed" => Ok(Self::CommitConfirmed),
            "reveal_broadcasted" => Ok(Self::RevealBroadcasted),
            "reveal_confirmed" => Ok(Self::RevealConfirmed),
            "failed" => Ok(Self::Failed),
            _ => Err(AppError::Internal(anyhow::anyhow!(
                "unknown broadcast_status: {s}"
            ))),
        }
    }
}

/// Lifecycle state of a proposal, aligned with ASM state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Offchain, collecting signatures. Expires after 7 days.
    Pending,
    /// Threshold reached, broadcast to Bitcoin. Onchain, awaiting enactment (~2016 blocks).
    Approved,
    /// ASM applied the governance change (not merely reveal confirmed on Bitcoin).
    Enacted,
    /// Canceled during the approved window.
    Canceled,
    /// Expired before reaching threshold.
    Expired,
    /// The role's on-chain sequence number passed this proposal's, so the ASM will refuse its
    /// transaction from here on. Terminal, and independent of whether it was ever broadcast: the
    /// seqno is inside the signed message, so the proposal cannot be relabelled and resent.
    /// See docs/specs/proposal-lifecycle-seqno-truth.md.
    Superseded,
}

impl std::fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Enacted => "enacted",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
            Self::Superseded => "superseded",
        };
        write!(f, "{s}")
    }
}

/// A multisig proposal stored by the coordination backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub action_id: ActionId,
    pub seq_no: SeqNo,
    pub authority: Authority,
    pub status: ProposalStatus,
    pub required_signatures: u16,
    /// Hex-encoded MultisigAction payload (opaque to backend).
    pub action_hex: String,
    /// Human-written label for the proposal. Coordination metadata only: it is not part of
    /// `action_hex`, not covered by any signature, and not an input to `compute_action_id`.
    /// Optional, because proposals created before this field existed have none.
    pub title: Option<String>,
    pub signatures: Vec<ProposalSignature>,
    pub broadcast_status: BroadcastStatus,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
    /// Set when this row is a cancel proposal; points to the target proposal's action_id.
    pub target_action_id: Option<ActionId>,
    /// Bitcoin block height at which the target update activates. Set after RevealConfirmed.
    pub activation_height: Option<u64>,
    /// ASM queue UpdateId assigned to this update when its reveal tx confirmed. Set after RevealConfirmed.
    /// Required to build a valid CancelAction (target_id field). Distinct from seq_no.
    pub update_id_in_queue: Option<u32>,
    /// When the proposal was created. Used to enforce the 7-day expiry TTL.
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

impl Proposal {
    pub fn is_cancel(&self) -> bool {
        self.target_action_id.is_some()
    }
}

/// A signature submitted for a proposal by a signer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalSignature {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// Compute ActionId = hex(sha256(seq_no_be_bytes || action_hex_bytes)).
///
/// Deterministic: same (seq_no, action_hex) always produces the same ActionId.
pub(crate) fn compute_action_id(seq_no: SeqNo, action_hex: &str) -> Result<ActionId, AppError> {
    let action_bytes = hex::decode(action_hex)
        .map_err(|e| AppError::BadRequest(format!("invalid action hex: {e}")))?;

    let mut hasher = Sha256::new();
    hasher.update(seq_no.to_be_bytes());
    hasher.update(&action_bytes);
    let hash = hasher.finalize();

    Ok(ActionId(hex::encode(hash)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTION_HEX: &str = "deadbeef";

    #[test]
    fn test_action_id_is_deterministic() {
        let id1 = compute_action_id(1, ACTION_HEX).unwrap();
        let id2 = compute_action_id(1, ACTION_HEX).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_action_id_differs_by_seq_no() {
        let id1 = compute_action_id(1, ACTION_HEX).unwrap();
        let id2 = compute_action_id(2, ACTION_HEX).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_action_id_differs_by_action() {
        let id1 = compute_action_id(1, "deadbeef").unwrap();
        let id2 = compute_action_id(1, "cafebabe").unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_create_invalid_action_hex() {
        let result = compute_action_id(1, "not_valid_hex");
        assert!(result.is_err());
    }
}
