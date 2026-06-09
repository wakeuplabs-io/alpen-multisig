//! System-level IPC commands: clipboard write and native file-save dialog.

#[tauri::command]
pub fn write_clipboard(text: String) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_json_file(content: String, filename: String) -> Result<(), String> {
    let handle = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name(&filename)
        .save_file()
        .await;

    if let Some(path) = handle {
        std::fs::write(path.path(), content.as_bytes()).map_err(|e| e.to_string())?;
    }

    Ok(())
}
