//! G9/B0 — measures what a Ledger actually renders while signing the Admin ID Verification
//! Certificate message.
//!
//! PRD 06 req. 3.b.ii.iv asks the signer to "clearly read and understand each message they are
//! signing", and `desktop-app/src/lib/device-copy.ts` warns that a Ledger shows either the message
//! text or its SHA-256 hash depending on model and app version. This test drives the real signing
//! path against Speculos and records every screen the device paints, so what the client docs claim
//! is measured rather than assumed.
//!
//! Run with Speculos up:
//!   LEDGER_SPECULOS_URL=http://localhost:5001 \
//!   cargo test -p desktop-app --test g9_ledger_certificate_screens -- --nocapture

use desktop_app::infrastructure::hw_wallet::ledger;
use desktop_app::infrastructure::signing::SignatureResult;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const ADMIN_ID_PATH: &str = "m/84'/1'/73'/0/0";

/// One screen the device painted, as Speculos reports its text lines.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Screen {
    lines: Vec<String>,
}

impl Screen {
    /// Human-readable rendering, one line per Speculos text element.
    fn joined(&self) -> String {
        self.lines.join(" | ")
    }
}

/// Screen furniture the Bitcoin app draws around the payload: the step title, the page counter
/// and the action prompts. These sit *between* payload fragments, so they have to go before the
/// fragments can be concatenated back together.
fn is_chrome(line: &str) -> bool {
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
fn rendered_payload(screens: &[Screen]) -> String {
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
fn sign_and_record(
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
fn drop_idle_screen(screens: &[Screen]) -> Vec<Screen> {
    screens
        .iter()
        .filter(|s| !s.joined().contains("app is ready"))
        .cloned()
        .collect()
}

#[test]
fn ledger_certificate_message_screens() {
    let Ok(base) = std::env::var("LEDGER_SPECULOS_URL") else {
        eprintln!("skip: LEDGER_SPECULOS_URL not set");
        return;
    };
    // Auto-approve would click through the screens before we could read them.
    std::env::set_var("LEDGER_SPECULOS_AUTO_APPROVE", "0");

    let info = ledger::connect(Some(ADMIN_ID_PATH.to_string())).expect("ledger connect");
    let admin_id = info.address_sample.expect("admin id address");
    let message = format!("Admin ID: {admin_id}");
    eprintln!("admin id : {admin_id}");
    eprintln!("message  : {message}");

    let shot_dir = std::env::var("G9_SHOT_DIR").unwrap_or_else(|_| "/tmp".into());
    let (screens, signature) = sign_and_record(&base, &message, &shot_dir, "g9-b0-ledger");
    let signature = signature.expect("ledger signature");
    let screens = drop_idle_screen(&screens);

    let all_text = screens
        .iter()
        .map(Screen::joined)
        .collect::<Vec<_>>()
        .join("\n");
    eprintln!("\n--- screens ---\n{all_text}\n---------------");
    eprintln!("signature: {}", signature.signature_hex);

    let payload = rendered_payload(&screens);
    let showed_admin_id = payload.contains(&admin_id);
    let showed_hash = all_text.to_lowercase().contains("hash");
    eprintln!("shows Admin ID text : {showed_admin_id}");
    eprintln!("shows message hash  : {showed_hash}");

    assert!(
        !screens.is_empty(),
        "Speculos reported no screens — the device never rendered the signing flow"
    );
    assert!(
        showed_admin_id,
        "the device never rendered the Admin ID itself — PRD 06 req. 3.b.ii.iv is not met on this \
         device. Screens were:\n{all_text}"
    );
    assert!(
        !showed_hash,
        "the device fell back to the message hash; the signer cannot read what they sign. \
         Screens were:\n{all_text}"
    );
}

/// Why G3 measured a hash and G9 measures text: the Bitcoin app switches to the hash once the
/// message no longer fits its text-review flow. Opt in with `G9_MEASURE_THRESHOLD=1`; it drives
/// the same device, so it must not run alongside the test above (`--test-threads=1`).
#[test]
fn ledger_text_to_hash_threshold() {
    let Ok(base) = std::env::var("LEDGER_SPECULOS_URL") else {
        eprintln!("skip: LEDGER_SPECULOS_URL not set");
        return;
    };
    if std::env::var("G9_MEASURE_THRESHOLD").is_err() {
        eprintln!("skip: set G9_MEASURE_THRESHOLD=1 to run the threshold sweep");
        return;
    }
    std::env::set_var("LEDGER_SPECULOS_AUTO_APPROVE", "0");
    let shot_dir = std::env::var("G9_SHOT_DIR").unwrap_or_else(|_| "/tmp".into());

    let cases: Vec<(String, String)> = [51usize, 64, 96, 128, 160, 192, 224, 256]
        .into_iter()
        .map(|len| (format!("len-{len:03}"), "a".repeat(len)))
        .chain([
            // The two shapes that could explain G3's hash screens: a message far longer than any
            // review flow wants to paginate, and one carrying bytes the device cannot render.
            ("long-1000".to_string(), "a".repeat(1000)),
            (
                "non-ascii".to_string(),
                String::from_utf8_lossy(&[0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
                    .into_owned(),
            ),
            // The real session-authentication message (`auth_crypto.rs:10-12`), which G3 measured
            // as a hash, and the same text with its newlines replaced by spaces. If only the first
            // hashes, the trigger is the newline, not the length.
            (
                "session-lf".to_string(),
                format!(
                    "Strata Session Authentication v1\nRole: strata_administrator\nChallenge: {}",
                    "ab".repeat(32)
                ),
            ),
            (
                "session-sp".to_string(),
                format!(
                    "Strata Session Authentication v1 Role: strata_administrator Challenge: {}",
                    "ab".repeat(32)
                ),
            ),
        ])
        .collect();

    for (label, message) in cases {
        let len = message.len();
        let (screens, result) =
            sign_and_record(&base, &message, &shot_dir, &format!("g9-b0-{label}"));
        let screens = drop_idle_screen(&screens);
        let all_text = screens
            .iter()
            .map(Screen::joined)
            .collect::<Vec<_>>()
            .join(" / ");
        let hashed = all_text.to_lowercase().contains("hash");
        let text_shown = rendered_payload(&screens).contains(&message);
        eprintln!(
            "{label:10} (len {len:4}): hash={hashed:5} text={text_shown:5} signed={:5} | {all_text}",
            result.is_ok()
        );
    }
}
