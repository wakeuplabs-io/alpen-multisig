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

mod ledger_screens;

use desktop_app::infrastructure::hw_wallet::ledger;
use ledger_screens::{drop_idle_screen, rendered_payload, sign_and_record, Screen, ADMIN_ID_PATH};

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
