//! Speculos screen capture shared by the device-screen measurements (G9/B0, G10/B0).
//!
//! Drives a signing flow against the emulator and records every screen the device paints, so what
//! the client docs claim about a device is measured rather than assumed. Each measurement owns its
//! message and its assertions; only the plumbing lives here.

#![allow(dead_code)]

use desktop_app::infrastructure::hw_wallet::ledger;
use desktop_app::infrastructure::signing::SignatureResult;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Derivation path of the Admin ID, the key every one of these measurements signs with.
pub const ADMIN_ID_PATH: &str = "m/84'/1'/73'/0/0";

/// One screen the device painted, as Speculos reports its text lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    lines: Vec<String>,
}

impl Screen {
    /// Human-readable rendering, one line per Speculos text element.
    pub fn joined(&self) -> String {
        self.lines.join(" | ")
    }
}

/// Screen furniture the Bitcoin app draws around the payload: the step title, the page counter
/// and the action prompts. These sit *between* payload fragments, so they have to go before the
/// fragments can be concatenated back together.
pub fn is_chrome(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("Message (") && line.ends_with(')')
        || matches!(
            line,
            "Path" | "Message" | "Sign message" | "Approve" | "Reject" | "Cancel"
        )
        || line.starts_with("84'/")
}

/// Speculos reports each text element separately, and the Bitcoin app **wraps** a long payload
/// across lines and pages. Dropping the chrome and concatenating the rest with no separator
/// reconstructs what a human reads off the device across the whole flow.
pub fn rendered_payload(screens: &[Screen]) -> String {
    screens
        .iter()
        .flat_map(|s| s.lines.iter())
        .map(|l| l.trim())
        .filter(|l| !is_chrome(l))
        .collect::<Vec<_>>()
        .concat()
}

struct Speculos {
    base: String,
    http: reqwest::Client,
}

impl Speculos {
    async fn current_screen(&self) -> Option<Screen> {
        let body = self
            .http
            .get(format!("{}/events?currentscreenonly=true", self.base))
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;
        let parsed: serde_json::Value = serde_json::from_str(&body).ok()?;
        let lines = parsed
            .get("events")?
            .as_array()?
            .iter()
            .filter_map(|e| e.get("text")?.as_str().map(str::to_string))
            .collect::<Vec<_>>();
        (!lines.is_empty()).then_some(Screen { lines })
    }

    async fn press(&self, button: &str) {
        let _ = self
            .http
            .post(format!("{}/button/{button}", self.base))
            .json(&serde_json::json!({ "action": "press-and-release" }))
            .send()
            .await;
    }

    async fn save_screenshot(&self, path: &str) {
        let Ok(resp) = self
            .http
            .get(format!("{}/screenshot", self.base))
            .send()
            .await
        else {
            return;
        };
        if let Ok(bytes) = resp.bytes().await {
            let _ = std::fs::write(path, bytes);
        }
    }
}

/// Signs `message` on the device while recording every screen it paints, saving one screenshot
/// per screen as `<shot_dir>/<prefix>-NN.png`. Returns the screens in the order they appeared.
pub fn sign_and_record(
    base: &str,
    message: &str,
    shot_dir: &str,
    prefix: &str,
) -> (Vec<Screen>, Result<SignatureResult, String>) {
    let (tx, rx) = mpsc::channel();
    let signing_message = message.to_string();
    let signer = std::thread::spawn(move || {
        let result = ledger::sign_bitcoin_message(&signing_message, ADMIN_ID_PATH);
        let _ = tx.send(());
        result
    });

    let speculos = Speculos {
        base: base.trim_end_matches('/').to_string(),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("http client"),
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let screens = rt.block_on(async {
        let mut seen: Vec<Screen> = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut shot = 0;
        while Instant::now() < deadline {
            if rx.try_recv().is_ok() {
                break;
            }
            if let Some(screen) = speculos.current_screen().await {
                if seen.last() != Some(&screen) {
                    eprintln!("screen {shot:02}: {}", screen.joined());
                    speculos
                        .save_screenshot(&format!("{shot_dir}/{prefix}-{shot:02}.png"))
                        .await;
                    seen.push(screen.clone());
                    shot += 1;
                }
                let text = screen.joined().to_lowercase();
                if text.contains("approve") || text.contains("sign message") {
                    speculos.press("both").await;
                } else {
                    speculos.press("right").await;
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        seen
    });

    (screens, signer.join().expect("signer thread"))
}

/// The device paints its idle screen before the flow starts; it is not part of what was signed.
pub fn drop_idle_screen(screens: &[Screen]) -> Vec<Screen> {
    screens
        .iter()
        .filter(|s| !s.joined().contains("app is ready"))
        .cloned()
        .collect()
}
