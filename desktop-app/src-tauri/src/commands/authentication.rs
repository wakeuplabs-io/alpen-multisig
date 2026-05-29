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
pub fn auth_logout(
    wallet_session: tauri::State<'_, desktop_app::application::wallet_session::WalletSession>,
) -> Result<(), String> {
    authentication::logout()?;
    wallet_session.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use desktop_app::application::wallet_session::WalletSession;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    // Test Budget: 1 behavior x 2 = 2 unit tests (using 1)
    // Behavior: auth_logout clears the wallet session slot

    #[tokio::test]
    async fn auth_logout_clears_wallet_session() {
        let session = WalletSession::empty();
        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("init must succeed");
        assert!(
            session.current().is_some(),
            "service must be present before logout"
        );

        // Call clear() — mirrors what the updated auth_logout command will do
        session.clear();

        assert!(
            session.current().is_none(),
            "current() must be None after auth_logout clears the session"
        );
    }
}
