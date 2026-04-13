#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use std::sync::Mutex;

fn main() {
    let backend_url =
        std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/api/v1".to_string());

    tauri::Builder::default()
        .manage(state::AppState {
            session_token: Mutex::new(None),
            backend_url,
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::get_challenge,
            commands::auth::create_session,
            commands::auth::delete_session,
            commands::proposals::list_proposals,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
