//! Proposal business logic — CRUD operations for the orchestrator.
//!
//! Functions receive a `ProposalRepository` as a parameter (dependency injection).
//! No authentication or quorum detection — those are added in future slices.

use crate::application::repository::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{ActionId, Proposal, ProposalSignature, ProposalStatus, SeqNo};
use crate::error::AppError;
use sha2::{Digest, Sha256};

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

/// Create a new proposal with first signature. Rejects duplicate ActionId.
pub(crate) fn create_update_action(
    repo: &mut dyn ProposalRepository,
    authority: Authority,
    seq_no: SeqNo,
    action_hex: &str,
    signer_pubkey: &str,
    signature_hex: &str,
) -> Result<Proposal, AppError> {
    let action_id = compute_action_id(seq_no, action_hex)?;

    let proposal = Proposal {
        action_id,
        seq_no,
        authority,
        status: ProposalStatus::Pending,
        action_hex: action_hex.to_string(),
        signatures: vec![ProposalSignature {
            signer_pubkey: signer_pubkey.to_string(),
            signature_hex: signature_hex.to_string(),
        }],
    };

    repo.save_proposal(proposal.clone())?;

    Ok(proposal)
}

/// Add a signature to an existing proposal. Rejects duplicate signer.
pub(crate) fn approve_action(
    repo: &mut dyn ProposalRepository,
    action_id: &ActionId,
    signer_pubkey: &str,
    signature_hex: &str,
) -> Result<Proposal, AppError> {
    let proposal = repo
        .find_by_action_id_mut(action_id)
        .ok_or(AppError::NotFound)?;

    let already_signed = proposal
        .signatures
        .iter()
        .any(|s| s.signer_pubkey == signer_pubkey);

    if already_signed {
        return Err(AppError::Conflict("signer already signed".to_string()));
    }

    proposal.signatures.push(ProposalSignature {
        signer_pubkey: signer_pubkey.to_string(),
        signature_hex: signature_hex.to_string(),
    });

    Ok(proposal.clone())
}

/// Get proposal by ActionId.
pub(crate) fn get_update_action(
    repo: &dyn ProposalRepository,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    repo.find_by_action_id(action_id)
        .cloned()
        .ok_or(AppError::NotFound)
}

/// List proposals, optionally filtered by status.
pub(crate) fn list_proposals(
    repo: &dyn ProposalRepository,
    status: Option<ProposalStatus>,
) -> Vec<Proposal> {
    repo.list_by_status(status).into_iter().cloned().collect()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::InMemoryProposalRepository;

    fn new_repo() -> InMemoryProposalRepository {
        InMemoryProposalRepository::new()
    }

    const ACTION_HEX: &str = "deadbeef";
    const SIGNER_A: &str = "pubkey_a";
    const SIGNER_B: &str = "pubkey_b";
    const SIG_A: &str = "sig_a";
    const SIG_B: &str = "sig_b";

    // ─── ActionId ───────────────────────────────────────────────────────────

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

    // ─── create_update_action ───────────────────────────────────────────────

    #[test]
    fn test_create_update_action() {
        let mut repo = new_repo();

        let proposal = create_update_action(
            &mut repo,
            Authority::StrataAdmin,
            1,
            ACTION_HEX,
            SIGNER_A,
            SIG_A,
        )
        .unwrap();

        assert_eq!(proposal.seq_no, 1);
        assert_eq!(proposal.authority, Authority::StrataAdmin);
        assert_eq!(proposal.action_hex, ACTION_HEX);
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.signatures.len(), 1);
        assert_eq!(proposal.signatures[0].signer_pubkey, SIGNER_A);
        assert_eq!(proposal.signatures[0].signature_hex, SIG_A);

        // ActionId is deterministic
        let expected_id = compute_action_id(1, ACTION_HEX).unwrap();
        assert_eq!(proposal.action_id, expected_id);
    }

    #[test]
    fn test_create_duplicate_action_rejected() {
        let mut repo = new_repo();

        create_update_action(
            &mut repo,
            Authority::StrataAdmin,
            1,
            ACTION_HEX,
            SIGNER_A,
            SIG_A,
        )
        .unwrap();

        let result = create_update_action(
            &mut repo,
            Authority::StrataAdmin,
            1,
            ACTION_HEX,
            SIGNER_B,
            SIG_B,
        );

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    // ─── approve_action ─────────────────────────────────────────────────────

    #[test]
    fn test_approve_action() {
        let mut repo = new_repo();

        let created = create_update_action(
            &mut repo,
            Authority::StrataAdmin,
            1,
            ACTION_HEX,
            SIGNER_A,
            SIG_A,
        )
        .unwrap();

        let updated = approve_action(&mut repo, &created.action_id, SIGNER_B, SIG_B).unwrap();

        assert_eq!(updated.signatures.len(), 2);
        assert_eq!(updated.signatures[1].signer_pubkey, SIGNER_B);
    }

    #[test]
    fn test_approve_duplicate_signer_rejected() {
        let mut repo = new_repo();

        let created = create_update_action(
            &mut repo,
            Authority::StrataAdmin,
            1,
            ACTION_HEX,
            SIGNER_A,
            SIG_A,
        )
        .unwrap();

        let result = approve_action(&mut repo, &created.action_id, SIGNER_A, "another_sig");

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[test]
    fn test_approve_nonexistent_proposal() {
        let mut repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let result = approve_action(&mut repo, &fake_id, SIGNER_A, SIG_A);

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    // ─── get_update_action ──────────────────────────────────────────────────

    #[test]
    fn test_get_update_action() {
        let mut repo = new_repo();

        let created = create_update_action(
            &mut repo,
            Authority::StrataAdmin,
            1,
            ACTION_HEX,
            SIGNER_A,
            SIG_A,
        )
        .unwrap();

        let fetched = get_update_action(&repo, &created.action_id).unwrap();

        assert_eq!(fetched.action_id, created.action_id);
        assert_eq!(fetched.action_hex, created.action_hex);
        assert_eq!(fetched.seq_no, created.seq_no);
    }

    #[test]
    fn test_get_nonexistent_proposal() {
        let repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let result = get_update_action(&repo, &fake_id);

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    // ─── list_proposals ─────────────────────────────────────────────────────

    #[test]
    fn test_list_proposals() {
        let mut repo = new_repo();

        create_update_action(&mut repo, Authority::StrataAdmin, 1, "aa", SIGNER_A, SIG_A).unwrap();
        create_update_action(&mut repo, Authority::StrataAdmin, 2, "bb", SIGNER_A, SIG_A).unwrap();

        let all = list_proposals(&repo, None);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_proposals_with_status_filter() {
        let mut repo = new_repo();

        create_update_action(&mut repo, Authority::StrataAdmin, 1, "aa", SIGNER_A, SIG_A).unwrap();
        create_update_action(&mut repo, Authority::StrataAdmin, 2, "bb", SIGNER_A, SIG_A).unwrap();

        let pending = list_proposals(&repo, Some(ProposalStatus::Pending));
        assert_eq!(pending.len(), 2);

        let approved = list_proposals(&repo, Some(ProposalStatus::Approved));
        assert_eq!(approved.len(), 0);
    }
}
