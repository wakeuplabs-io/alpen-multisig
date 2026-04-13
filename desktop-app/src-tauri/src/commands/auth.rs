//! Auth Tauri commands.

use crate::state::AppState;
use desktop_app::application::auth;
use desktop_app::domain::session::{AuthChallenge, CreateSessionPayload, SessionInfo};
use tauri::State;

#[tauri::command]
pub(crate) async fn get_challenge(
    state: State<'_, AppState>,
    pubkey: String,
    authority: String,
) -> Result<AuthChallenge, String> {
    auth::fetch_challenge(&state.backend_url, &pubkey, &authority).await
}

#[tauri::command]
pub(crate) async fn create_session(
    state: State<'_, AppState>,
    payload: CreateSessionPayload,
) -> Result<SessionInfo, String> {
    auth::create_session(&state.backend_url, &state.session_token, payload).await
}

#[tauri::command]
pub(crate) async fn delete_session(state: State<'_, AppState>) -> Result<(), String> {
    auth::delete_session(&state.backend_url, &state.session_token).await
}
