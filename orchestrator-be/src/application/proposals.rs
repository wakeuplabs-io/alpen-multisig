//! Proposal business logic — CRUD operations over the repository trait.
//!
//! Functions receive a `ProposalRepository` as a parameter (dependency injection).
//! No authentication or quorum detection — those are added in future slices.

use crate::application::traits::{ProposalRepository, SignerSetRepository};
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    compute_action_id, ActionId, Proposal, ProposalSignature, ProposalStatus, SeqNo,
};
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionContext<'a> {
    pub authority: Authority,
    pub signer_pubkey: &'a str,
}

/// Create a new proposal with first signature. Rejects duplicate ActionId.
///
/// Mirrors PRD: `create_update_action(action, seq, sig)`.
pub(crate) fn create_update_action(
    repo: &mut dyn ProposalRepository,
    signer_set_repo: &dyn SignerSetRepository,
    session: SessionContext<'_>,
    seq_no: SeqNo,
    action_hex: &str,
    sig: &ProposalSignature,
) -> Result<Proposal, AppError> {
    if !sig
        .signer_pubkey
        .eq_ignore_ascii_case(session.signer_pubkey)
    {
        return Err(AppError::Unauthorized);
    }

    let is_member =
        signer_set_repo.is_signer_for_authority(session.authority, session.signer_pubkey)?;
    if !is_member {
        return Err(AppError::Unauthorized);
    }

    let action_id = compute_action_id(seq_no, action_hex)?;

    let proposal = Proposal {
        action_id,
        seq_no,
        authority: session.authority,
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
    signer_set_repo: &dyn SignerSetRepository,
    session: SessionContext<'_>,
    action_id: &ActionId,
    sig: &ProposalSignature,
) -> Result<Proposal, AppError> {
    if !sig
        .signer_pubkey
        .eq_ignore_ascii_case(session.signer_pubkey)
    {
        return Err(AppError::Unauthorized);
    }
    let is_member =
        signer_set_repo.is_signer_for_authority(session.authority, session.signer_pubkey)?;
    if !is_member {
        return Err(AppError::Unauthorized);
    }

    let proposal = repo
        .find_by_action_id_mut(action_id)
        .ok_or(AppError::NotFound)?;
    if proposal.authority != session.authority {
        return Err(AppError::Unauthorized);
    }

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
    use crate::infrastructure::signer_set_repo::InMemorySignerSetRepository;

    fn new_repo() -> InMemoryProposalRepository {
        InMemoryProposalRepository::new()
    }

    fn signer_set_repo() -> InMemorySignerSetRepository {
        InMemorySignerSetRepository::new()
    }

    const ACTION_HEX: &str = "deadbeef";

    fn sig_a() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .to_string(),
            signature_hex: "sig_a".to_string(),
        }
    }

    fn sig_b() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5"
                .to_string(),
            signature_hex: "sig_b".to_string(),
        }
    }

    fn sig_non_member() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "03aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            signature_hex: "sig_x".to_string(),
        }
    }

    #[test]
    fn test_create_update_action() {
        let mut repo = new_repo();
        let signer_set_repo = signer_set_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let proposal =
            create_update_action(&mut repo, &signer_set_repo, session, 1, ACTION_HEX, &sig)
                .unwrap();

        assert_eq!(proposal.seq_no, 1);
        assert_eq!(
            proposal.authority,
            crate::domain::authority::Authority::StrataAdmin
        );
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
        let signer_set_repo = signer_set_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(
            &mut repo,
            &signer_set_repo,
            session.clone(),
            1,
            ACTION_HEX,
            &sig,
        )
        .unwrap();

        let result =
            create_update_action(&mut repo, &signer_set_repo, session, 1, ACTION_HEX, &sig);

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[test]
    fn test_approve_action() {
        let mut repo = new_repo();
        let signer_set_repo = signer_set_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(
            &mut repo,
            &signer_set_repo,
            session.clone(),
            1,
            ACTION_HEX,
            &sig,
        )
        .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        let updated = approve_action(
            &mut repo,
            &signer_set_repo,
            session_b,
            &created.action_id,
            &sig_b(),
        )
        .unwrap();

        assert_eq!(updated.signatures.len(), 2);
        assert_eq!(
            updated.signatures[1].signer_pubkey,
            "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5"
        );
    }

    #[test]
    fn test_approve_duplicate_signer_rejected() {
        let mut repo = new_repo();
        let signer_set_repo = signer_set_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(
            &mut repo,
            &signer_set_repo,
            session.clone(),
            1,
            ACTION_HEX,
            &sig,
        )
        .unwrap();

        let dup_sig = ProposalSignature {
            signer_pubkey: "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .to_string(),
            signature_hex: "different_sig".to_string(),
        };
        let result = approve_action(
            &mut repo,
            &signer_set_repo,
            session,
            &created.action_id,
            &dup_sig,
        );

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[test]
    fn test_approve_nonexistent_proposal() {
        let mut repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let signer_set_repo = signer_set_repo();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_a().signer_pubkey,
        };
        let result = approve_action(&mut repo, &signer_set_repo, session, &fake_id, &sig_a());

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    #[test]
    fn test_get_update_action() {
        let mut repo = new_repo();
        let signer_set_repo = signer_set_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created =
            create_update_action(&mut repo, &signer_set_repo, session, 1, ACTION_HEX, &sig)
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
        let signer_set_repo = signer_set_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&mut repo, &signer_set_repo, session.clone(), 1, "aa", &sig).unwrap();
        create_update_action(&mut repo, &signer_set_repo, session, 2, "bb", &sig).unwrap();

        let all = list_proposals(&repo, None);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_proposals_with_status_filter() {
        let mut repo = new_repo();
        let signer_set_repo = signer_set_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&mut repo, &signer_set_repo, session.clone(), 1, "aa", &sig).unwrap();
        create_update_action(&mut repo, &signer_set_repo, session, 2, "bb", &sig).unwrap();

        let pending = list_proposals(&repo, Some(ProposalStatus::Pending));
        assert_eq!(pending.len(), 2);

        let approved = list_proposals(&repo, Some(ProposalStatus::Approved));
        assert_eq!(approved.len(), 0);
    }

    #[test]
    fn test_create_rejects_unauthorized_signer() {
        let mut repo = new_repo();
        let signer_set_repo = signer_set_repo();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_non_member().signer_pubkey,
        };

        let err = create_update_action(
            &mut repo,
            &signer_set_repo,
            session,
            1,
            ACTION_HEX,
            &sig_non_member(),
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[test]
    fn test_create_rejects_signer_mismatch_against_session() {
        let mut repo = new_repo();
        let signer_set_repo = signer_set_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: "03aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        };

        let err = create_update_action(&mut repo, &signer_set_repo, session, 1, ACTION_HEX, &sig)
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }
}
