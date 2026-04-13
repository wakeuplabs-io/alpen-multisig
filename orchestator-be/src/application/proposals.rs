//! Proposal business logic — CRUD operations over the repository trait.
//!
//! Functions receive a `ProposalRepository` as a parameter (dependency injection).
//! No authentication or quorum detection — those are added in future slices.

use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    compute_action_id, ActionId, Proposal, ProposalSignature, ProposalStatus, SeqNo,
};
use crate::error::AppError;

/// Create a new proposal with first signature. Rejects duplicate ActionId.
///
/// Mirrors PRD: `create_update_action(action, seq, sig)`.
pub(crate) fn create_update_action(
    repo: &mut dyn ProposalRepository,
    authority: Authority,
    seq_no: SeqNo,
    action_hex: &str,
    sig: &ProposalSignature,
) -> Result<Proposal, AppError> {
    let action_id = compute_action_id(seq_no, action_hex)?;

    let proposal = Proposal {
        action_id,
        seq_no,
        authority,
        status: ProposalStatus::Pending,
        action_hex: action_hex.to_string(),
        signatures: vec![sig.clone()],
    };

    repo.save_proposal(proposal.clone())?;

    Ok(proposal)
}

/// Add a signature to an existing proposal. Rejects duplicate signer.
///
/// Mirrors PRD: `approve_action(id, sig)`.
pub(crate) fn approve_action(
    repo: &mut dyn ProposalRepository,
    action_id: &ActionId,
    sig: &ProposalSignature,
) -> Result<Proposal, AppError> {
    let proposal = repo
        .find_by_action_id_mut(action_id)
        .ok_or(AppError::NotFound)?;

    let already_signed = proposal
        .signatures
        .iter()
        .any(|s| s.signer_pubkey == sig.signer_pubkey);

    if already_signed {
        return Err(AppError::Conflict("signer already signed".to_string()));
    }

    proposal.signatures.push(sig.clone());

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory_repo::InMemoryProposalRepository;

    fn new_repo() -> InMemoryProposalRepository {
        InMemoryProposalRepository::new()
    }

    const ACTION_HEX: &str = "deadbeef";

    fn sig_a() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "pubkey_a".to_string(),
            signature_hex: "sig_a".to_string(),
        }
    }

    fn sig_b() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "pubkey_b".to_string(),
            signature_hex: "sig_b".to_string(),
        }
    }

    #[test]
    fn test_create_update_action() {
        let mut repo = new_repo();
        let sig = sig_a();

        let proposal =
            create_update_action(&mut repo, Authority::StrataAdmin, 1, ACTION_HEX, &sig).unwrap();

        assert_eq!(proposal.seq_no, 1);
        assert_eq!(proposal.authority, Authority::StrataAdmin);
        assert_eq!(proposal.action_hex, ACTION_HEX);
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.signatures.len(), 1);
        assert_eq!(proposal.signatures[0].signer_pubkey, sig.signer_pubkey);

        let expected_id = compute_action_id(1, ACTION_HEX).unwrap();
        assert_eq!(proposal.action_id, expected_id);
    }

    #[test]
    fn test_create_duplicate_action_rejected() {
        let mut repo = new_repo();

        create_update_action(&mut repo, Authority::StrataAdmin, 1, ACTION_HEX, &sig_a()).unwrap();

        let result =
            create_update_action(&mut repo, Authority::StrataAdmin, 1, ACTION_HEX, &sig_b());

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[test]
    fn test_approve_action() {
        let mut repo = new_repo();

        let created =
            create_update_action(&mut repo, Authority::StrataAdmin, 1, ACTION_HEX, &sig_a())
                .unwrap();

        let updated = approve_action(&mut repo, &created.action_id, &sig_b()).unwrap();

        assert_eq!(updated.signatures.len(), 2);
        assert_eq!(updated.signatures[1].signer_pubkey, "pubkey_b");
    }

    #[test]
    fn test_approve_duplicate_signer_rejected() {
        let mut repo = new_repo();

        let created =
            create_update_action(&mut repo, Authority::StrataAdmin, 1, ACTION_HEX, &sig_a())
                .unwrap();

        let dup_sig = ProposalSignature {
            signer_pubkey: "pubkey_a".to_string(),
            signature_hex: "different_sig".to_string(),
        };
        let result = approve_action(&mut repo, &created.action_id, &dup_sig);

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[test]
    fn test_approve_nonexistent_proposal() {
        let mut repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let result = approve_action(&mut repo, &fake_id, &sig_a());

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    #[test]
    fn test_get_update_action() {
        let mut repo = new_repo();

        let created =
            create_update_action(&mut repo, Authority::StrataAdmin, 1, ACTION_HEX, &sig_a())
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

    #[test]
    fn test_list_proposals() {
        let mut repo = new_repo();

        create_update_action(&mut repo, Authority::StrataAdmin, 1, "aa", &sig_a()).unwrap();
        create_update_action(&mut repo, Authority::StrataAdmin, 2, "bb", &sig_a()).unwrap();

        let all = list_proposals(&repo, None);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_proposals_with_status_filter() {
        let mut repo = new_repo();

        create_update_action(&mut repo, Authority::StrataAdmin, 1, "aa", &sig_a()).unwrap();
        create_update_action(&mut repo, Authority::StrataAdmin, 2, "bb", &sig_a()).unwrap();

        let pending = list_proposals(&repo, Some(ProposalStatus::Pending));
        assert_eq!(pending.len(), 2);

        let approved = list_proposals(&repo, Some(ProposalStatus::Approved));
        assert_eq!(approved.len(), 0);
    }
}
