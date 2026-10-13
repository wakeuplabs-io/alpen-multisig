//! System-level IPC commands: clipboard write, native file-save dialog, and reading a
//! file dropped onto the window.
//!
//! On Linux/X11 the clipboard selection is served by the process that owns it. Creating
//! an `arboard::Clipboard`, calling `set_text`, and dropping the handle destroys that
//! owner and hands off via `SAVE_TARGETS` — which fails on desktops without a
//! freedesktop clipboard manager (e.g. Lubuntu/LXQt). Keeping one handle alive for the
//! app lifetime matches what `tauri-plugin-clipboard-manager` does and is how #428 is
//! addressed for those environments.
//!
//! File drops arrive as native window events, not HTML5 `drop` events: WebKitGTK does
//! not deliver dropped files to the DOM, and Windows blocks them while Tauri's native
//! drag-drop is on (#606). The window records the dropped paths and the frontend reads
//! one back by path — only a path from the last drop, so this is not a general file read.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Largest dropped file the app reads back. A proposal bundle is a few KB.
const MAX_DROPPED_FILE_BYTES: u64 = 1024 * 1024;

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

/// Paths from the most recent native file drop onto the window.
#[derive(Default)]
pub struct DroppedFilesState {
    paths: Mutex<Vec<PathBuf>>,
}

impl DroppedFilesState {
    /// Replaces the recorded paths with the ones from a new drop.
    pub fn record(&self, paths: &[PathBuf]) {
        if let Ok(mut guard) = self.paths.lock() {
            *guard = paths.to_vec();
        }
    }

    fn read_json(&self, path: &Path) -> Result<String, String> {
        let dropped = self
            .paths
            .lock()
            .map_err(|_| "dropped files lock poisoned".to_string())?;
        if !dropped.iter().any(|p| p == path) {
            return Err("This file was not dropped onto the app.".to_string());
        }
        let is_json = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
        if !is_json {
            return Err("Drop a .json bundle file.".to_string());
        }
        let metadata = std::fs::metadata(path).map_err(|e| format!("Could not read file: {e}"))?;
        if !metadata.is_file() {
            return Err("Drop a .json bundle file.".to_string());
        }
        if metadata.len() > MAX_DROPPED_FILE_BYTES {
            return Err("The file is too large to be a proposal bundle.".to_string());
        }
        std::fs::read_to_string(path).map_err(|e| format!("Could not read file: {e}"))
    }
}

/// Reads a `.json` file the user just dropped onto the window.
#[tauri::command]
pub fn read_dropped_json(
    path: String,
    state: tauri::State<'_, DroppedFilesState>,
) -> Result<String, String> {
    state.read_json(Path::new(&path))
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
    use super::{ClipboardState, DroppedFilesState, MAX_DROPPED_FILE_BYTES};
    use std::path::PathBuf;

    fn temp_file(name: &str, content: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dropped-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(name);
        std::fs::write(&path, content).expect("write temp file");
        path
    }

    #[test]
    fn dropped_json_is_read_back() {
        let path = temp_file("bundle.json", br#"{"seqNo":2}"#);
        let state = DroppedFilesState::default();
        state.record(std::slice::from_ref(&path));
        assert_eq!(state.read_json(&path).expect("read"), r#"{"seqNo":2}"#);
    }

    /// The command must not become a general file read: only paths from the last drop.
    #[test]
    fn path_not_from_a_drop_is_rejected() {
        let dropped = temp_file("dropped.json", b"{}");
        let other = temp_file("other.json", b"{}");
        let state = DroppedFilesState::default();
        state.record(&[dropped]);
        assert!(state.read_json(&other).is_err());
    }

    #[test]
    fn a_new_drop_replaces_the_previous_paths() {
        let first = temp_file("first.json", b"{}");
        let second = temp_file("second.json", b"{}");
        let state = DroppedFilesState::default();
        state.record(std::slice::from_ref(&first));
        state.record(std::slice::from_ref(&second));
        assert!(state.read_json(&first).is_err());
        assert!(state.read_json(&second).is_ok());
    }

    #[test]
    fn non_json_and_oversized_drops_are_rejected() {
        let text = temp_file("notes.txt", b"{}");
        let big = temp_file("big.json", &vec![b' '; MAX_DROPPED_FILE_BYTES as usize + 1]);
        let state = DroppedFilesState::default();
        state.record(&[text.clone(), big.clone()]);
        assert!(state.read_json(&text).is_err());
        assert!(state.read_json(&big).is_err());
    }

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
