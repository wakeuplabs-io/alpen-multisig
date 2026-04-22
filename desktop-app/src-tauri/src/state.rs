use bitcoin::secp256k1::SecretKey;
use std::sync::Mutex;

/// Shared application state managed by Tauri.
///
/// The session token and ephemeral key live here and are never exposed to the WebView.
/// All authenticated requests read them from this state and inject auth headers.
pub(crate) struct AppState {
    pub(crate) session_token: Mutex<Option<String>>,
    pub(crate) selected_authority: Mutex<Option<String>>,
    pub(crate) backend_url: String,
    /// Ephemeral private key generated at session start. Never leaves the Rust process.
    pub(crate) ephemeral_secret_key: Mutex<Option<SecretKey>>,
    pub(crate) ephemeral_pubkey_hex: Mutex<Option<String>>,
}
