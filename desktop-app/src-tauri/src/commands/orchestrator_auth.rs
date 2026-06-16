use desktop_app::application::orchestrator_auth;
use desktop_app::application::orchestrator_client::{
    CompleteOrchestratorAuthRequest, OrchestratorAuthChallenge, OrchestratorAuthSession,
};
use desktop_app::application::orchestrator_url::validate_orchestrator_base_url;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartOrchestratorAuthInput {
    pub base_url: String,
    pub authority: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteOrchestratorAuthInput {
    pub base_url: String,
    pub challenge_id: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
    pub signature_format: String,
}

#[tauri::command]
pub async fn orchestrator_auth_start(
    input: StartOrchestratorAuthInput,
) -> Result<OrchestratorAuthChallenge, String> {
    validate_orchestrator_base_url(&input.base_url)?;
    orchestrator_auth::start_challenge(&input.base_url, &input.authority).await
}

#[tauri::command]
pub async fn orchestrator_auth_complete(
    input: CompleteOrchestratorAuthInput,
) -> Result<OrchestratorAuthSession, String> {
    validate_orchestrator_base_url(&input.base_url)?;
    orchestrator_auth::complete_auth(
        &input.base_url,
        CompleteOrchestratorAuthRequest {
            challenge_id: input.challenge_id,
            signer_pubkey: input.signer_pubkey,
            signature_hex: input.signature_hex,
            signature_format: input.signature_format,
        },
    )
    .await
}

#[tauri::command]
pub fn orchestrator_auth_get_session() -> Result<Option<OrchestratorAuthSession>, String> {
    orchestrator_auth::get_session()
}

#[tauri::command]
pub async fn orchestrator_auth_logout(base_url: String) -> Result<(), String> {
    validate_orchestrator_base_url(&base_url)?;
    orchestrator_auth::logout(&base_url).await
}
