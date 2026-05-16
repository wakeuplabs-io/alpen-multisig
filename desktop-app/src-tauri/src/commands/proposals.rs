use desktop_app::application::orchestrator_auth;
use desktop_app::application::orchestrator_client::{OrchestratorClient, OrchestratorError};
use desktop_app::application::proposals;
use desktop_app::application::proposals::ProposalError;
use desktop_app::domain::proposal::{Proposal, ProposalSignature, Signature};
use desktop_app::infrastructure::orchestrator_client::HttpOrchestratorClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProposalInput {
    pub base_url: String,
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNextSeqNoInput {
    pub base_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProposalsInput {
    pub base_url: String,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProposalInput {
    pub base_url: String,
    pub action_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveProposalInput {
    pub base_url: String,
    pub action_id: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalSignatureDto {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDto {
    pub action_id: String,
    pub seq_no: u64,
    pub authority: String,
    pub status: String,
    pub required_signatures: u16,
    pub action_hex: String,
    pub signatures: Vec<ProposalSignatureDto>,
    pub broadcast_status: String,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
}

/// IPC payload for broadcast commands — orchestrator owns RPC, operator key, and network (P-015).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastInput {
    pub base_url: String,
    pub action_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareBroadcastDto {
    pub action_id: String,
    pub commit_address: String,
    pub commit_amount_sats: u64,
    pub estimated_fee_sats: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastResultDto {
    pub action_id: String,
    pub proposal_status: String,
    pub broadcast_status: String,
    pub commit_txid: String,
    pub reveal_txid: String,
}

fn map_signature(signature: ProposalSignature) -> ProposalSignatureDto {
    ProposalSignatureDto {
        signer_pubkey: signature.signer_pubkey,
        signature_hex: signature.signature_hex,
    }
}

fn map_proposal(proposal: Proposal) -> ProposalDto {
    ProposalDto {
        action_id: proposal.action_id,
        seq_no: proposal.seq_no,
        authority: proposal.authority.as_str().to_string(),
        status: proposal.status,
        required_signatures: proposal.required_signatures,
        action_hex: proposal.action_hex,
        signatures: proposal.signatures.into_iter().map(map_signature).collect(),
        broadcast_status: proposal.broadcast_status,
        commit_txid: proposal.commit_txid,
        reveal_txid: proposal.reveal_txid,
        broadcast_error: proposal.broadcast_error,
    }
}

fn build_client(base_url: String) -> Result<HttpOrchestratorClient, String> {
    let session = orchestrator_auth::get_session()?
        .ok_or_else(|| "no orchestrator session; authenticate first".to_string())?;
    Ok(HttpOrchestratorClient::new(base_url).with_bearer_token(session.token))
}

fn map_proposal_error(error: ProposalError) -> String {
    match error {
        ProposalError::Orchestrator(OrchestratorError::Backend { status: 401, .. }) => {
            "orchestrator session unauthorized (401). Re-authenticate on this screen and retry. If the backend restarted, start a fresh auth challenge.".to_string()
        }
        other => other.to_string(),
    }
}

#[tauri::command]
pub async fn proposals_get_next_seq_no(input: GetNextSeqNoInput) -> Result<u64, String> {
    let client = build_client(input.base_url)?;
    client
        .get_next_seq_no()
        .await
        .map_err(|e: OrchestratorError| e.to_string())
}

#[tauri::command]
pub async fn proposals_create(input: CreateProposalInput) -> Result<ProposalDto, String> {
    let client = build_client(input.base_url)?;
    let signature = Signature {
        signer_pubkey: input.signer_pubkey,
        signature_hex: input.signature_hex,
    };
    let proposal = proposals::create_update_action(
        &client,
        input.action_hex.as_str(),
        input.seq_no,
        &signature,
    )
    .await
    .map_err(map_proposal_error)?;
    Ok(map_proposal(proposal))
}

#[tauri::command]
pub async fn proposals_list(input: ListProposalsInput) -> Result<Vec<ProposalDto>, String> {
    let client = build_client(input.base_url)?;
    let proposals = proposals::list_proposals(&client, input.status.as_deref())
        .await
        .map_err(map_proposal_error)?;
    Ok(proposals.into_iter().map(map_proposal).collect())
}

#[tauri::command]
pub async fn proposals_get(input: GetProposalInput) -> Result<ProposalDto, String> {
    let client = build_client(input.base_url)?;
    let proposal = proposals::get_update_action(&client, &input.action_id)
        .await
        .map_err(map_proposal_error)?;
    Ok(map_proposal(proposal))
}

#[tauri::command]
pub async fn proposals_approve(input: ApproveProposalInput) -> Result<ProposalDto, String> {
    let client = build_client(input.base_url)?;
    let signature = Signature {
        signer_pubkey: input.signer_pubkey,
        signature_hex: input.signature_hex,
    };
    let proposal = proposals::approve_action(&client, &input.action_id, &signature)
        .await
        .map_err(map_proposal_error)?;
    Ok(map_proposal(proposal))
}

#[tauri::command]
pub async fn proposals_prepare_broadcast(
    input: BroadcastInput,
) -> Result<PrepareBroadcastDto, String> {
    let client = build_client(input.base_url)?;
    let bundle = proposals::prepare_broadcast(&client, &input.action_id)
        .await
        .map_err(map_proposal_error)?;

    Ok(PrepareBroadcastDto {
        action_id: bundle.action_id,
        commit_address: bundle.commit_address,
        commit_amount_sats: bundle.commit_amount_sats,
        estimated_fee_sats: bundle.estimated_fee_sats,
    })
}

#[tauri::command]
pub async fn proposals_broadcast(input: BroadcastInput) -> Result<BroadcastResultDto, String> {
    let client = build_client(input.base_url)?;
    let result = proposals::execute_broadcast(&client, &input.action_id)
        .await
        .map_err(map_proposal_error)?;

    Ok(BroadcastResultDto {
        action_id: result.action_id,
        proposal_status: result.proposal_status,
        broadcast_status: result.broadcast_status,
        commit_txid: result.commit_txid,
        reveal_txid: result.reveal_txid,
    })
}
