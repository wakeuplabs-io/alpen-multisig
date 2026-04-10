//! Thin Tauri command wrappers. Each command extracts State, delegates to
//! the application layer, and maps errors to String for the IPC boundary.

use crate::state::AppState;
use desktop_app::application::{self, AuthChallenge, CreateSessionPayload, SessionInfo};
use tauri::State;

#[tauri::command]
pub(crate) async fn get_challenge(
    state: State<'_, AppState>,
    pubkey: String,
    authority: String,
) -> Result<AuthChallenge, String> {
    application::fetch_challenge(&state.backend_url, &pubkey, &authority).await
}

#[tauri::command]
pub(crate) async fn create_session(
    state: State<'_, AppState>,
    payload: CreateSessionPayload,
) -> Result<SessionInfo, String> {
    application::create_session(&state.backend_url, &state.session_token, payload).await
}

#[tauri::command]
pub(crate) async fn delete_session(state: State<'_, AppState>) -> Result<(), String> {
    application::delete_session(&state.backend_url, &state.session_token).await
}

#[tauri::command]
pub(crate) async fn list_proposals(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    application::fetch_proposals(&state.backend_url, &state.session_token, status).await
}
