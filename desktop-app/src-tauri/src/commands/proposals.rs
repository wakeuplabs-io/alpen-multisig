use desktop_app::application::commit_funding::BdkAdminWalletMnemonic;
use desktop_app::application::orchestrator_auth;
use desktop_app::application::orchestrator_client::{
    CreateCancelProposalRequest, OrchestratorClient, OrchestratorError,
};
use desktop_app::application::pending_reveals::PendingReveals;
use desktop_app::application::proposals;
use desktop_app::application::proposals::{BroadcastError, ProposalError};
use desktop_app::application::wallet_session::WalletSession;
use desktop_app::domain::proposal::{
    CancelProposalSummary, Proposal, ProposalSignature, Signature,
};
use desktop_app::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient;
use desktop_app::infrastructure::broadcast_env;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCancelProposalInput {
    pub base_url: String,
    pub target_action_id: String,
    pub seq_no: u64,
    pub action_hex: String,
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
pub struct CancelProposalSummaryDto {
    pub action_id: String,
    pub status: String,
    pub signatures: Vec<ProposalSignatureDto>,
    pub required_signatures: u16,
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
    pub target_action_id: Option<String>,
    pub activation_height: Option<u64>,
    pub update_id_in_queue: Option<u32>,
    pub cancel_proposal: Option<CancelProposalSummaryDto>,
}

#[tauri::command]
pub async fn proposals_create_cancel(
    input: CreateCancelProposalInput,
) -> Result<ProposalDto, String> {
    let client = build_client(input.base_url)?;
    let proposal = client
        .create_cancel_proposal(
            &input.target_action_id,
            CreateCancelProposalRequest {
                seq_no: input.seq_no,
                action_hex: input.action_hex,
                signer_pubkey: input.signer_pubkey,
                signature_hex: input.signature_hex,
            },
        )
        .await
        .map_err(|e| match e {
            OrchestratorError::Backend { status: 401, .. } => {
                "orchestrator session unauthorized (401). Re-authenticate and retry.".to_string()
            }
            other => other.to_string(),
        })?;
    Ok(map_proposal(proposal))
}

/// IPC payload for broadcast commands — Bitcoin RPC and operator key load from Tauri env (P-066).
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

fn map_cancel_summary(summary: CancelProposalSummary) -> CancelProposalSummaryDto {
    CancelProposalSummaryDto {
        action_id: summary.action_id,
        status: summary.status,
        signatures: summary.signatures.into_iter().map(map_signature).collect(),
        required_signatures: summary.required_signatures,
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
        target_action_id: proposal.target_action_id,
        activation_height: proposal.activation_height,
        update_id_in_queue: proposal.update_id_in_queue,
        cancel_proposal: proposal.cancel_proposal.map(map_cancel_summary),
    }
}

fn validate_orchestrator_base_url(base_url: &str) -> Result<(), String> {
    let trimmed = base_url.trim();
    if trimmed.starts_with("https://") {
        return Ok(());
    }
    if trimmed.starts_with("http://localhost")
        || trimmed.starts_with("http://127.0.0.1")
        || trimmed.starts_with("http://[::1]")
    {
        return Ok(());
    }
    Err(
        "orchestrator base_url must use https:// (http://localhost or 127.0.0.1 allowed for local dev only)"
            .to_string(),
    )
}

fn build_client(base_url: String) -> Result<HttpOrchestratorClient, String> {
    validate_orchestrator_base_url(&base_url)?;
    let session = orchestrator_auth::get_session()?
        .ok_or_else(|| "no orchestrator session; authenticate first".to_string())?;
    Ok(HttpOrchestratorClient::new(base_url).with_bearer_token(session.token))
}

#[cfg(test)]
mod url_tests {
    use super::validate_orchestrator_base_url;

    #[test]
    fn rejects_plain_http_remote() {
        assert!(validate_orchestrator_base_url("http://evil.example/api/v1").is_err());
    }

    #[test]
    fn allows_https() {
        assert!(validate_orchestrator_base_url("https://orchestrator.example/api/v1").is_ok());
    }

    #[test]
    fn allows_localhost_http() {
        assert!(validate_orchestrator_base_url("http://127.0.0.1:3000/api/v1").is_ok());
    }
}

#[cfg(test)]
mod wallet_session_state_tests {
    use super::proposals_broadcast;
    use desktop_app::application::wallet_session::WalletSession;

    /// REGRESSION: proposals_broadcast must take WalletSession (not Arc<WalletService>).
    /// Phase 3.7 migrated managed state to WalletSession; a stale Arc<WalletService> param
    /// causes Tauri runtime error: "state not managed for field 'walletService'".
    #[test]
    #[allow(clippy::let_underscore_future)]
    fn proposals_broadcast_uses_wallet_session_state() {
        use desktop_app::application::pending_reveals::PendingReveals;
        fn _check(
            input: super::BroadcastInput,
            s: tauri::State<'_, WalletSession>,
            p: tauri::State<'_, PendingReveals>,
        ) {
            let _ = proposals_broadcast(input, s, p);
        }
    }
}

fn map_broadcast_error(error: BroadcastError) -> String {
    match error {
        BroadcastError::ProposalFetch(OrchestratorError::Backend { status: 401, .. }) => {
            "orchestrator session unauthorized (401). Re-authenticate on this screen and retry."
                .to_string()
        }
        other => other.to_string(),
    }
}

fn map_proposal_error(error: ProposalError) -> String {
    match error {
        ProposalError::Orchestrator(OrchestratorError::Backend { status: 401, .. }) => {
            "orchestrator session unauthorized (401). Re-authenticate on this screen and retry. If the backend restarted, start a fresh auth challenge.".to_string()
        }
        other => other.to_string(),
    }
}

fn serialize_wallet_error<E: serde::Serialize + std::fmt::Debug>(e: E) -> String {
    serde_json::to_string(&e).unwrap_or_else(|_| format!("{:?}", e))
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
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<PrepareBroadcastDto, String> {
    let client = build_client(input.base_url)?;
    let env = broadcast_env::load_broadcast_env(&wallet_session).map_err(|e| e.to_string())?;
    let btc_rpc = HttpBitcoinRpcClient::new(&env.btc_rpc_url, &env.btc_rpc_user, &env.btc_rpc_pass);

    let (commit_address, commit_amount_sats, estimated_fee_sats) =
        proposals::prepare_broadcast_local(
            &client,
            &btc_rpc,
            &env.asm_rpc_url,
            env.network,
            &input.action_id,
        )
        .await
        .map_err(map_broadcast_error)?;

    Ok(PrepareBroadcastDto {
        action_id: input.action_id,
        commit_address,
        commit_amount_sats,
        estimated_fee_sats,
    })
}

#[tauri::command]
pub async fn proposals_broadcast(
    input: BroadcastInput,
    wallet_session: tauri::State<'_, WalletSession>,
    pending_reveals: tauri::State<'_, PendingReveals>,
) -> Result<BroadcastResultDto, String> {
    let wallet_service = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    let client = build_client(input.base_url)?;
    let env = broadcast_env::load_broadcast_env(&wallet_session).map_err(|e| e.to_string())?;
    let btc_rpc = std::sync::Arc::new(HttpBitcoinRpcClient::new(
        &env.btc_rpc_url,
        &env.btc_rpc_user,
        &env.btc_rpc_pass,
    ));
    let commit_funding = BdkAdminWalletMnemonic::new(std::sync::Arc::clone(&wallet_service));
    let reveal_change_address = wallet_service
        .reveal_change_address()
        .await
        .map_err(|e| e.to_string())?;
    let reveal_change_spk = reveal_change_address.script_pubkey();

    let (commit_txid, reveal_txid) = proposals::broadcast_commit_then_reveal(
        &client,
        btc_rpc.as_ref(),
        &env.asm_rpc_url,
        env.magic_bytes,
        env.network,
        &input.action_id,
        env.confirm_poll_interval_ms,
        env.confirm_timeout_ms,
        &commit_funding,
        reveal_change_spk,
        &pending_reveals,
    )
    .await
    .map_err(map_broadcast_error)?;

    let proposal = client
        .get_proposal(&input.action_id)
        .await
        .map_err(|e| map_broadcast_error(BroadcastError::ProposalFetch(e)))?;

    Ok(BroadcastResultDto {
        action_id: proposal.action_id,
        proposal_status: proposal.status,
        broadcast_status: proposal.broadcast_status,
        commit_txid: proposal.commit_txid.unwrap_or(commit_txid),
        reveal_txid: proposal.reveal_txid.unwrap_or(reveal_txid),
    })
}
