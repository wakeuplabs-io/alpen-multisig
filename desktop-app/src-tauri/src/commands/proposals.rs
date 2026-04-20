//! Proposal Tauri commands.

use crate::state::AppState;
use desktop_app::application::proposals;
use tauri::State;

#[tauri::command]
pub(crate) async fn list_proposals(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    proposals::fetch_proposals(
        &state.backend_url,
        &state.session_token,
        &state.selected_authority,
        status,
    )
    .await
}
