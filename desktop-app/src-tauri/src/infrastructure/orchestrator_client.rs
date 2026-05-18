//! HTTP implementation of the `OrchestratorClient` trait.

use crate::application::orchestrator_client::{
    ApproveActionRequest, CompleteOrchestratorAuthRequest, CreateProposalRequest,
    NextSeqNoResponse, OrchestratorAuthChallenge, OrchestratorAuthSession, OrchestratorClient,
    OrchestratorError, ProposalListResponse, StartOrchestratorAuthRequest,
    TransitionProposalRequest,
};
use crate::domain::proposal::Proposal;

pub struct HttpOrchestratorClient {
    base_url: String,
    client: reqwest::Client,
    token: Option<String>,
}

impl HttpOrchestratorClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            token: None,
        }
    }

    pub fn with_bearer_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    fn with_auth_headers(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, OrchestratorError> {
        let Some(token) = &self.token else {
            return Err(OrchestratorError::Request(
                "missing orchestrator bearer token".to_string(),
            ));
        };

        Ok(request.header("authorization", format!("Bearer {token}")))
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
    async fn auth_challenge(
        &self,
        request: StartOrchestratorAuthRequest,
    ) -> Result<OrchestratorAuthChallenge, OrchestratorError> {
        let req = self
            .client
            .post(format!("{}/auth/challenge", self.base_url))
            .json(&request);
        self.send_and_parse(req).await
    }

    async fn auth_verify(
        &self,
        request: CompleteOrchestratorAuthRequest,
    ) -> Result<OrchestratorAuthSession, OrchestratorError> {
        let req = self
            .client
            .post(format!("{}/auth/verify", self.base_url))
            .json(&request);
        self.send_and_parse(req).await
    }

    async fn auth_logout(&self) -> Result<(), OrchestratorError> {
        let req =
            self.with_auth_headers(self.client.post(format!("{}/auth/logout", self.base_url)))?;
        let res = req
            .send()
            .await
            .map_err(|e| OrchestratorError::Request(e.to_string()))?;
        if !res.status().is_success() {
            let status = res.status().as_u16();
            let message = res.text().await.unwrap_or_default();
            return Err(OrchestratorError::Backend { status, message });
        }
        Ok(())
    }

    async fn create_proposal(
        &self,
        request: CreateProposalRequest,
    ) -> Result<Proposal, OrchestratorError> {
        let req = self.with_auth_headers(
            self.client
                .post(format!("{}/proposals", self.base_url))
                .json(&request),
        )?;
        self.send_and_parse(req).await
    }

    async fn get_proposal(&self, action_id: &str) -> Result<Proposal, OrchestratorError> {
        let req = self.with_auth_headers(
            self.client
                .get(format!("{}/proposals/{}", self.base_url, action_id)),
        )?;
        self.send_and_parse(req).await
    }

    async fn approve_action(
        &self,
        action_id: &str,
        request: ApproveActionRequest,
    ) -> Result<Proposal, OrchestratorError> {
        let req = self.with_auth_headers(
            self.client
                .post(format!("{}/proposals/{}/approve", self.base_url, action_id))
                .json(&request),
        )?;
        self.send_and_parse(req).await
    }

    async fn transition_to_approved(
        &self,
        action_id: &str,
        request: TransitionProposalRequest,
    ) -> Result<Proposal, OrchestratorError> {
        let req = self.with_auth_headers(
            self.client
                .patch(format!("{}/proposals/{}", self.base_url, action_id))
                .json(&request),
        )?;
        self.send_and_parse(req).await
    }

    async fn list_proposals(
        &self,
        status: Option<&str>,
    ) -> Result<Vec<Proposal>, OrchestratorError> {
        let mut req = self.client.get(format!("{}/proposals", self.base_url));
        if let Some(status) = status {
            req = req.query(&[("status", status)]);
        }
        let req = self.with_auth_headers(req)?;
        let response: ProposalListResponse = self.send_and_parse(req).await?;
        Ok(response.proposals)
    }

    async fn get_next_seq_no(&self) -> Result<u64, OrchestratorError> {
        let req = self.with_auth_headers(
            self.client
                .get(format!("{}/proposals/next-seq-no", self.base_url)),
        )?;
        let response: NextSeqNoResponse = self.send_and_parse(req).await?;
        Ok(response.next_seq_no)
    }

    async fn claim_broadcast(&self, action_id: &str) -> Result<Proposal, OrchestratorError> {
        let req = self.with_auth_headers(self.client.post(format!(
            "{}/proposals/{action_id}/broadcast/claim",
            self.base_url
        )))?;
        self.send_and_parse(req).await
    }

    async fn report_broadcast_progress(
        &self,
        action_id: &str,
        request: crate::application::orchestrator_client::ReportBroadcastProgressRequest,
    ) -> Result<Proposal, OrchestratorError> {
        let req = self.with_auth_headers(
            self.client
                .patch(format!("{}/proposals/{action_id}/broadcast", self.base_url))
                .json(&request),
        )?;
        self.send_and_parse(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_proposal_requires_bearer_context() {
        let client = HttpOrchestratorClient::new("http://127.0.0.1:9".to_string());

        let err = client
            .create_proposal(CreateProposalRequest {
                seq_no: 1,
                action_hex: "deadbeef".to_string(),
                signer_pubkey: "02aa".to_string(),
                signature_hex: "sig".to_string(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, OrchestratorError::Request(_)));
    }

    #[tokio::test]
    async fn create_proposal_with_bearer_attempts_http_request() {
        let client = HttpOrchestratorClient::new("http://127.0.0.1:9".to_string())
            .with_bearer_token("test-token".to_string());

        let err = client
            .create_proposal(CreateProposalRequest {
                seq_no: 1,
                action_hex: "deadbeef".to_string(),
                signer_pubkey: "02aa".to_string(),
                signature_hex: "sig".to_string(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::Request(_)));
    }
}
