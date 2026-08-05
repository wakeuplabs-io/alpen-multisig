//! System-level IPC commands: clipboard write and native file-save dialog.
//!
//! On Linux/X11 the clipboard selection is served by the process that owns it. Creating
//! an `arboard::Clipboard`, calling `set_text`, and dropping the handle destroys that
//! owner and hands off via `SAVE_TARGETS` — which fails on desktops without a
//! freedesktop clipboard manager (e.g. Lubuntu/LXQt). Keeping one handle alive for the
//! app lifetime matches what `tauri-plugin-clipboard-manager` does and is how #428 is
//! addressed for those environments.

use std::sync::Mutex;

/// App-lifetime clipboard owner. Lazily opens `arboard` on the first write and never
/// drops the handle while the process is running, so X11 can still serve pastes.
#[derive(Default)]
pub struct ClipboardState {
    inner: Mutex<Option<arboard::Clipboard>>,
}

impl ClipboardState {
    /// Write `text` using a stable clipboard handle. Reuses the existing owner when
    /// present; opens one on first use.
    pub fn write_text(&self, text: &str) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "clipboard lock poisoned".to_string())?;
        if guard.is_none() {
            let clipboard =
                arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
            *guard = Some(clipboard);
        }
        guard
            .as_mut()
            .expect("clipboard handle set above")
            .set_text(text)
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn write_clipboard(
    text: String,
    state: tauri::State<'_, ClipboardState>,
) -> Result<(), String> {
    state.write_text(&text)
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

#[cfg(test)]
mod tests {
    use super::ClipboardState;

    /// Two writes through the same state must keep a single owner alive between calls
    /// (the invariant that fixes Lubuntu/LXQt). Skip when no display is available.
    #[test]
    fn clipboard_state_reuses_owner_across_writes() {
        let state = ClipboardState::default();
        let first = format!("probe-a-{}", std::process::id());
        let second = format!("probe-b-{}", std::process::id());

        if let Err(e) = state.write_text(&first) {
            eprintln!("skipping clipboard owner test (no display?): {e}");
            return;
        }

        // A second write must succeed against the same retained handle — not a fresh
        // Clipboard::new() that would drop the previous owner on Linux/X11.
        state
            .write_text(&second)
            .expect("second write on retained clipboard owner");

        let mut guard = state.inner.lock().expect("clipboard lock");
        let clipboard = guard.as_mut().expect("handle retained after writes");
        let read_back = clipboard.get_text().expect("read back retained clipboard");
        assert_eq!(read_back, second);
    }
}
