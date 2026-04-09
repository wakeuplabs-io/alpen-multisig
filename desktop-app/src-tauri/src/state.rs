use std::sync::Mutex;

/// Shared application state managed by Tauri.
///
/// The session token lives here and is never exposed to the WebView.
/// All authenticated requests read it from this state and inject it as a Bearer header.
pub(crate) struct AppState {
    pub(crate) session_token: Mutex<Option<String>>,
    pub(crate) backend_url: String,
}
