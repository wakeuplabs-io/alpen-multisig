//! Auth business logic — challenge fetch and session lifecycle over HTTP.

use crate::domain::session::{AuthChallenge, BackendSession, CreateSessionPayload, SessionInfo};
use std::sync::Mutex;

pub async fn fetch_challenge(
    backend_url: &str,
    pubkey: &str,
    authority: &str,
) -> Result<AuthChallenge, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{backend_url}/auth/challenge"))
        .query(&[("signer_pubkey", pubkey), ("authority", authority)])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Challenge request failed: {}", res.status()));
    }

    res.json::<AuthChallenge>().await.map_err(|e| e.to_string())
}

pub async fn create_session(
    backend_url: &str,
    session_token: &Mutex<Option<String>>,
    payload: CreateSessionPayload,
) -> Result<SessionInfo, String> {
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{backend_url}/auth/session"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Session creation failed: {}", res.status()));
    }

    let session = res
        .json::<BackendSession>()
        .await
        .map_err(|e| e.to_string())?;

    *session_token.lock().unwrap() = Some(session.session_id);

    Ok(SessionInfo {
        signer_pubkey: session.signer_pubkey,
        authority: session.authority,
        expires_at: session.expires_at,
    })
}

pub async fn delete_session(
    backend_url: &str,
    session_token: &Mutex<Option<String>>,
) -> Result<(), String> {
    let token = session_token.lock().unwrap().take();

    if let Some(token) = token {
        let client = reqwest::Client::new();
        client
            .delete(format!("{backend_url}/auth/session"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
