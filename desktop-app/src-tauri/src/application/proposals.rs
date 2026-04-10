//! Proposal management — application layer entry point for the desktop app.
//!
//! Public API mirrors the PRD's `MultisigBackend` trait semantics:
//! - `create_update_action(action, seq_no, signature)` — propose + first signature
//! - `approve_action(action_id, signature)` — add approval signature
//! - `get_update_action(action_id)` — fetch proposal detail
//! - `get_signatures(action_id)` — fetch collected signatures
//! - `get_actions_by_seqno(seq_no)` — list actions for a sequence number
//! - `list_proposals(status?)` — list proposals (orchestrator convenience)
//!
//! Authority is implicit — bound to the authenticated session, not passed per call.
//! Signing happens externally (HW wallet, software signer) before reaching this layer.

use crate::application::orchestrator_client::{
    CreateProposalRequest, OrchestratorClient, OrchestratorError, SubmitSignatureRequest,
};

// ─── Errors ─────────────────────────────────────────────────────────────────

/// Errors that can occur during proposal operations.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProposalError {
    #[error("Orchestrator error: {0}")]
    Orchestrator(#[from] OrchestratorError),
}

// ─── Domain types ──────────────────────────────────────────────────────────

/// A cryptographic signature from a signer.
#[derive(Debug, Clone)]
pub(crate) struct Signature {
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// A proposal (action + signatures).
#[derive(Debug, Clone)]
pub(crate) struct Proposal {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<Signature>,
    pub(crate) threshold: u32,
}

/// Summary for list views.
#[derive(Debug, Clone)]
pub(crate) struct ProposalSummary {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) status: String,
    pub(crate) signature_count: u32,
    pub(crate) threshold: u32,
}

/// Result of submitting a signature.
#[derive(Debug, Clone)]
pub(crate) struct ApprovalResult {
    pub(crate) quorum_reached: bool,
    pub(crate) signatures_count: u32,
    pub(crate) threshold: u32,
}

// ─── Production functions ───────────────────────────────────────────────────

/// Create a new action and store the creator's signature.
///
/// Mirrors PRD: `create_update_action(action, seq, sig)`.
/// Authority is bound to the session — the orchestrator resolves it.
pub(crate) async fn create_update_action(
    client: &dyn OrchestratorClient,
    action_hex: &str,
    seq_no: u64,
    signature: &Signature,
) -> Result<Proposal, ProposalError> {
    let request = CreateProposalRequest {
        seq_no,
        action_hex: action_hex.to_string(),
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
    };

    let res = client.create_proposal(request).await?;

    Ok(to_proposal(res))
}

/// Append an approval signature for an existing action.
///
/// Mirrors PRD: `approve_action(id, sig)`.
pub(crate) async fn approve_action(
    client: &dyn OrchestratorClient,
    action_id: &str,
    signature: &Signature,
) -> Result<ApprovalResult, ProposalError> {
    let request = SubmitSignatureRequest {
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
    };

    let res = client.submit_signature(action_id, request).await?;

    Ok(ApprovalResult {
        quorum_reached: res.quorum_reached,
        signatures_count: res.signatures_count,
        threshold: res.threshold,
    })
}

/// Fetch the action payload and details.
///
/// Mirrors PRD: `get_update_action(id)`.
pub(crate) async fn get_update_action(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<Proposal, ProposalError> {
    let res = client.get_proposal(action_id).await?;

    Ok(Proposal {
        action_id: res.action_id,
        authority: res.authority,
        seq_no: res.seq_no,
        action_hex: res.action_hex,
        status: res.status,
        signatures: res.signatures.into_iter().map(to_signature).collect(),
        threshold: res.threshold,
    })
}

/// List proposals, optionally filtered by status.
pub(crate) async fn list_proposals(
    client: &dyn OrchestratorClient,
    status: Option<&str>,
) -> Result<Vec<ProposalSummary>, ProposalError> {
    let items = client.list_proposals(status).await?;

    Ok(items
        .into_iter()
        .map(|s| ProposalSummary {
            action_id: s.action_id,
            authority: s.authority,
            seq_no: s.seq_no,
            status: s.status,
            signature_count: s.signature_count,
            threshold: s.threshold,
        })
        .collect())
}

// ─── Mapping helpers ────────────────────────────────────────────────────────

fn to_signature(s: crate::application::orchestrator_client::SignatureInfo) -> Signature {
    Signature {
        signer_pubkey: s.signer_pubkey,
        signature_hex: s.signature_hex,
    }
}

fn to_proposal(res: crate::application::orchestrator_client::ProposalResponse) -> Proposal {
    Proposal {
        action_id: res.action_id,
        authority: res.authority,
        seq_no: res.seq_no,
        action_hex: res.action_hex,
        status: res.status,
        signatures: res.signatures.into_iter().map(to_signature).collect(),
        threshold: res.threshold,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::orchestrator_client::{
        OrchestratorError, ProposalDetail as OrcProposalDetail,
        ProposalResponse as OrcProposalResponse, ProposalSummary as OrcProposalSummary,
        SignatureInfo, SignatureResponse as OrcSignatureResponse,
    };
    use crate::signing;
    use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
    use rand::rngs::OsRng;
    use std::num::NonZero;
    use std::sync::Mutex;
    use strata_asm_params::Role;
    use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate;
    use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
    use strata_crypto::keys::compressed::CompressedPublicKey;
    use strata_crypto::threshold_signature::ThresholdConfigUpdate;

    // ─── Test helpers ───────────────────────────────────────────────────────

    fn generate_test_keypair() -> (String, String) {
        let sk = SecretKey::new(&mut OsRng);
        let pk = PublicKey::from_secret_key(SECP256K1, &sk);
        (hex::encode(sk.secret_bytes()), hex::encode(pk.serialize()))
    }

    fn demo_action_hex() -> String {
        let demo_bytes = [0x42u8; 32];
        let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
        let new_signer = CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, &demo_sk));
        let config_update = ThresholdConfigUpdate::new(
            vec![new_signer],
            vec![],
            NonZero::new(2).expect("non-zero"),
        );
        let multisig_update = MultisigUpdate::new(config_update, Role::StrataAdministrator);
        let action = MultisigAction::Update(UpdateAction::Multisig(multisig_update));
        hex::encode(borsh::to_vec(&action).expect("action borsh-serializes"))
    }

    /// Sign externally (simulates what HW wallet or software signer would do).
    fn sign_action(secret_key_hex: &str, seq_no: u64, action_hex: &str) -> Signature {
        let sighash = signing::compute_sighash(seq_no, action_hex).expect("sighash ok");
        let sig = signing::sign_sighash(secret_key_hex, &sighash.sighash_hex).expect("sign ok");
        Signature {
            signer_pubkey: sig.public_key_hex,
            signature_hex: sig.signature_hex,
        }
    }

    /// Mock orchestrator client.
    struct MockOrchestratorClient {
        last_create_request: Mutex<Option<CreateProposalRequest>>,
        last_submit_request: Mutex<Option<(String, SubmitSignatureRequest)>>,
        should_fail: bool,
    }

    impl MockOrchestratorClient {
        fn new() -> Self {
            Self {
                last_create_request: Mutex::new(None),
                last_submit_request: Mutex::new(None),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                last_create_request: Mutex::new(None),
                last_submit_request: Mutex::new(None),
                should_fail: true,
            }
        }

        fn last_create_request(&self) -> Option<CreateProposalRequest> {
            self.last_create_request.lock().unwrap().take()
        }

        fn last_submit_request(&self) -> Option<(String, SubmitSignatureRequest)> {
            self.last_submit_request.lock().unwrap().take()
        }
    }

    #[async_trait::async_trait]
    impl OrchestratorClient for MockOrchestratorClient {
        async fn create_proposal(
            &self,
            request: CreateProposalRequest,
        ) -> Result<OrcProposalResponse, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            let response = OrcProposalResponse {
                action_id: format!("action_{}", request.seq_no),
                authority: "strata_admin".to_string(),
                seq_no: request.seq_no,
                action_hex: request.action_hex.clone(),
                status: "pending".to_string(),
                signatures: vec![SignatureInfo {
                    signer_pubkey: request.signer_pubkey.clone(),
                    signature_hex: request.signature_hex.clone(),
                }],
                threshold: 2,
            };
            *self.last_create_request.lock().unwrap() = Some(request);
            Ok(response)
        }

        async fn list_proposals(
            &self,
            _status: Option<&str>,
        ) -> Result<Vec<OrcProposalSummary>, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(vec![OrcProposalSummary {
                action_id: "action_1".to_string(),
                authority: "strata_admin".to_string(),
                seq_no: 1,
                status: "pending".to_string(),
                signature_count: 1,
                threshold: 2,
            }])
        }

        async fn get_proposal(
            &self,
            action_id: &str,
        ) -> Result<OrcProposalDetail, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(OrcProposalDetail {
                action_id: action_id.to_string(),
                authority: "strata_admin".to_string(),
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: "pending".to_string(),
                signatures: vec![],
                threshold: 2,
            })
        }

        async fn submit_signature(
            &self,
            action_id: &str,
            request: SubmitSignatureRequest,
        ) -> Result<OrcSignatureResponse, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.last_submit_request.lock().unwrap() = Some((action_id.to_string(), request));
            Ok(OrcSignatureResponse {
                quorum_reached: false,
                signatures_count: 1,
                threshold: 2,
            })
        }
    }

    // ─── Tests ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_update_action() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = create_update_action(&mock, &action_hex, 1, &sig)
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.status, "pending");
        assert_eq!(result.signatures.len(), 1);
        assert_eq!(result.signatures[0].signer_pubkey, sig.signer_pubkey);

        let req = mock.last_create_request().expect("request sent");
        assert_eq!(req.seq_no, 1);
        assert_eq!(req.action_hex, action_hex);
    }

    #[tokio::test]
    async fn test_approve_action() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = approve_action(&mock, "action_1", &sig)
            .await
            .expect("should succeed");

        assert!(!result.quorum_reached);
        assert_eq!(result.signatures_count, 1);

        let (action_id, req) = mock.last_submit_request().expect("request sent");
        assert_eq!(action_id, "action_1");
        assert_eq!(req.signer_pubkey, sig.signer_pubkey);
    }

    #[tokio::test]
    async fn test_get_update_action() {
        let mock = MockOrchestratorClient::new();

        let result = get_update_action(&mock, "action_1")
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.authority, "strata_admin");
        assert_eq!(result.threshold, 2);
    }

    #[tokio::test]
    async fn test_list_proposals() {
        let mock = MockOrchestratorClient::new();

        let result = list_proposals(&mock, Some("pending"))
            .await
            .expect("should succeed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "pending");
        assert_eq!(result[0].threshold, 2);
    }

    #[tokio::test]
    async fn test_create_then_get_consistent() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let created = create_update_action(&mock, &action_hex, 1, &sig)
            .await
            .expect("should succeed");

        let detail = get_update_action(&mock, &created.action_id)
            .await
            .expect("should succeed");

        assert_eq!(created.authority, detail.authority);
        assert_eq!(created.seq_no, detail.seq_no);
    }

    #[tokio::test]
    async fn test_signature_is_verifiable() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let _result = create_update_action(&mock, &action_hex, 1, &sig)
            .await
            .expect("should succeed");

        let req = mock.last_create_request().expect("request sent");
        let sighash = signing::compute_sighash(1, &action_hex).expect("sighash ok");
        let verify = signing::verify_threshold(
            &[req.signer_pubkey],
            1,
            &[req.signature_hex],
            &sighash.sighash_hex,
        )
        .expect("verify ok");

        assert!(verify.valid);
    }

    #[tokio::test]
    async fn test_create_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = create_update_action(&mock, &action_hex, 1, &sig).await;

        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }

    #[tokio::test]
    async fn test_approve_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let result = approve_action(&mock, "action_1", &sig).await;

        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }
}
