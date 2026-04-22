//! Proposal domain types — pure value objects, no framework dependencies.

use crate::domain::authority::Authority;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
