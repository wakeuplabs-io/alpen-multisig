//! Orchestrator client — trait, transport types, and HTTP implementation.
//!
//! This module owns the contract between the desktop app and the orchestrator
//! backend: the trait that defines available operations, the DTOs that define
//! the JSON shapes exchanged, and the real HTTP implementation.

use serde::{Deserialize, Serialize};
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

/// Request to submit a signature for an existing proposal.
#[derive(Debug, Serialize)]
pub(crate) struct SubmitSignatureRequest {
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

/// Response from submitting a signature.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SignatureResponse {
    pub(crate) quorum_reached: bool,
    pub(crate) signatures_count: u32,
    pub(crate) threshold: u32,
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
