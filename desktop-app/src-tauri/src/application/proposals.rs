//! Proposal signing flow — orchestrates signing.rs + orchestrator client.
//!
//! These functions compose sighash computation, ECDSA signing, and orchestrator
//! communication into high-level operations (create proposal, sign proposal, etc.).

use crate::application::orchestrator_client::{OrchestratorClient, OrchestratorError};
use crate::signing;
use serde::{Deserialize, Serialize};

// ─── Errors ─────────────────────────────────────────────────────────────────

/// Errors that can occur during proposal operations.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProposalError {
    #[error("Signing failed: {0}")]
    Signing(String),
    #[error("Orchestrator error: {0}")]
    Orchestrator(#[from] OrchestratorError),
}

// ─── Transport DTOs ─────────────────────────────────────────────────────────

/// Request to create a proposal with initial signature.
#[derive(Debug, Serialize)]
pub(crate) struct CreateProposalRequest {
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Response from creating a proposal.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProposalResponse {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<SignatureInfo>,
}

/// Summary of a proposal for list views.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProposalSummary {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) status: String,
    pub(crate) signature_count: u32,
    pub(crate) threshold: u32,
}

/// Full proposal detail including all signatures.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProposalDetail {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<SignatureInfo>,
    pub(crate) threshold: u32,
}

/// A single signature on a proposal.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct SignatureInfo {
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Request to submit a signature for an existing proposal.
#[derive(Debug, Serialize)]
pub(crate) struct SubmitSignatureRequest {
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Response from submitting a signature.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SignatureResponse {
    pub(crate) quorum_reached: bool,
    pub(crate) signatures_count: u32,
    pub(crate) threshold: u32,
}

// ─── Production functions ───────────────────────────────────────────────────

/// Compute sighash and sign it. Shared by create_proposal and sign_proposal.
fn compute_and_sign(
    secret_key_hex: &str,
    seq_no: u64,
    action_hex: &str,
) -> Result<(String, String, String), ProposalError> {
    let sighash = signing::compute_sighash(seq_no, action_hex).map_err(ProposalError::Signing)?;
    let sig_result = signing::sign_sighash(secret_key_hex, &sighash.sighash_hex)
        .map_err(ProposalError::Signing)?;
    Ok((
        sighash.sighash_hex,
        sig_result.public_key_hex,
        sig_result.signature_hex,
    ))
}

/// Create a proposal: compute sighash, sign it, send to orchestrator with first signature.
pub(crate) async fn create_proposal(
    client: &dyn OrchestratorClient,
    secret_key_hex: &str,
    authority: &str,
    seq_no: u64,
    action_hex: &str,
) -> Result<ProposalResponse, ProposalError> {
    let (_sighash, pubkey, signature) = compute_and_sign(secret_key_hex, seq_no, action_hex)?;

    let request = CreateProposalRequest {
        authority: authority.to_string(),
        seq_no,
        action_hex: action_hex.to_string(),
        signer_pubkey: pubkey,
        signature_hex: signature,
    };

    Ok(client.create_proposal(request).await?)
}

/// Sign an existing proposal: compute sighash, sign it, submit signature to orchestrator.
pub(crate) async fn sign_proposal(
    client: &dyn OrchestratorClient,
    secret_key_hex: &str,
    action_id: &str,
    action_hex: &str,
    seq_no: u64,
) -> Result<SignatureResponse, ProposalError> {
    let (_sighash, pubkey, signature) = compute_and_sign(secret_key_hex, seq_no, action_hex)?;

    let request = SubmitSignatureRequest {
        signer_pubkey: pubkey,
        signature_hex: signature,
    };

    Ok(client.submit_signature(action_id, request).await?)
}

/// List proposals for an authority, optionally filtered by status.
pub(crate) async fn list_proposals(
    client: &dyn OrchestratorClient,
    authority: &str,
    status: Option<&str>,
) -> Result<Vec<ProposalSummary>, ProposalError> {
    Ok(client.list_proposals(authority, status).await?)
}

/// Get full details of a specific proposal.
pub(crate) async fn get_proposal(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<ProposalDetail, ProposalError> {
    Ok(client.get_proposal(action_id).await?)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::orchestrator_client::OrchestratorError;
    use crate::signing;
    use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
    use borsh::BorshSerialize;
    use rand::rngs::OsRng;
    use std::num::NonZero;
    use std::sync::Mutex;
    use strata_asm_params::Role;
    use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate;
    use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
    use strata_crypto::keys::compressed::CompressedPublicKey;
    use strata_crypto::threshold_signature::ThresholdConfigUpdate;

    // ─── Test helpers ───────────────────────────────────────────────────────

    struct TestKeypair {
        secret_key_hex: String,
        public_key_hex: String,
    }

    fn generate_test_keypair() -> TestKeypair {
        let sk = SecretKey::new(&mut OsRng);
        let pk = PublicKey::from_secret_key(SECP256K1, &sk);
        TestKeypair {
            secret_key_hex: hex::encode(sk.secret_bytes()),
            public_key_hex: hex::encode(pk.serialize()),
        }
    }

    fn build_demo_action() -> MultisigAction {
        let demo_bytes = [0x42u8; 32];
        let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
        let new_signer = CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, &demo_sk));
        let config_update = ThresholdConfigUpdate::new(
            vec![new_signer],
            vec![],
            NonZero::new(2).expect("non-zero"),
        );
        let multisig_update = MultisigUpdate::new(config_update, Role::StrataAdministrator);
        MultisigAction::Update(UpdateAction::Multisig(multisig_update))
    }

    fn demo_action_hex() -> String {
        hex::encode(borsh::to_vec(&build_demo_action()).expect("action borsh-serializes"))
    }

    /// Mock orchestrator client that records calls and returns canned responses.
    struct MockOrchestratorClient {
        /// Stores the last CreateProposalRequest received.
        last_create_request: Mutex<Option<CreateProposalRequest>>,
        /// Stores the last SubmitSignatureRequest received (with action_id).
        last_submit_request: Mutex<Option<(String, SubmitSignatureRequest)>>,
        /// Whether to return an error on next call.
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
        ) -> Result<ProposalResponse, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            let response = ProposalResponse {
                action_id: format!("action_{}", request.seq_no),
                authority: request.authority.clone(),
                seq_no: request.seq_no,
                action_hex: request.action_hex.clone(),
                status: "pending".to_string(),
                signatures: vec![SignatureInfo {
                    signer_pubkey: request.signer_pubkey.clone(),
                    signature_hex: request.signature_hex.clone(),
                }],
            };
            *self.last_create_request.lock().unwrap() = Some(request);
            Ok(response)
        }

        async fn list_proposals(
            &self,
            authority: &str,
            _status: Option<&str>,
        ) -> Result<Vec<ProposalSummary>, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(vec![ProposalSummary {
                action_id: "action_1".to_string(),
                authority: authority.to_string(),
                seq_no: 1,
                status: "pending".to_string(),
                signature_count: 1,
                threshold: 2,
            }])
        }

        async fn get_proposal(&self, action_id: &str) -> Result<ProposalDetail, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(ProposalDetail {
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
        ) -> Result<SignatureResponse, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.last_submit_request.lock().unwrap() = Some((action_id.to_string(), request));
            Ok(SignatureResponse {
                quorum_reached: false,
                signatures_count: 1,
                threshold: 2,
            })
        }
    }

    // ─── Tests ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_proposal_computes_sighash_signs_and_sends() {
        let mock = MockOrchestratorClient::new();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();

        let result = create_proposal(&mock, &keys.secret_key_hex, "strata_admin", 1, &action_hex)
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.authority, "strata_admin");
        assert_eq!(result.seq_no, 1);
        assert_eq!(result.status, "pending");
        assert_eq!(result.signatures.len(), 1);

        // Verify the mock received the correct data
        let req = mock
            .last_create_request()
            .expect("should have received request");
        assert_eq!(req.authority, "strata_admin");
        assert_eq!(req.seq_no, 1);
        assert_eq!(req.action_hex, action_hex);
        assert_eq!(req.signer_pubkey, keys.public_key_hex);
        assert!(!req.signature_hex.is_empty());
    }

    #[tokio::test]
    async fn test_sign_proposal_computes_sighash_signs_and_submits() {
        let mock = MockOrchestratorClient::new();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();

        let result = sign_proposal(&mock, &keys.secret_key_hex, "action_1", &action_hex, 1)
            .await
            .expect("should succeed");

        assert!(!result.quorum_reached);
        assert_eq!(result.signatures_count, 1);

        let (action_id, req) = mock
            .last_submit_request()
            .expect("should have received request");
        assert_eq!(action_id, "action_1");
        assert_eq!(req.signer_pubkey, keys.public_key_hex);
        assert!(!req.signature_hex.is_empty());
    }

    #[tokio::test]
    async fn test_list_proposals_returns_filtered_results() {
        let mock = MockOrchestratorClient::new();

        let result = list_proposals(&mock, "strata_admin", Some("pending"))
            .await
            .expect("should succeed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].authority, "strata_admin");
        assert_eq!(result[0].status, "pending");
    }

    #[tokio::test]
    async fn test_get_proposal_returns_detail() {
        let mock = MockOrchestratorClient::new();

        let result = get_proposal(&mock, "action_1")
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.authority, "strata_admin");
        assert_eq!(result.threshold, 2);
    }

    #[tokio::test]
    async fn test_create_then_get_proposal_data_consistent() {
        let mock = MockOrchestratorClient::new();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();

        let created = create_proposal(&mock, &keys.secret_key_hex, "strata_admin", 1, &action_hex)
            .await
            .expect("create should succeed");

        let detail = get_proposal(&mock, &created.action_id)
            .await
            .expect("get should succeed");

        assert_eq!(created.authority, detail.authority);
        assert_eq!(created.seq_no, detail.seq_no);
        assert_eq!(created.action_hex, detail.action_hex);
    }

    #[tokio::test]
    async fn test_create_proposal_signature_is_verifiable() {
        let mock = MockOrchestratorClient::new();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();

        let _result = create_proposal(&mock, &keys.secret_key_hex, "strata_admin", 1, &action_hex)
            .await
            .expect("should succeed");

        let req = mock
            .last_create_request()
            .expect("should have received request");

        // Recompute sighash and verify the signature
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
    async fn test_create_proposal_invalid_action_hex_fails() {
        let mock = MockOrchestratorClient::new();
        let keys = generate_test_keypair();

        let result = create_proposal(
            &mock,
            &keys.secret_key_hex,
            "strata_admin",
            1,
            "not_valid_hex",
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProposalError::Signing(_)));
    }

    #[tokio::test]
    async fn test_create_proposal_invalid_secret_key_fails() {
        let mock = MockOrchestratorClient::new();
        let action_hex = demo_action_hex();

        let result = create_proposal(&mock, "invalid_key", "strata_admin", 1, &action_hex).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProposalError::Signing(_)));
    }

    #[tokio::test]
    async fn test_create_proposal_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();

        let result =
            create_proposal(&mock, &keys.secret_key_hex, "strata_admin", 1, &action_hex).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProposalError::Orchestrator(_)));
    }

    #[tokio::test]
    async fn test_sign_proposal_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();

        let result = sign_proposal(&mock, &keys.secret_key_hex, "action_1", &action_hex, 1).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProposalError::Orchestrator(_)));
    }
}
