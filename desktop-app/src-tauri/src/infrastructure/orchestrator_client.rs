//! HTTP implementation of the `OrchestratorClient` trait.

use bitcoin::secp256k1::{Message, SecretKey, SECP256K1};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::orchestrator_client::{
    ApproveActionRequest, CreateProposalRequest, OrchestratorClient, OrchestratorError,
};
use crate::domain::proposal::Proposal;

pub struct HttpOrchestratorClient {
    base_url: String,
    client: reqwest::Client,
    session_token: Option<String>,
    ephemeral_secret_key: Option<SecretKey>,
}

impl HttpOrchestratorClient {
    /// Unauthenticated client — for tests or pre-session use.
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            session_token: None,
            ephemeral_secret_key: None,
        }
    }

    /// Authenticated client — adds request signatures on every call.
    pub fn new_authenticated(
        base_url: String,
        session_token: String,
        ephemeral_secret_key: SecretKey,
    ) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            session_token: Some(session_token),
            ephemeral_secret_key: Some(ephemeral_secret_key),
        }
    }

    /// Attach auth headers to a request builder if a session is active.
    /// Returns `OrchestratorError::Request` if auth is required but not set.
    fn sign_request(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, OrchestratorError> {
        let (token, eph_sk) = match (&self.session_token, &self.ephemeral_secret_key) {
            (Some(t), Some(k)) => (t, k),
            _ => return Ok(builder),
        };

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| OrchestratorError::Request(format!("system clock error: {e}")))?
            .as_millis() as i64;

        let mut hasher = Sha256::new();
        hasher.update(b"alpen-request:v1");
        hasher.update(token.as_bytes());
        hasher.update(timestamp_ms.to_be_bytes());
        let hash = hasher.finalize();

        let msg = Message::from_digest_slice(&hash)
            .map_err(|e| OrchestratorError::Request(format!("hash error: {e}")))?;
        let sig = SECP256K1.sign_ecdsa(&msg, eph_sk);
        let sig_hex = hex::encode(sig.serialize_compact());

        Ok(builder
            .bearer_auth(token)
            .header("x-session-timestamp", timestamp_ms.to_string())
            .header("x-request-sig", sig_hex))
    }

    async fn send_and_parse<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, OrchestratorError> {
        let res = request
            .send()
            .await
            .map_err(|e| OrchestratorError::Request(e.to_string()))?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let message = res.text().await.unwrap_or_default();
            return Err(OrchestratorError::Backend { status, message });
        }

        res.json()
            .await
            .map_err(|e| OrchestratorError::Deserialization(e.to_string()))
    }
}

#[async_trait::async_trait]
impl OrchestratorClient for HttpOrchestratorClient {
    async fn create_proposal(
        &self,
        request: CreateProposalRequest,
    ) -> Result<Proposal, OrchestratorError> {
        let req = self
            .client
            .post(format!("{}/proposals", self.base_url))
            .json(&request);
        let req = self.sign_request(req)?;
        self.send_and_parse(req).await
    }

    async fn get_proposal(&self, action_id: &str) -> Result<Proposal, OrchestratorError> {
        let req = self
            .client
            .get(format!("{}/proposals/{}", self.base_url, action_id));
        let req = self.sign_request(req)?;
        self.send_and_parse(req).await
    }

    async fn approve_action(
        &self,
        action_id: &str,
        request: ApproveActionRequest,
    ) -> Result<Proposal, OrchestratorError> {
        let req = self
            .client
            .post(format!("{}/proposals/{}/approve", self.base_url, action_id))
            .json(&request);
        let req = self.sign_request(req)?;
        self.send_and_parse(req).await
    }
}
