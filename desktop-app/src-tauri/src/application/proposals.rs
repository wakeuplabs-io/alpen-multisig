//! Proposal management — application layer entry point for the desktop app.
//!
//! Public API mirrors the PRD's `MultisigBackend` trait semantics:
//! - `create_update_action(action_hex, seq_no, signature)` — propose + first signature
//! - `approve_action(action_id, signature)` — add approval signature
//! - `get_update_action(action_id)` — fetch proposal detail
//!
//! Authority is implicit — bound to the authenticated session, not passed per call.
//! Signing and action encoding happen before reaching this layer.

use bitcoin::secp256k1::{Message, SecretKey, SECP256K1};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::orchestrator_client::{
    ApproveActionRequest, CreateProposalRequest, OrchestratorClient, OrchestratorError,
};
use crate::domain::authority::Authority;
use crate::domain::proposal::{Proposal, Signature};

/// Errors that can occur during proposal operations.
#[derive(Debug, thiserror::Error)]
pub enum ProposalError {
    #[error("Orchestrator error: {0}")]
    Orchestrator(#[from] OrchestratorError),
}

/// Session-authenticated listing endpoint used by the desktop UI.
///
/// Requires an active session: `session_token`, `authority`, and `ephemeral_secret_key`
/// are extracted from Tauri state before calling this function.
pub async fn fetch_proposals(
    backend_url: &str,
    session_token: &str,
    selected_authority: &str,
    ephemeral_secret_key: &SecretKey,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock error: {e}"))?
        .as_millis() as i64;

    let mut hasher = Sha256::new();
    hasher.update(b"alpen-request:v1");
    hasher.update(session_token.as_bytes());
    hasher.update(timestamp_ms.to_be_bytes());
    let hash = hasher.finalize();

    let msg = Message::from_digest_slice(&hash).map_err(|e| format!("request hash error: {e}"))?;
    let sig = SECP256K1.sign_ecdsa(&msg, ephemeral_secret_key);
    let sig_hex = hex::encode(sig.serialize_compact());

    let client = reqwest::Client::new();
    let mut req = client
        .get(format!("{backend_url}/proposals"))
        .bearer_auth(session_token)
        .header("x-session-authority", selected_authority)
        .header("x-session-timestamp", timestamp_ms.to_string())
        .header("x-request-sig", sig_hex);

    if let Some(s) = status {
        req = req.query(&[("status", s)]);
    }

    let res = req.send().await.map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Request failed: {}", res.status()));
    }

    res.json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())
}

/// Create a new action and store the creator's signature.
///
/// Mirrors PRD: `create_update_action(action, seq, sig)`.
///
/// Callers are responsible for encoding the action to SSZ hex before calling this
/// function (`infrastructure::action_codec::encode_hex`).
pub async fn create_update_action(
    client: &dyn OrchestratorClient,
    authority: Authority,
    action_hex: &str,
    seq_no: u64,
    signature: &Signature,
) -> Result<Proposal, ProposalError> {
    let request = CreateProposalRequest {
        authority: authority.as_str().to_string(),
        seq_no,
        action_hex: action_hex.to_string(),
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
    };

    let proposal = client.create_proposal(request).await?;
    Ok(proposal)
}

/// Append an approval signature for an existing action.
///
/// Mirrors PRD: `approve_action(id, sig)`.
pub async fn approve_action(
    client: &dyn OrchestratorClient,
    action_id: &str,
    signature: &Signature,
) -> Result<Proposal, ProposalError> {
    let request = ApproveActionRequest {
        signer_pubkey: signature.signer_pubkey.clone(),
        signature_hex: signature.signature_hex.clone(),
    };

    let proposal = client.approve_action(action_id, request).await?;
    Ok(proposal)
}

/// Fetch the action payload and details.
///
/// Mirrors PRD: `get_update_action(id)`.
pub async fn get_update_action(
    client: &dyn OrchestratorClient,
    action_id: &str,
) -> Result<Proposal, ProposalError> {
    let proposal = client.get_proposal(action_id).await?;
    Ok(proposal)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::orchestrator_client::OrchestratorError;
    use crate::domain::action::{Action, CompressedPubKey, MultisigUpdate};
    use crate::domain::authority::Authority;
    use crate::domain::proposal::{Proposal as OrcProposal, ProposalSignature};
    use crate::infrastructure::action_codec;
    use crate::infrastructure::signing;
    use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};
    use rand::rngs::OsRng;
    use std::num::NonZeroU8;
    use std::sync::Mutex;

    // ─── Test helpers ───────────────────────────────────────────────────────

    fn generate_test_keypair() -> (String, String) {
        let sk = SecretKey::new(&mut OsRng);
        let pk = PublicKey::from_secret_key(SECP256K1, &sk);
        (hex::encode(sk.secret_bytes()), hex::encode(pk.serialize()))
    }

    /// Builds a sample `Action::MultisigUpdate` via domain types only.
    fn demo_action() -> Action {
        let demo_bytes = [0x42u8; 32];
        let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
        let new_signer_pk = PublicKey::from_secret_key(SECP256K1, &demo_sk);
        let new_signer = CompressedPubKey::new(new_signer_pk.serialize());
        Action::MultisigUpdate(MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: vec![new_signer],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).expect("non-zero"),
        })
    }

    fn demo_action_hex() -> String {
        action_codec::encode_hex(&demo_action()).expect("encode ok")
    }

    fn sign_action(secret_key_hex: &str, seq_no: u64, action_hex: &str) -> Signature {
        let sighash = signing::compute_sighash(seq_no, action_hex).expect("sighash ok");
        let sig = signing::sign_sighash(secret_key_hex, &sighash.sighash_hex).expect("sign ok");
        Signature {
            signer_pubkey: sig.public_key_hex,
            signature_hex: sig.signature_hex,
        }
    }

    struct MockOrchestratorClient {
        last_create_request: Mutex<Option<CreateProposalRequest>>,
        last_approve_request: Mutex<Option<(String, ApproveActionRequest)>>,
        should_fail: bool,
    }

    impl MockOrchestratorClient {
        fn new() -> Self {
            Self {
                last_create_request: Mutex::new(None),
                last_approve_request: Mutex::new(None),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                last_create_request: Mutex::new(None),
                last_approve_request: Mutex::new(None),
                should_fail: true,
            }
        }

        fn last_create_request(&self) -> Option<CreateProposalRequest> {
            self.last_create_request.lock().unwrap().take()
        }

        fn last_approve_request(&self) -> Option<(String, ApproveActionRequest)> {
            self.last_approve_request.lock().unwrap().take()
        }
    }

    #[async_trait::async_trait]
    impl OrchestratorClient for MockOrchestratorClient {
        async fn create_proposal(
            &self,
            request: CreateProposalRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            let response = OrcProposal {
                action_id: format!("action_{}", request.seq_no),
                authority: Authority::StrataAdmin,
                seq_no: request.seq_no,
                action_hex: request.action_hex.clone(),
                status: "pending".to_string(),
                signatures: vec![ProposalSignature {
                    signer_pubkey: request.signer_pubkey.clone(),
                    signature_hex: request.signature_hex.clone(),
                }],
            };
            *self.last_create_request.lock().unwrap() = Some(request);
            Ok(response)
        }

        async fn get_proposal(&self, action_id: &str) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: "pending".to_string(),
                signatures: vec![],
            })
        }

        async fn approve_action(
            &self,
            action_id: &str,
            request: ApproveActionRequest,
        ) -> Result<OrcProposal, OrchestratorError> {
            if self.should_fail {
                return Err(OrchestratorError::Backend {
                    status: 500,
                    message: "mock error".to_string(),
                });
            }
            *self.last_approve_request.lock().unwrap() = Some((action_id.to_string(), request));
            Ok(OrcProposal {
                action_id: action_id.to_string(),
                authority: Authority::StrataAdmin,
                seq_no: 1,
                action_hex: demo_action_hex(),
                status: "pending".to_string(),
                signatures: vec![],
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

        let result = create_update_action(&mock, Authority::StrataAdmin, &action_hex, 1, &sig)
            .await
            .expect("should succeed");

        assert_eq!(result.action_id, "action_1");
        assert_eq!(result.status, "pending");
        assert_eq!(result.signatures.len(), 1);
        assert_eq!(result.signatures[0].signer_pubkey, sig.signer_pubkey);

        let req = mock.last_create_request().expect("request sent");
        assert_eq!(req.seq_no, 1);
        assert_eq!(req.action_hex, action_hex);
        assert_eq!(req.authority, "strata_admin");
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

        assert_eq!(result.action_id, "action_1");

        let (action_id, req) = mock.last_approve_request().expect("request sent");
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
        assert_eq!(result.authority, Authority::StrataAdmin);
    }

    #[tokio::test]
    async fn test_create_then_get_consistent() {
        let mock = MockOrchestratorClient::new();
        let (sk, _pk) = generate_test_keypair();
        let action_hex = demo_action_hex();
        let sig = sign_action(&sk, 1, &action_hex);

        let created = create_update_action(&mock, Authority::StrataAdmin, &action_hex, 1, &sig)
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

        let _result = create_update_action(&mock, Authority::StrataAdmin, &action_hex, 1, &sig)
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

        let result =
            create_update_action(&mock, Authority::StrataAdmin, &action_hex, 1, &sig).await;

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
