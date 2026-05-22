#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    desktop_app::infrastructure::env_loader::load_dotenv_files();

    commands::invoke::attach_invoke_handlers(tauri::Builder::default())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
