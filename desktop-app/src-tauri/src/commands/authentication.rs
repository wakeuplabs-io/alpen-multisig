use desktop_app::application::authentication;
use desktop_app::application::authentication::{CompleteAuthInput, StartChallengeInput};
use desktop_app::domain::auth::{AuthChallenge, AuthSession};

#[tauri::command]
pub async fn auth_start_challenge(input: StartChallengeInput) -> Result<AuthChallenge, String> {
    authentication::start_challenge(input).await
}

#[tauri::command]
pub fn auth_complete(input: CompleteAuthInput) -> Result<AuthSession, String> {
    authentication::complete_auth(input)
}

#[tauri::command]
pub fn auth_get_session() -> Result<authentication::SessionResult, String> {
    authentication::get_session()
}

#[tauri::command]
pub fn auth_logout() -> Result<(), String> {
    authentication::logout()
}
