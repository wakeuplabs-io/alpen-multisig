//! Orchestrator client — trait and HTTP implementation.
//!
//! Abstracts communication with the orchestrator backend so the application
//! layer can be tested with a mock implementation.

use crate::application::types::{
    CreateProposalRequest, ProposalDetail, ProposalResponse, ProposalSummary, SignatureResponse,
    SubmitSignatureRequest,
};
use std::sync::{Arc, Mutex};

// ─── Errors ─────────────────────────────────────────────────────────────────

/// Errors from orchestrator communication.
#[derive(Debug, thiserror::Error)]
pub(crate) enum OrchestratorError {
    #[error("HTTP request failed: {0}")]
    Request(String),
    #[error("Orchestrator returned error {status}: {message}")]
    Backend { status: u16, message: String },
    #[error("Failed to deserialize response: {0}")]
    Deserialization(String),
}

// ─── Trait ───────────────────────────────────────────────────────────────────

/// Abstracts the orchestrator HTTP API.
/// Real implementation uses reqwest; test mock uses in-memory state.
#[async_trait::async_trait]
pub(crate) trait OrchestratorClient: Send + Sync {
    /// Create a new proposal with the first signature.
    async fn create_proposal(
        &self,
        request: CreateProposalRequest,
    ) -> Result<ProposalResponse, OrchestratorError>;

    /// List proposals for an authority, optionally filtered by status.
    async fn list_proposals(
        &self,
        authority: &str,
        status: Option<&str>,
    ) -> Result<Vec<ProposalSummary>, OrchestratorError>;

    /// Get full details of a specific proposal.
    async fn get_proposal(&self, action_id: &str) -> Result<ProposalDetail, OrchestratorError>;

    /// Submit a signature for an existing proposal.
    async fn submit_signature(
        &self,
        action_id: &str,
        request: SubmitSignatureRequest,
    ) -> Result<SignatureResponse, OrchestratorError>;
}

// ─── HTTP Implementation ────────────────────────────────────────────────────

/// Real orchestrator client that communicates via HTTP/reqwest.
/// Bearer token is injected from the shared session state.
pub(crate) struct HttpOrchestratorClient {
    base_url: String,
    session_token: Arc<Mutex<Option<String>>>,
    client: reqwest::Client,
}

impl HttpOrchestratorClient {
    /// Create a new HTTP client for the orchestrator.
    pub(crate) fn new(base_url: String, session_token: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            base_url,
            session_token,
            client: reqwest::Client::new(),
        }
    }

    /// Get the current bearer token or return an error.
    fn token(&self) -> Result<String, OrchestratorError> {
        self.session_token
            .lock()
            .map_err(|e| OrchestratorError::Request(format!("failed to lock session token: {e}")))?
            .clone()
            .ok_or_else(|| OrchestratorError::Request("not authenticated".to_string()))
    }

    /// Send a request and parse the JSON response, handling errors uniformly.
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
    ) -> Result<ProposalResponse, OrchestratorError> {
        let token = self.token()?;
        let req = self
            .client
            .post(format!("{}/proposals", self.base_url))
            .bearer_auth(token)
            .json(&request);
        self.send_and_parse(req).await
    }

    async fn list_proposals(
        &self,
        authority: &str,
        status: Option<&str>,
    ) -> Result<Vec<ProposalSummary>, OrchestratorError> {
        let token = self.token()?;
        let mut req = self
            .client
            .get(format!("{}/proposals", self.base_url))
            .bearer_auth(token)
            .query(&[("authority", authority)]);

        if let Some(s) = status {
            req = req.query(&[("status", s)]);
        }

        self.send_and_parse(req).await
    }

    async fn get_proposal(&self, action_id: &str) -> Result<ProposalDetail, OrchestratorError> {
        let token = self.token()?;
        let req = self
            .client
            .get(format!("{}/proposals/{}", self.base_url, action_id))
            .bearer_auth(token);
        self.send_and_parse(req).await
    }

    async fn submit_signature(
        &self,
        action_id: &str,
        request: SubmitSignatureRequest,
    ) -> Result<SignatureResponse, OrchestratorError> {
        let token = self.token()?;
        let req = self
            .client
            .post(format!(
                "{}/proposals/{}/signatures",
                self.base_url, action_id
            ))
            .bearer_auth(token)
            .json(&request);
        self.send_and_parse(req).await
    }
}
