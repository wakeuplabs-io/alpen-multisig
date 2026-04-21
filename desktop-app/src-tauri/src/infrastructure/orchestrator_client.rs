//! HTTP implementation of the `OrchestratorClient` trait.

use crate::application::orchestrator_client::{
    ApproveActionRequest, CreateProposalRequest, OrchestratorClient, OrchestratorError,
};
use crate::domain::proposal::Proposal;

pub struct HttpOrchestratorClient {
    base_url: String,
    client: reqwest::Client,
}

impl HttpOrchestratorClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
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
