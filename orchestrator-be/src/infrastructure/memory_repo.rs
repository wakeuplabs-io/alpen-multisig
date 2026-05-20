//! In-memory proposal repository for POC and testing.

use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    ActionId, BroadcastStatus, Proposal, ProposalSignature, ProposalStatus,
};
use crate::error::AppError;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::authority::Authority;
    use crate::domain::proposal::{
        ActionId, BroadcastStatus, Proposal, ProposalSignature, ProposalStatus,
    };

    fn make_proposal(action_id: &str, signer_pubkey: &str) -> Proposal {
        Proposal {
            action_id: ActionId(action_id.to_string()),
            seq_no: 1,
            authority: Authority::StrataAdmin,
            status: ProposalStatus::Pending,
            required_signatures: 2,
            action_hex: "deadbeef".to_string(),
            signatures: vec![ProposalSignature {
                signer_pubkey: signer_pubkey.to_string(),
                signature_hex: "sig0".to_string(),
            }],
            broadcast_status: BroadcastStatus::Idle,
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
        }
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
