#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::State;

// ─── App State ────────────────────────────────────────────────────────────────
//
// Session token lives here — never exposed to the WebView.
// All authenticated requests read it from this state and inject it as a Bearer header.

struct AppState {
	session_token: Mutex<Option<String>>,
	backend_url: String,
}

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct AuthChallenge {
	nonce: String,
	expires_at: String,
}

#[derive(Debug, Deserialize)]
struct BackendSession {
	session_id: String,
	signer_pubkey: String,
	authority: String,
	expires_at: String,
}

/// What we return to React — session_id is kept in Rust, never forwarded.
#[derive(Debug, Serialize)]
struct SessionInfo {
	signer_pubkey: String,
	authority: String,
	expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateSessionPayload {
	ephemeral_pubkey: String,
	nonce: String,
	attestation_signature: String,
	signer_pubkey: String,
	authority: String,
}

// ─── Auth Commands ────────────────────────────────────────────────────────────

/// Step 1 of auth: fetch a nonce challenge from the backend.
/// The nonce is returned to React so the hardware wallet (Trezor/Ledger JS SDK)
/// can sign it before calling create_session.
#[tauri::command]
fn greet(name: String) -> String {
	format!("Hello, {}! Greetings from Rust.", name)
}

#[tauri::command]
async fn get_challenge(
	state: State<'_, AppState>,
	pubkey: String,
	authority: String,
) -> Result<AuthChallenge, String> {
	let client = reqwest::Client::new();

	let res = client
		.get(format!("{}/auth/challenge", state.backend_url))
		.query(&[("signer_pubkey", &pubkey), ("authority", &authority)])
		.send()
		.await
		.map_err(|e| e.to_string())?;

	if !res.status().is_success() {
		return Err(format!("Challenge request failed: {}", res.status()));
	}

	res.json::<AuthChallenge>().await.map_err(|e| e.to_string())
}

/// Step 2 of auth: exchange the signed nonce for a session.
/// The session_id (bearer token) is stored in AppState — React never sees it.
/// React receives only the session metadata needed to render the UI.
#[tauri::command]
async fn create_session(
	state: State<'_, AppState>,
	payload: CreateSessionPayload,
) -> Result<SessionInfo, String> {
	let client = reqwest::Client::new();

	let res = client
		.post(format!("{}/auth/session", state.backend_url))
		.json(&payload)
		.send()
		.await
		.map_err(|e| e.to_string())?;

	if !res.status().is_success() {
		return Err(format!("Session creation failed: {}", res.status()));
	}

	let session = res.json::<BackendSession>().await.map_err(|e| e.to_string())?;

	// Store token — this is the only place the bearer token ever lives
	*state.session_token.lock().unwrap() = Some(session.session_id);

	Ok(SessionInfo {
		signer_pubkey: session.signer_pubkey,
		authority: session.authority,
		expires_at: session.expires_at,
	})
}

/// Sign-out: revoke the session on the backend and clear the local token.
#[tauri::command]
async fn delete_session(state: State<'_, AppState>) -> Result<(), String> {
	let token = state.session_token.lock().unwrap().take();

	if let Some(token) = token {
		let client = reqwest::Client::new();
		client
			.delete(format!("{}/auth/session", state.backend_url))
			.bearer_auth(token)
			.send()
			.await
			.map_err(|e| e.to_string())?;
	}

	Ok(())
}

// ─── Proposal Commands (example) ──────────────────────────────────────────────
//
// This shows the pattern for authenticated commands.
// The token is read from AppState and injected — React passes no token at all.
// Remaining commands (get_proposal, create_proposal, submit_signature,
// list_signatures) follow this exact same pattern.

#[tauri::command]
async fn list_proposals(
	state: State<'_, AppState>,
	status: Option<String>,
) -> Result<serde_json::Value, String> {
	let token = state
		.session_token
		.lock()
		.unwrap()
		.clone()
		.ok_or("Not authenticated")?;

	let client = reqwest::Client::new();
	let mut req = client
		.get(format!("{}/proposals", state.backend_url))
		.bearer_auth(token);

	if let Some(s) = status {
		req = req.query(&[("status", s)]);
	}

	let res = req.send().await.map_err(|e| e.to_string())?;

	if !res.status().is_success() {
		return Err(format!("Request failed: {}", res.status()));
	}

	res.json::<serde_json::Value>().await.map_err(|e| e.to_string())
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
	let backend_url = std::env::var("BACKEND_URL")
		.unwrap_or_else(|_| "http://127.0.0.1:3000/api/v1".to_string());

	tauri::Builder::default()
		.manage(AppState {
			session_token: Mutex::new(None),
			backend_url,
		})
		.invoke_handler(tauri::generate_handler![
			greet,
			get_challenge,
			create_session,
			delete_session,
			list_proposals,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application")
}
