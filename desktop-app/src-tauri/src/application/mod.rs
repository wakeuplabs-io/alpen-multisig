//! Application layer — business logic for backend communication.
//!
//! `proposals` is the public entry point for proposal operations.
//! `orchestrator_client` defines the HTTP client trait and implementation.

pub mod orchestrator_client;
pub mod proposals;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// ─── Auth types (placeholder — no auth in POC) ─────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub nonce: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct BackendSession {
    pub session_id: String,
    pub signer_pubkey: String,
    pub authority: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub signer_pubkey: String,
    pub authority: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionPayload {
    pub ephemeral_pubkey: String,
    pub nonce: String,
    pub attestation_signature: String,
    pub signer_pubkey: String,
    pub authority: String,
}

// ─── Auth functions (placeholder — no auth in POC) ─────────────────────────

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

pub async fn fetch_proposals(
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
