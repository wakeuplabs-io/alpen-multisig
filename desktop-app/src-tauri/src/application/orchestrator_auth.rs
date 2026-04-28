use std::sync::{Mutex, OnceLock};

use crate::application::orchestrator_client::{
    CompleteOrchestratorAuthRequest, OrchestratorAuthChallenge, OrchestratorAuthSession,
    OrchestratorClient, StartOrchestratorAuthRequest,
};
use crate::infrastructure::orchestrator_client::HttpOrchestratorClient;

#[derive(Default)]
struct OrchestratorAuthState {
    session: Option<OrchestratorAuthSession>,
}

fn state() -> &'static Mutex<OrchestratorAuthState> {
    static STATE: OnceLock<Mutex<OrchestratorAuthState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(OrchestratorAuthState::default()))
}

pub async fn start_challenge(
    base_url: &str,
    authority: &str,
) -> Result<OrchestratorAuthChallenge, String> {
    let client = HttpOrchestratorClient::new(base_url.to_string());
    client
        .auth_challenge(StartOrchestratorAuthRequest {
            authority: authority.to_string(),
        })
        .await
        .map_err(|e| e.to_string())
}

pub async fn complete_auth(
    base_url: &str,
    request: CompleteOrchestratorAuthRequest,
) -> Result<OrchestratorAuthSession, String> {
    let client = HttpOrchestratorClient::new(base_url.to_string());
    let session = client
        .auth_verify(request)
        .await
        .map_err(|e| e.to_string())?;
    state()
        .lock()
        .map_err(|_| "orchestrator auth state lock poisoned".to_string())?
        .session = Some(session.clone());
    Ok(session)
}

pub fn get_session() -> Result<Option<OrchestratorAuthSession>, String> {
    Ok(state()
        .lock()
        .map_err(|_| "orchestrator auth state lock poisoned".to_string())?
        .session
        .clone())
}

pub async fn logout(base_url: &str) -> Result<(), String> {
    let token = {
        let lock = state()
            .lock()
            .map_err(|_| "orchestrator auth state lock poisoned".to_string())?;
        lock.session
            .as_ref()
            .map(|session| session.token.clone())
            .ok_or_else(|| "no orchestrator session".to_string())?
    };
    let client = HttpOrchestratorClient::new(base_url.to_string()).with_bearer_token(token);
    client.auth_logout().await.map_err(|e| e.to_string())?;
    state()
        .lock()
        .map_err(|_| "orchestrator auth state lock poisoned".to_string())?
        .session = None;
    Ok(())
}
