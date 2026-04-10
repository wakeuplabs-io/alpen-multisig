//! Orchestrator client — trait, transport types, and HTTP implementation.
//!
//! This module owns the contract between the desktop app and the orchestrator
//! backend: the trait that defines available operations, the DTOs that define
//! the JSON shapes exchanged, and the real HTTP implementation.

use serde::{Deserialize, Serialize};

// ─── Errors ─────────────────────────────────────────────────────────────────

/// Errors from orchestrator communication.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
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
pub struct CreateProposalRequest {
    pub authority: String,
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// Request to approve (add signature to) an existing proposal.
#[derive(Debug, Serialize)]
pub struct ApproveActionRequest {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// A proposal as returned by the orchestrator.
#[derive(Debug, Clone, Deserialize)]
pub struct Proposal {
    pub action_id: String,
    pub seq_no: u64,
    pub authority: String,
    pub status: String,
    pub action_hex: String,
    pub signatures: Vec<ProposalSignature>,
}

/// A signature on a proposal.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProposalSignature {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

// ─── Trait ───────────────────────────────────────────────────────────────────

/// Abstracts the orchestrator HTTP API.
#[async_trait::async_trait]
pub trait OrchestratorClient: Send + Sync {
    /// Create a new proposal with the first signature.
    async fn create_proposal(
        &self,
        request: CreateProposalRequest,
    ) -> Result<Proposal, OrchestratorError>;

    /// Get full details of a specific proposal.
    async fn get_proposal(&self, action_id: &str) -> Result<Proposal, OrchestratorError>;

    /// Submit an approval signature for an existing proposal.
    async fn approve_action(
        &self,
        action_id: &str,
        request: ApproveActionRequest,
    ) -> Result<Proposal, OrchestratorError>;
}

// ─── HTTP Implementation ────────────────────────────────────────────────────

/// Real orchestrator client that communicates via HTTP/reqwest.
pub struct HttpOrchestratorClient {
    base_url: String,
    client: reqwest::Client,
}

impl HttpOrchestratorClient {
    /// Create a new HTTP client for the orchestrator.
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
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
    ) -> Result<Proposal, OrchestratorError> {
        let req = self
            .client
            .post(format!("{}/proposals", self.base_url))
            .json(&request);
        self.send_and_parse(req).await
    }

    async fn get_proposal(&self, action_id: &str) -> Result<Proposal, OrchestratorError> {
        let req = self
            .client
            .get(format!("{}/proposals/{}", self.base_url, action_id));
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
        self.send_and_parse(req).await
    }
}
