//! Application layer — business logic for backend communication and session management.
//!
//! Commands delegate here. Functions receive dependencies as parameters (backend_url,
//! session_token Mutex) to enable future testability without Tauri framework.
//! See ADR-002 for the evolution strategy.

pub(crate) mod orchestrator_client;
pub(crate) mod proposals;
pub(crate) mod types;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AuthChallenge {
    pub(crate) nonce: String,
    pub(crate) expires_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BackendSession {
    pub(crate) session_id: String,
    pub(crate) signer_pubkey: String,
    pub(crate) authority: String,
    pub(crate) expires_at: String,
}

/// What we return to React — session_id is kept in Rust, never forwarded.
#[derive(Debug, Serialize)]
pub(crate) struct SessionInfo {
    pub(crate) signer_pubkey: String,
    pub(crate) authority: String,
    pub(crate) expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CreateSessionPayload {
    pub(crate) ephemeral_pubkey: String,
    pub(crate) nonce: String,
    pub(crate) attestation_signature: String,
    pub(crate) signer_pubkey: String,
    pub(crate) authority: String,
}

// ─── Auth ────────────────────────────────────────────────────────────────────

/// Fetch a nonce challenge from the backend.
pub(crate) async fn fetch_challenge(
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

/// Exchange a signed attestation for a session. Stores the token in the Mutex.
pub(crate) async fn create_session(
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

    // Store token — this is the only place the bearer token ever lives
    *session_token.lock().unwrap() = Some(session.session_id);

    Ok(SessionInfo {
        signer_pubkey: session.signer_pubkey,
        authority: session.authority,
        expires_at: session.expires_at,
    })
}

/// Sign-out: revoke the session on the backend and clear the local token.
pub(crate) async fn delete_session(
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

// ─── Proposals ───────────────────────────────────────────────────────────────

/// Fetch proposals from the backend, injecting the Bearer token.
pub(crate) async fn fetch_proposals(
    backend_url: &str,
    session_token: &Mutex<Option<String>>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    let token = session_token
        .lock()
        .unwrap()
        .clone()
        .ok_or("Not authenticated")?;

    let client = reqwest::Client::new();
    let mut req = client
        .get(format!("{backend_url}/proposals"))
        .bearer_auth(token);

    if let Some(s) = status {
        req = req.query(&[("status", s)]);
    }

    let res = req.send().await.map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Request failed: {}", res.status()));
    }

    res.json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())
}
