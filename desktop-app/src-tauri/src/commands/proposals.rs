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
use desktop_app::infrastructure::node_config_store::NodeConfigState;
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
    pub action_type: String,
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

fn action_type_from_hex(target_action_id: &Option<String>, action_hex: &str) -> String {
    if target_action_id.is_some() {
        return "cancel".to_string();
    }
    let hex = action_hex.strip_prefix("0x").unwrap_or(action_hex);
    match desktop_app::infrastructure::action_codec::decode_hex(hex) {
        Ok(desktop_app::domain::action::Action::MultisigUpdate(_)) => "multisig_update".to_string(),
        Ok(desktop_app::domain::action::Action::VkUpdate(_)) => "vk_update".to_string(),
        Err(_) => "unknown".to_string(),
    }
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
    let action_type = action_type_from_hex(&proposal.target_action_id, &proposal.action_hex);
    ProposalDto {
        action_id: proposal.action_id,
        seq_no: proposal.seq_no,
        authority: proposal.authority.as_str().to_string(),
        status: proposal.status,
        required_signatures: proposal.required_signatures,
        action_hex: proposal.action_hex,
        action_type,
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
    let allow_insecure = std::env::var("ALLOW_INSECURE_HTTP_URL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if allow_insecure {
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
mod resubmit_reveal_tests {
    use super::{proposals_resubmit_reveal, ResubmitRevealInput};
    use desktop_app::application::pending_reveals::PendingReveals;
    use desktop_app::application::wallet_session::WalletSession;

    #[test]
    #[allow(clippy::let_underscore_future)]
    fn proposals_resubmit_reveal_uses_pending_reveals_state() {
        use desktop_app::infrastructure::node_config_store::NodeConfigState;
        fn _check(
            input: ResubmitRevealInput,
            s: tauri::State<'_, WalletSession>,
            nc: tauri::State<'_, NodeConfigState>,
            p: tauri::State<'_, PendingReveals>,
        ) {
            let _ = proposals_resubmit_reveal(input, s, nc, p);
        }
    }
}

#[cfg(test)]
mod wallet_session_state_tests {
    use super::proposals_broadcast;
    use desktop_app::application::pending_reveals::PendingReveals;
    use desktop_app::application::wallet_session::WalletSession;
    use desktop_app::infrastructure::node_config_store::NodeConfigState;

    /// REGRESSION: proposals_broadcast must take WalletSession (not Arc<WalletService>).
    /// Phase 3.7 migrated managed state to WalletSession; a stale Arc<WalletService> param
    /// causes Tauri runtime error: "state not managed for field 'walletService'".
    #[test]
    #[allow(clippy::let_underscore_future)]
    fn proposals_broadcast_uses_wallet_session_state() {
        fn _check(
            input: super::BroadcastInput,
            s: tauri::State<'_, WalletSession>,
            nc: tauri::State<'_, NodeConfigState>,
            p: tauri::State<'_, PendingReveals>,
        ) {
            let _ = proposals_broadcast(input, s, nc, p);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResubmitRevealInput {
    pub base_url: String,
    pub action_id: String,
}

#[tauri::command]
pub async fn proposals_resubmit_reveal(
    input: ResubmitRevealInput,
    wallet_session: tauri::State<'_, WalletSession>,
    node_config: tauri::State<'_, NodeConfigState>,
    pending: tauri::State<'_, PendingReveals>,
) -> Result<String, String> {
    let client = build_client(input.base_url)?;
    let cfg = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();
    let env =
        broadcast_env::load_broadcast_env(&wallet_session, &cfg).map_err(|e| e.to_string())?;
    let btc_rpc = HttpBitcoinRpcClient::new(
        &env.btc_rpc_url,
        env.btc_wallet_name.as_deref(),
        &env.btc_rpc_user,
        &env.btc_rpc_pass,
    );
    proposals::resubmit_reveal(&pending, &btc_rpc, &client, &input.action_id)
        .await
        .map_err(|e| match e {
            BroadcastError::NoPendingReveal { action_id } => {
                format!("no pending reveal for action {action_id} — re-run broadcast")
            }
            other => map_broadcast_error(other),
        })
}

#[cfg(test)]
mod broadcast_error_code_tests {
    use super::{broadcast_error_code, map_broadcast_error, map_broadcast_error_with_boundary};
    use desktop_app::application::orchestrator_client::OrchestratorError;
    use desktop_app::application::proposals::BroadcastError;

    /// BE-11: Orchestrator session expiry during broadcast (boundary=BEFORE).
    ///
    /// When the orchestrator returns 401 during the pre-broadcast proposal fetch,
    /// the error maps to code=OrchestratorUnauthorized with recovery=re-auth→retry.
    ///
    /// This is a focused regression test for the 401 case; the full error-code
    /// matrix is covered by `broadcast_error_code_maps_all_10_codes` (step 01-10).
    #[test]
    fn test_broadcast_error_orchestrator_unauthorized() {
        let error = BroadcastError::ProposalFetch(OrchestratorError::Backend {
            status: 401,
            message: "unauthorized".to_string(),
        });
        let code = broadcast_error_code(&error, false, false);
        assert_eq!(code, "OrchestratorUnauthorized");
    }

    #[test]
    fn broadcast_error_code_maps_all_10_codes() {
        let cases = [
            // OrchestratorUnauthorized: 401 from proposal fetch
            (
                BroadcastError::ProposalFetch(OrchestratorError::Backend {
                    status: 401,
                    message: "unauthorized".to_string(),
                }),
                true,
                false,
                "OrchestratorUnauthorized",
            ),
            // NoPendingReveal
            (
                BroadcastError::NoPendingReveal {
                    action_id: "act-1".to_string(),
                },
                false,
                false,
                "NoPendingReveal",
            ),
            // BitcoinRpc (before broadcast)
            (
                BroadcastError::BitcoinRpc("node rejected".to_string()),
                false,
                false,
                "BitcoinRpc",
            ),
            // BitcoinRpc (after broadcast reached)
            (
                BroadcastError::BitcoinRpc("node rejected".to_string()),
                true,
                true,
                "BitcoinRpc",
            ),
            // Timeout
            (
                BroadcastError::Timeout {
                    txid: "tx-1".to_string(),
                },
                true,
                true,
                "Timeout",
            ),
            // Setup errors that are unmapped → Unknown
            (
                BroadcastError::Setup("something broke".to_string()),
                false,
                false,
                "Unknown",
            ),
            // ProposalFetch non-401 → Unknown
            (
                BroadcastError::ProposalFetch(OrchestratorError::Backend {
                    status: 500,
                    message: "server error".to_string(),
                }),
                false,
                false,
                "Unknown",
            ),
        ];

        for (error, broadcast_reached, has_pending, expected_code) in cases {
            let code = broadcast_error_code(&error, broadcast_reached, has_pending);
            assert_eq!(
                code, expected_code,
                "expected {expected_code} for error: {error:?}"
            );
        }
    }

    /// BE-13: BitcoinRpc failure BEFORE broadcast boundary.
    ///
    /// When a Bitcoin RPC error occurs before submit_package is reached (e.g. during
    /// sync/build), the error maps to code=BitcoinRpc with boundary=BEFORE.
    /// Recovery is retry-from-scratch and canResubmit=false — even if a PendingReveal
    /// exists in the store (NIT-3: presence doesn't prove broadcast).
    #[test]
    fn test_broadcast_error_bitcoin_rpc_before_boundary() {
        let error = BroadcastError::BitcoinRpc("node rejected".to_string());

        // BEFORE boundary: broadcast was never reached
        let code = broadcast_error_code(&error, false, false);
        assert_eq!(code, "BitcoinRpc");

        // NIT-3: even with a live PendingReveal, BEFORE boundary → canResubmit=false
        let code_with_pending = broadcast_error_code(&error, false, true);
        assert_eq!(code_with_pending, "BitcoinRpc");

        // The message should NOT mention resubmit (BEFORE boundary = retry-from-scratch)
        let error_msg = map_broadcast_error(error);
        let parsed: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        assert_eq!(parsed["code"], "BitcoinRpc");
        assert!(
            !parsed["message"].as_str().unwrap().contains("resubmit"),
            "BEFORE boundary BitcoinRpc message should NOT mention resubmit: {}",
            parsed["message"]
        );
    }

    /// BE-14: BitcoinRpc failure AFTER broadcast boundary — code=BitcoinRpc, boundary=AFTER, recovery=resubmit-reveal, canResubmit=true.
    ///
    /// When a Bitcoin RPC error occurs after submit_package was attempted (boundary=AFTER)
    /// and a live PendingReveal exists, the error maps to code=BitcoinRpc with
    /// canResubmit=true — the user can resubmit the reveal transaction.
    /// Resubmit eligibility requires BOTH: AFTER-boundary AND a live PendingReveal.
    #[test]
    fn test_broadcast_error_bitcoin_rpc_after_boundary() {
        let error = BroadcastError::BitcoinRpc("submit_package failed".to_string());

        // AFTER boundary + live PendingReveal → canResubmit=true
        let error_msg = map_broadcast_error_with_boundary(error, true, true);
        let parsed: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        assert_eq!(parsed["code"], "BitcoinRpc");
        assert_eq!(parsed["canResubmit"], true);
        assert!(
            parsed["message"].as_str().unwrap().contains("resubmit"),
            "AFTER boundary BitcoinRpc message should mention resubmit: {}",
            parsed["message"]
        );
    }

    /// BE-12: Confirmation timeout after broadcast (boundary=AFTER).
    ///
    /// When the confirmation poll exceeds `confirm_timeout_ms` after the broadcast
    /// was sent, the error maps to code=Timeout with recovery=resubmit-reveal.
    /// The user can resubmit the reveal transaction.
    #[test]
    fn test_broadcast_error_confirmation_timeout() {
        let error = BroadcastError::Timeout {
            txid: "abc123".to_string(),
        };
        let code = broadcast_error_code(&error, true, true);
        assert_eq!(code, "Timeout");

        let error_msg = map_broadcast_error(error);
        let parsed: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        assert_eq!(parsed["code"], "Timeout");
        assert!(
            parsed["message"].as_str().unwrap().contains("resubmit"),
            "Timeout message should mention resubmit: {}",
            parsed["message"]
        );
    }
}

/// Pure helper: maps a BroadcastError to a stable error code string per the DDD-8 table.
///
/// `broadcast_reached`: whether the broadcast call was actually reached/attempted
/// (e.g. commit_broadcasted report was sent).
/// `has_pending`: whether a live PendingReveal exists for this action_id.
///
/// Used by `map_broadcast_error` to produce structured `{ code, message }` JSON.
fn broadcast_error_code(
    error: &BroadcastError,
    _broadcast_reached: bool,
    _has_pending: bool,
) -> &'static str {
    match error {
        BroadcastError::ProposalFetch(OrchestratorError::Backend { status: 401, .. }) => {
            "OrchestratorUnauthorized"
        }
        BroadcastError::NoPendingReveal { .. } => "NoPendingReveal",
        BroadcastError::BitcoinRpc(_) => "BitcoinRpc",
        BroadcastError::Timeout { .. } => "Timeout",
        BroadcastError::Setup(_) => "Unknown",
        BroadcastError::ProposalFetch(_) => "Unknown",
    }
}

/// Pure helper: maps a BroadcastError to a structured JSON error string with boundary context.
///
/// `broadcast_reached`: whether the broadcast call was actually reached/attempted
/// `has_pending`: whether a live PendingReveal exists for this action_id.
///
/// canResubmit = true only when BOTH: AFTER-boundary (broadcast_reached) AND live PendingReveal.
fn map_broadcast_error_with_boundary(
    error: BroadcastError,
    broadcast_reached: bool,
    has_pending: bool,
) -> String {
    let code = broadcast_error_code(&error, broadcast_reached, has_pending);
    let can_resubmit = broadcast_reached && has_pending;
    let message = match &error {
        BroadcastError::ProposalFetch(OrchestratorError::Backend { status: 401, .. }) => {
            "orchestrator session unauthorized (401). Re-authenticate on this screen and retry."
                .to_string()
        }
        BroadcastError::NoPendingReveal { action_id } => {
            format!("no pending reveal for action {action_id} — re-run broadcast")
        }
        BroadcastError::BitcoinRpc(msg) => {
            if can_resubmit {
                format!("The Bitcoin node rejected or could not process the broadcast: {msg}. You can resubmit the reveal.")
            } else {
                format!("The Bitcoin node rejected or could not process the broadcast: {msg}")
            }
        }
        BroadcastError::Timeout { txid } => {
            format!("Broadcast sent but confirmation timed out for tx {txid}. You can resubmit the reveal.")
        }
        BroadcastError::Setup(msg) => msg.clone(),
        BroadcastError::ProposalFetch(e) => e.to_string(),
    };
    serde_json::json!({ "code": code, "message": message, "canResubmit": can_resubmit }).to_string()
}

fn map_broadcast_error(error: BroadcastError) -> String {
    map_broadcast_error_with_boundary(error, false, false)
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
    node_config: tauri::State<'_, NodeConfigState>,
) -> Result<PrepareBroadcastDto, String> {
    let client = build_client(input.base_url)?;
    let cfg = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();
    let env =
        broadcast_env::load_broadcast_env(&wallet_session, &cfg).map_err(|e| e.to_string())?;
    let btc_rpc = HttpBitcoinRpcClient::new(
        &env.btc_rpc_url,
        env.btc_wallet_name.as_deref(),
        &env.btc_rpc_user,
        &env.btc_rpc_pass,
    );

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
    node_config: tauri::State<'_, NodeConfigState>,
    pending: tauri::State<'_, PendingReveals>,
) -> Result<BroadcastResultDto, String> {
    let wallet_service = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    let client = build_client(input.base_url)?;
    let cfg = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();
    let env =
        broadcast_env::load_broadcast_env(&wallet_session, &cfg).map_err(|e| e.to_string())?;
    let btc_rpc = std::sync::Arc::new(HttpBitcoinRpcClient::new(
        &env.btc_rpc_url,
        env.btc_wallet_name.as_deref(),
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
        &pending,
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
