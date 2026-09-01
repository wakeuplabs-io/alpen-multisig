//! In-memory proposal repository for POC and testing.

use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    ActionId, BroadcastStatus, Proposal, ProposalSignature, ProposalStatus,
};
use crate::error::AppError;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct InMemoryProposalRepository {
    proposals: RwLock<HashMap<ActionId, Proposal>>,
}

impl InMemoryProposalRepository {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn new() -> Self {
        Self {
            proposals: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl ProposalRepository for InMemoryProposalRepository {
    async fn save_proposal(&self, proposal: Proposal) -> Result<(), AppError> {
        let mut proposals = self
            .proposals
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        if proposals.contains_key(&proposal.action_id) {
            return Err(AppError::Conflict("proposal already exists".to_string()));
        }
        proposals.insert(proposal.action_id.clone(), proposal);
        Ok(())
    }

    async fn find_by_action_id(&self, action_id: &ActionId) -> Result<Option<Proposal>, AppError> {
        let proposals = self
            .proposals
            .read()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        Ok(proposals.get(action_id).cloned())
    }

    async fn add_signature(
        &self,
        action_id: &ActionId,
        signer_pubkey: &str,
        signature_hex: &str,
    ) -> Result<Option<Proposal>, AppError> {
        let mut proposals = self
            .proposals
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        let Some(proposal) = proposals.get_mut(action_id) else {
            return Ok(None);
        };
        let already_signed = proposal
            .signatures
            .iter()
            .any(|s| s.signer_pubkey.eq_ignore_ascii_case(signer_pubkey));
        if already_signed {
            return Err(AppError::Conflict("signer already signed".to_string()));
        }
        proposal.signatures.push(ProposalSignature {
            signer_pubkey: signer_pubkey.to_string(),
            signature_hex: signature_hex.to_string(),
        });
        Ok(Some(proposal.clone()))
    }

    async fn list_by_status(
        &self,
        authority: Authority,
        status: Option<ProposalStatus>,
    ) -> Result<Vec<Proposal>, AppError> {
        let proposals = self
            .proposals
            .read()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        Ok(proposals
            .values()
            .filter(|p| p.authority == authority && status.is_none_or(|s| p.status == s))
            .cloned()
            .collect())
    }

    async fn claim_broadcast(&self, action_id: &ActionId) -> Result<Proposal, AppError> {
        let mut proposals = self
            .proposals
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        let Some(proposal) = proposals.get_mut(action_id) else {
            return Err(AppError::NotFound);
        };
        match proposal.broadcast_status {
            BroadcastStatus::Idle | BroadcastStatus::Failed => {}
            _ => {
                return Err(AppError::Conflict(
                    "broadcast already in progress or completed".to_string(),
                ))
            }
        }
        proposal.broadcast_status = BroadcastStatus::CommitBroadcasted;
        proposal.updated_at = Utc::now();
        Ok(proposal.clone())
    }

    async fn update_broadcast_status(
        &self,
        action_id: &ActionId,
        status: BroadcastStatus,
        proposal_status: Option<ProposalStatus>,
        commit_txid: Option<&str>,
        reveal_txid: Option<&str>,
        error: Option<&str>,
    ) -> Result<Option<Proposal>, AppError> {
        let mut proposals = self
            .proposals
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        let Some(proposal) = proposals.get_mut(action_id) else {
            return Ok(None);
        };
        proposal.broadcast_status = status;
        proposal.updated_at = Utc::now();
        if let Some(s) = proposal_status {
            proposal.status = s;
        }
        if let Some(txid) = commit_txid {
            proposal.commit_txid = Some(txid.to_string());
        }
        if let Some(txid) = reveal_txid {
            proposal.reveal_txid = Some(txid.to_string());
        }
        proposal.broadcast_error = error.map(|s| s.to_string());
        Ok(Some(proposal.clone()))
    }

    async fn find_cancel_for_target(
        &self,
        target: &ActionId,
    ) -> Result<Option<Proposal>, AppError> {
        let proposals = self
            .proposals
            .read()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        Ok(proposals
            .values()
            .find(|p| p.target_action_id.as_ref() == Some(target))
            .cloned())
    }

    async fn update_activation_height(
        &self,
        action_id: &ActionId,
        height: u64,
    ) -> Result<(), AppError> {
        let mut proposals = self
            .proposals
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        if let Some(proposal) = proposals.get_mut(action_id) {
            proposal.activation_height = Some(height);
            proposal.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_update_id_in_queue(
        &self,
        action_id: &ActionId,
        update_id: u32,
    ) -> Result<(), AppError> {
        let mut proposals = self
            .proposals
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;
        if let Some(proposal) = proposals.get_mut(action_id) {
            proposal.update_id_in_queue = Some(update_id);
            proposal.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn enact_cancel(
        &self,
        cancel_action_id: &ActionId,
        target_action_id: &ActionId,
    ) -> Result<bool, AppError> {
        let mut proposals = self
            .proposals
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;

        let target_is_approved = proposals
            .get(target_action_id)
            .map(|p| p.status == ProposalStatus::Approved)
            .unwrap_or(false);

        if !target_is_approved {
            return Ok(false);
        }

        if let Some(target) = proposals.get_mut(target_action_id) {
            target.status = ProposalStatus::Canceled;
            target.updated_at = Utc::now();
        }
        if let Some(proposal) = proposals.get_mut(cancel_action_id) {
            proposal.status = ProposalStatus::Enacted;
            proposal.updated_at = Utc::now();
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::authority::Authority;
    use crate::domain::proposal::{
        ActionId, BroadcastStatus, Proposal, ProposalSignature, ProposalStatus,
    };
    use chrono::Utc;

    fn make_proposal(action_id: &str, signer_pubkey: &str) -> Proposal {
        Proposal {
            action_id: ActionId(action_id.to_string()),
            seq_no: 1,
            authority: Authority::StrataAdmin,
            status: ProposalStatus::Pending,
            required_signatures: 2,
            action_hex: "deadbeef".to_string(),
            title: None,
            signatures: vec![ProposalSignature {
                signer_pubkey: signer_pubkey.to_string(),
                signature_hex: "sig0".to_string(),
            }],
            broadcast_status: BroadcastStatus::Idle,
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// `updated_at` is what the screens read to say how long a bundle has been sitting where it
    /// is, and the Postgres repository bumps it on every write that changes a proposal. This one
    /// used to bump on the broadcast writes only, so the two disagreed and nothing noticed:
    /// the suite runs against this repository.
    #[tokio::test]
    async fn every_write_moves_updated_at() {
        let repo = InMemoryProposalRepository::new();
        let action_id = ActionId("action1".to_string());
        let mut proposal = make_proposal("action1", "02aa");
        proposal.updated_at = Utc::now() - chrono::Duration::hours(1);
        let before = proposal.updated_at;
        repo.save_proposal(proposal).await.unwrap();

        repo.update_activation_height(&action_id, 101)
            .await
            .unwrap();
        let after_height = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert!(
            after_height.updated_at > before,
            "activation height is a change"
        );

        repo.update_update_id_in_queue(&action_id, 7).await.unwrap();
        let after_queue = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert!(
            after_queue.updated_at >= after_height.updated_at,
            "the queue id is a change too"
        );
    }

    #[tokio::test]
    async fn test_add_signature_rejects_duplicate_signer_under_write_lock() {
        let repo = InMemoryProposalRepository::new();
        let pubkey = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let proposal = make_proposal("action1", pubkey);
        repo.save_proposal(proposal).await.unwrap();

        // first add should fail — signer already in initial signatures
        let result = repo
            .add_signature(&ActionId("action1".to_string()), pubkey, "sig_dup")
            .await;
        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }
}
