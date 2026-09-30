use std::sync::{Mutex, OnceLock};

use crate::application::orchestrator_client::{
    CompleteOrchestratorAuthRequest, OrchestratorAuthChallenge, OrchestratorAuthSession,
    OrchestratorClient, StartOrchestratorAuthRequest,
};
use crate::infrastructure::orchestrator_client::HttpOrchestratorClient;

#[derive(Default)]
struct OrchestratorAuthState {
    session: Option<OrchestratorAuthSession>,
    /// The orchestrator the session was obtained from.
    base_url: Option<String>,
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
    let mut lock = state()
        .lock()
        .map_err(|_| "orchestrator auth state lock poisoned".to_string())?;
    lock.session = Some(session.clone());
    lock.base_url = Some(base_url.to_string());
    Ok(session)
}

/// A client for the orchestrator of the current, unexpired session — for background work that
/// has no request to take a base URL from (the settle loop, #516). `None` without a session.
pub fn authenticated_client() -> Option<HttpOrchestratorClient> {
    let session = get_session().ok()??;
    let base_url = state().lock().ok()?.base_url.clone()?;
    Some(HttpOrchestratorClient::new(base_url).with_bearer_token(session.token))
}

pub fn get_session() -> Result<Option<OrchestratorAuthSession>, String> {
    let mut lock = state()
        .lock()
        .map_err(|_| "orchestrator auth state lock poisoned".to_string())?;

    let expired = lock
        .session
        .as_ref()
        .map(|s| now_unix_ms() >= s.expires_at_unix_ms)
        .unwrap_or(false);

    if expired {
        lock.session = None;
    }

    Ok(lock.session.clone())
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
