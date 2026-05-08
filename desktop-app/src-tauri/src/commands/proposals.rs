use desktop_app::application::orchestrator_auth;
use desktop_app::application::orchestrator_client::OrchestratorError;
use desktop_app::application::proposals;
use desktop_app::application::proposals::{BroadcastError, ProposalError};
use desktop_app::domain::proposal::{Proposal, ProposalSignature, Signature};
use desktop_app::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastInput {
    pub base_url: String,
    pub action_id: String,
    pub btc_rpc_url: String,
    pub btc_rpc_user: String,
    pub btc_rpc_pass: String,
    pub btc_wallet_name: Option<String>,
    pub operator_secret_key_hex: String,
    pub magic_bytes_hex: String,
    pub asm_rpc_url: String,
    /// Bitcoin network name: "bitcoin", "testnet", "signet", or "regtest" (default).
    pub network: Option<String>,
    pub confirm_poll_interval_ms: Option<u64>,
    pub confirm_timeout_ms: Option<u64>,
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

fn map_broadcast_error(error: BroadcastError) -> String {
    match error {
        BroadcastError::ProposalFetch(OrchestratorError::Backend { status: 401, .. }) => {
            "orchestrator session unauthorized (401). Re-authenticate on this screen and retry."
                .to_string()
        }
        other => other.to_string(),
    }
}

fn parse_network(network: Option<&str>) -> Result<bitcoin::Network, String> {
    match network.unwrap_or("regtest") {
        "bitcoin" => Ok(bitcoin::Network::Bitcoin),
        "testnet" => Ok(bitcoin::Network::Testnet),
        "signet" => Ok(bitcoin::Network::Signet),
        "regtest" => Ok(bitcoin::Network::Regtest),
        other => Err(format!(
            "unknown network '{other}'; expected bitcoin/testnet/signet/regtest"
        )),
    }
}

fn parse_operator_keypair(secret_key_hex: &str) -> Result<bitcoin::key::UntweakedKeypair, String> {
    let sk_bytes =
        hex::decode(secret_key_hex).map_err(|e| format!("invalid operator key hex: {e}"))?;
    let sk = bitcoin::secp256k1::SecretKey::from_slice(&sk_bytes)
        .map_err(|e| format!("invalid operator secret key: {e}"))?;
    Ok(bitcoin::key::UntweakedKeypair::from_secret_key(
        bitcoin::secp256k1::SECP256K1,
        &sk,
    ))
}

fn parse_magic_bytes(hex_str: &str) -> Result<strata_l1_txfmt::MagicBytes, String> {
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid magic bytes hex: {e}"))?;
    let arr: [u8; 4] = bytes
        .try_into()
        .map_err(|_| "magic bytes must be exactly 4 bytes".to_string())?;
    Ok(strata_l1_txfmt::MagicBytes::new(arr))
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
    let btc_rpc = HttpBitcoinRpcClient::new(
        &input.btc_rpc_url,
        input.btc_wallet_name.as_deref(),
        &input.btc_rpc_user,
        &input.btc_rpc_pass,
    );
    let keypair = parse_operator_keypair(&input.operator_secret_key_hex)?;
    let network = parse_network(input.network.as_deref())?;

    let (commit_address, commit_amount_sats, estimated_fee_sats) =
        proposals::prepare_broadcast_bundle(
            &client,
            &btc_rpc,
            &input.asm_rpc_url,
            &keypair,
            network,
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
pub async fn proposals_broadcast(input: BroadcastInput) -> Result<BroadcastResultDto, String> {
    let client = build_client(input.base_url)?;
    let btc_rpc = HttpBitcoinRpcClient::new(
        &input.btc_rpc_url,
        input.btc_wallet_name.as_deref(),
        &input.btc_rpc_user,
        &input.btc_rpc_pass,
    );
    let keypair = parse_operator_keypair(&input.operator_secret_key_hex)?;
    let magic_bytes = parse_magic_bytes(&input.magic_bytes_hex)?;
    let network = parse_network(input.network.as_deref())?;
    let confirm_poll_interval_ms = input.confirm_poll_interval_ms.unwrap_or(5_000);
    let confirm_timeout_ms = input.confirm_timeout_ms.unwrap_or(600_000);

    let (commit_txid, reveal_txid) = proposals::broadcast_commit_then_reveal(
        &client,
        &btc_rpc,
        &input.asm_rpc_url,
        &keypair,
        magic_bytes,
        network,
        &input.action_id,
        confirm_poll_interval_ms,
        confirm_timeout_ms,
    )
    .await
    .map_err(map_broadcast_error)?;

    Ok(BroadcastResultDto {
        action_id: input.action_id,
        proposal_status: "enacted".to_string(),
        broadcast_status: "reveal_confirmed".to_string(),
        commit_txid,
        reveal_txid,
    })
}
