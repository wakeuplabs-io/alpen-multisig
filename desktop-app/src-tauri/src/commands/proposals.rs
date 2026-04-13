//! Proposal Tauri commands.

use crate::state::AppState;
use desktop_app::application::auth;
use tauri::State;

#[tauri::command]
pub async fn list_proposals(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    auth::fetch_proposals(&state.backend_url, &state.session_token, status).await
}
