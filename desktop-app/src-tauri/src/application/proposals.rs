//! Proposal management — application layer entry point for the desktop app.
//!
//! These functions are the business API consumed by Tauri commands, CLI, or any
//! other interface. They receive already-signed data (signing happens externally,
//! e.g. hardware wallet or software signer) and delegate persistence/coordination
//! to the orchestrator client.
//!
//! Domain types are defined here — orchestrator DTOs never leak to consumers.

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

/// A signature attached to a proposal.
#[derive(Debug, Clone)]
pub(crate) struct Signature {
    pub(crate) signer_pubkey: String,
    pub(crate) signature_hex: String,
}

/// Result of creating a proposal.
#[derive(Debug, Clone)]
pub(crate) struct Proposal {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<Signature>,
}

/// Summary of a proposal for list views.
#[derive(Debug, Clone)]
pub(crate) struct ProposalSummary {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) status: String,
    pub(crate) signature_count: u32,
    pub(crate) threshold: u32,
}

/// Full proposal detail including all signatures.
#[derive(Debug, Clone)]
pub(crate) struct ProposalDetail {
    pub(crate) action_id: String,
    pub(crate) authority: String,
    pub(crate) seq_no: u64,
    pub(crate) action_hex: String,
    pub(crate) status: String,
    pub(crate) signatures: Vec<Signature>,
    pub(crate) threshold: u32,
}

/// Result of submitting a signature.
#[derive(Debug, Clone)]
pub(crate) struct SignatureResult {
    pub(crate) quorum_reached: bool,
    pub(crate) signatures_count: u32,
    pub(crate) threshold: u32,
}

// ─── Production functions ───────────────────────────────────────────────────

/// Create a proposal with the first signature.
///
/// The caller is responsible for computing the sighash and signing it
/// (via `signing::compute_sighash` + hardware wallet or `signing::sign_sighash`).
/// This function only handles coordination with the orchestrator.
pub(crate) async fn create_proposal(
    client: &dyn OrchestratorClient,
    authority: &str,
    seq_no: u64,
    action_hex: &str,
    signer_pubkey: &str,
    signature_hex: &str,
) -> Result<Proposal, ProposalError> {
    let request = CreateProposalRequest {
        authority: authority.to_string(),
        seq_no,
        action_hex: action_hex.to_string(),
        signer_pubkey: signer_pubkey.to_string(),
        signature_hex: signature_hex.to_string(),
    };

    let res = client.create_proposal(request).await?;

    Ok(Proposal {
        action_id: res.action_id,
        authority: res.authority,
        seq_no: res.seq_no,
        action_hex: res.action_hex,
        status: res.status,
        signatures: res
            .signatures
            .into_iter()
            .map(|s| Signature {
                signer_pubkey: s.signer_pubkey,
                signature_hex: s.signature_hex,
            })
            .collect(),
    })
}

/// Submit a signature for an existing proposal.
///
/// The caller is responsible for signing the sighash externally.
pub(crate) async fn sign_proposal(
    client: &dyn OrchestratorClient,
    action_id: &str,
    signer_pubkey: &str,
    signature_hex: &str,
) -> Result<SignatureResult, ProposalError> {
    let request = SubmitSignatureRequest {
        signer_pubkey: signer_pubkey.to_string(),
        signature_hex: signature_hex.to_string(),
    };

    let res = client.submit_signature(action_id, request).await?;

    Ok(SignatureResult {
        quorum_reached: res.quorum_reached,
        signatures_count: res.signatures_count,
        threshold: res.threshold,
    })
}

/// List proposals for an authority, optionally filtered by status.
pub(crate) async fn list_proposals(
    client: &dyn OrchestratorClient,
    authority: &str,
    status: Option<&str>,
) -> Result<Vec<ProposalSummary>, ProposalError> {
    let items = client.list_proposals(authority, status).await?;

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

/// Get full details of a specific proposal.
pub(crate) async fn get_proposal(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<ProposalDetail, ProposalError> {
    let res = client.get_proposal(action_id).await?;

    Ok(ProposalDetail {
        action_id: res.action_id,
        authority: res.authority,
        seq_no: res.seq_no,
        action_hex: res.action_hex,
        status: res.status,
        signatures: res
            .signatures
            .into_iter()
            .map(|s| Signature {
                signer_pubkey: s.signer_pubkey,
                signature_hex: s.signature_hex,
            })
            .collect(),
        threshold: res.threshold,
    })
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

    /// Sign externally (simulates what HW wallet or software signer would do).
    fn sign_action(secret_key_hex: &str, seq_no: u64, action_hex: &str) -> (String, String) {
        let sighash = signing::compute_sighash(seq_no, action_hex).expect("sighash ok");
        let sig = signing::sign_sighash(secret_key_hex, &sighash.sighash_hex).expect("sign ok");
        (sig.public_key_hex, sig.signature_hex)
    }

    /// Mock orchestrator client that records calls and returns canned responses.
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
        ) -> Result<Vec<OrcProposalSummary>, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(vec![OrcProposalSummary {
                action_id: "action_1".to_string(),
                authority: authority.to_string(),
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
    async fn test_create_proposal_sends_presigned_data() {
        let mock = MockOrchestratorClient::new();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();
        let (pubkey, signature) = sign_action(&keys.secret_key_hex, 1, &action_hex);

        let result = create_proposal(&mock, "strata_admin", 1, &action_hex, &pubkey, &signature)
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.authority, "strata_admin");
        assert_eq!(result.seq_no, 1);
        assert_eq!(result.status, "pending");
        assert_eq!(result.signatures.len(), 1);
        assert_eq!(result.signatures[0].signer_pubkey, pubkey);

        let req = mock
            .last_create_request()
            .expect("should have received request");
        assert_eq!(req.authority, "strata_admin");
        assert_eq!(req.seq_no, 1);
        assert_eq!(req.action_hex, action_hex);
        assert_eq!(req.signer_pubkey, pubkey);
        assert_eq!(req.signature_hex, signature);
    }

    #[tokio::test]
    async fn test_sign_proposal_submits_presigned_data() {
        let mock = MockOrchestratorClient::new();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();
        let (pubkey, signature) = sign_action(&keys.secret_key_hex, 1, &action_hex);

        let result = sign_proposal(&mock, "action_1", &pubkey, &signature)
            .await
            .expect("should succeed");

        assert!(!result.quorum_reached);
        assert_eq!(result.signatures_count, 1);

        let (action_id, req) = mock
            .last_submit_request()
            .expect("should have received request");
        assert_eq!(action_id, "action_1");
        assert_eq!(req.signer_pubkey, pubkey);
        assert_eq!(req.signature_hex, signature);
    }

    #[tokio::test]
    async fn test_list_proposals_returns_domain_types() {
        let mock = MockOrchestratorClient::new();

        let result = list_proposals(&mock, "strata_admin", Some("pending"))
            .await
            .expect("should succeed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].authority, "strata_admin");
        assert_eq!(result[0].status, "pending");
        assert_eq!(result[0].threshold, 2);
    }

    #[tokio::test]
    async fn test_get_proposal_returns_domain_detail() {
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
        let (pubkey, signature) = sign_action(&keys.secret_key_hex, 1, &action_hex);

        let created = create_proposal(&mock, "strata_admin", 1, &action_hex, &pubkey, &signature)
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
        let (pubkey, signature) = sign_action(&keys.secret_key_hex, 1, &action_hex);

        let _result = create_proposal(&mock, "strata_admin", 1, &action_hex, &pubkey, &signature)
            .await
            .expect("should succeed");

        let req = mock
            .last_create_request()
            .expect("should have received request");

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
    async fn test_create_proposal_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();
        let (pubkey, signature) = sign_action(&keys.secret_key_hex, 1, &action_hex);

        let result =
            create_proposal(&mock, "strata_admin", 1, &action_hex, &pubkey, &signature).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }

    #[tokio::test]
    async fn test_sign_proposal_backend_error_propagates() {
        let mock = MockOrchestratorClient::failing();
        let keys = generate_test_keypair();
        let action_hex = demo_action_hex();
        let (pubkey, signature) = sign_action(&keys.secret_key_hex, 1, &action_hex);

        let result = sign_proposal(&mock, "action_1", &pubkey, &signature).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProposalError::Orchestrator(_)
        ));
    }
}
