#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod signing;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            signing::generate_demo_keys,
            signing::compute_sighash,
            signing::sign_sighash,
            signing::verify_threshold,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
