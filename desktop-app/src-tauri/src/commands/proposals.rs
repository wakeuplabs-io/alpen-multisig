//! Proposal Tauri commands.

use crate::state::AppState;
use desktop_app::application::proposals;
use tauri::State;

#[tauri::command]
pub(crate) async fn list_proposals(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    let token = state
        .session_token
        .lock()
        .map_err(|_| "session_token lock poisoned")?
        .clone()
        .ok_or("Not authenticated")?;

    let authority = state
        .selected_authority
        .lock()
        .map_err(|_| "selected_authority lock poisoned")?
        .clone()
        .ok_or("No selected authority")?;

    let eph_sk = (*state
        .ephemeral_secret_key
        .lock()
        .map_err(|_| "ephemeral_secret_key lock poisoned")?)
    .ok_or("No ephemeral key — call authenticate first")?;

    proposals::fetch_proposals(&state.backend_url, &token, &authority, &eph_sk, status).await
}
