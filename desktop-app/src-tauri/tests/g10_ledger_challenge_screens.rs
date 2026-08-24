//! G10 — what a Ledger renders while signing the **login challenge**, and whether the signature
//! it produces over that screen still authenticates.
//!
//! G9/B0 measured that the Bitcoin app falls back to the SHA-256 "Message hash" screen when the
//! message is not printable ASCII, and that the session-authentication string hashes because of
//! its `\n` separators. `pipe_separated_challenge_renders_as_text` is B0's gate: the ` | ` format
//! only earns the change if the device actually paints it, with a real 64-hex challenge, on the
//! same build. `what_the_device_shows_is_what_the_verifier_accepts` is B2's device QA: the same
//! run has to end in a signature the shipped verifier takes.
//!
//! Run with Speculos up:
//!   LEDGER_SPECULOS_URL=http://localhost:5001 \
//!   cargo test -p desktop-app --test g10_ledger_challenge_screens -- --nocapture --test-threads=1

mod ledger_screens;

use desktop_app::infrastructure::challenge_verifier;
use ledger_screens::{drop_idle_screen, rendered_payload, sign_and_record, Screen};

/// A challenge digest is a SHA-256, so the real message always carries 64 hex characters. Using a
/// shorter stand-in would measure a shorter message than the one signers see.
fn challenge_hex() -> String {
    "ab".repeat(32)
}

fn newline_format(challenge_hex: &str) -> String {
    format!("Strata Session Authentication v1\nRole: strata_admin\nChallenge: {challenge_hex}")
}

/// The format G10 ships (`auth_crypto.rs` / `challenge_verifier.rs` after B1).
fn pipe_format(challenge_hex: &str) -> String {
    format!("Strata Session Authentication v1 | Role: strata_admin | Challenge: {challenge_hex}")
}

struct Rendering {
    all_text: String,
    showed_hash: bool,
    showed_message: bool,
}

fn measure(base: &str, message: &str, prefix: &str) -> Rendering {
    let shot_dir = std::env::var("G10_SHOT_DIR").unwrap_or_else(|_| "/tmp".into());
    let (screens, _signature) = sign_and_record(base, message, &shot_dir, prefix);
    let screens = drop_idle_screen(&screens);
    let all_text = screens
        .iter()
        .map(Screen::joined)
        .collect::<Vec<_>>()
        .join("\n");
    eprintln!(
        "\n--- {prefix} (len {}) ---\n{all_text}\n---",
        message.len()
    );
    Rendering {
        showed_hash: all_text.to_lowercase().contains("hash"),
        // The device wraps and paginates the payload, so the text is compared after the chrome is
        // stripped and the fragments concatenated back together.
        showed_message: alphanumeric(&rendered_payload(&screens)).contains(&alphanumeric(message)),
        all_text,
    }
}

/// Compares on letters and digits alone. Everything dropped here is something the device controls
/// rather than the message: it breaks pages with `...` continuations, wraps mid-word across lines,
/// and Speculos reports each line separately — while `|` and the separator spaces are exactly the
/// characters the format change is about. What must come back character for character is the
/// content: the label, the role and all 64 hex digits of the challenge.
fn alphanumeric(text: &str) -> String {
    text.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// The gate: the shipped format must render as text, and the format it replaces must not — the
/// second half is what proves the change is the reason, not the device or the build.
#[test]
fn pipe_separated_challenge_renders_as_text() {
    let Ok(base) = std::env::var("LEDGER_SPECULOS_URL") else {
        eprintln!("skip: LEDGER_SPECULOS_URL not set");
        return;
    };
    // Auto-approve would click through the screens before they can be read.
    std::env::set_var("LEDGER_SPECULOS_AUTO_APPROVE", "0");

    let challenge = challenge_hex();
    let pipe = measure(&base, &pipe_format(&challenge), "g10-b0-pipe");
    let newline = measure(&base, &newline_format(&challenge), "g10-b0-newline");

    eprintln!(
        "pipe    : hash={} text={}\nnewline : hash={} text={}",
        pipe.showed_hash, pipe.showed_message, newline.showed_hash, newline.showed_message
    );

    assert!(
        !pipe.showed_hash,
        "the ` | ` challenge still fell back to the message hash — the format does not buy the \
         readable screen and G10 must stop here. Screens were:\n{}",
        pipe.all_text
    );
    assert!(
        pipe.showed_message,
        "the device did not render the challenge text. Screens were:\n{}",
        pipe.all_text
    );
    assert!(
        newline.showed_hash,
        "the newline format rendered as text on this build, so the newlines are not what G10 is \
         fixing — re-measure before changing the signing contract. Screens were:\n{}",
        newline.all_text
    );
}

/// The claim the loop actually has to make: what the signer reads off the device is the same
/// string the verifier accepts. Screens and signature come from one run, and both the message and
/// the verification come from the shipped code — not from a literal retyped in the test, which
/// would pass just as happily if production drifted.
#[test]
fn what_the_device_shows_is_what_the_verifier_accepts() {
    let Ok(base) = std::env::var("LEDGER_SPECULOS_URL") else {
        eprintln!("skip: LEDGER_SPECULOS_URL not set");
        return;
    };
    std::env::set_var("LEDGER_SPECULOS_AUTO_APPROVE", "0");

    let challenge = challenge_hex();
    let message = challenge_verifier::render_challenge_message("strata_admin", &challenge);
    let shot_dir = std::env::var("G10_SHOT_DIR").unwrap_or_else(|_| "/tmp".into());
    let (screens, signature) = sign_and_record(&base, &message, &shot_dir, "g10-b2-login");
    let signature = signature.expect("ledger signature over the challenge");
    let screens = drop_idle_screen(&screens);

    let all_text = screens
        .iter()
        .map(Screen::joined)
        .collect::<Vec<_>>()
        .join("\n");
    eprintln!("\n--- login challenge ---\n{all_text}\n---");

    assert!(
        !all_text.to_lowercase().contains("hash"),
        "the device fell back to the message hash for the shipped challenge format. \
         Screens were:\n{all_text}"
    );
    assert!(
        alphanumeric(&rendered_payload(&screens)).contains(&alphanumeric(&message)),
        "the device did not render the whole challenge. Screens were:\n{all_text}"
    );

    challenge_verifier::verify_bitcoin_message_signature(
        &message,
        &signature.public_key_hex,
        &signature.signature_hex,
    )
    .expect("the signature the device produced over what it displayed must verify");
}

/// The governance message — what a signer approves to enact a proposal. Unlike the challenge and the
/// certificate, this one is **not ours to reformat**: the ASM subprotocol crate builds it as
/// `lines.join("\n")` (`asm/crates/subprotocols/admin/txs/src/signing_message.rs`) and the
/// subprotocol verifies that exact payload.
///
/// Recorded here because the app's copy depends on it. #420 settled that a Ledger may show either
/// the text or the hash and that the app cannot predict which, so it shows both — and this is the
/// message that keeps the hash case live rather than residual: it renders as a hash on a Nano S+
/// with app 2.4.2, while the client measured full text for it on a Nano X. A future reader tempted
/// to simplify that copy to "the Ledger shows the text" should run this first.
///
/// Opt in with `MEASURE_GOVERNANCE=1`; drives the same device, so `--test-threads=1`.
#[test]
fn governance_message_screen_is_model_dependent() {
    let Ok(base) = std::env::var("LEDGER_SPECULOS_URL") else {
        eprintln!("skip: LEDGER_SPECULOS_URL not set");
        return;
    };
    if std::env::var("MEASURE_GOVERNANCE").is_err() {
        eprintln!("skip: set MEASURE_GOVERNANCE=1 to run this measurement");
        return;
    }
    std::env::set_var("LEDGER_SPECULOS_AUTO_APPROVE", "0");

    // A real signer-set update, as `SigningMessage::for_action` renders it.
    let message = "Strata ASM Administration v1\n\
         Action: Strata Administrator Multisig Update\n\
         Authorized By: Strata Administrator\n\
         Sequence: 4\n\
         Action Details:\n  \
         New Threshold: 2\n  \
         Members to Add: 1\n  \
         1. Add Member: 020202020202020202020202020202020202020202020202020202020202020202\n  \
         Members to Remove: 0";

    let shot_dir = std::env::var("G10_SHOT_DIR").unwrap_or_else(|_| "/tmp".into());
    let (screens, _sig) = sign_and_record(&base, message, &shot_dir, "governance-message");
    let screens = drop_idle_screen(&screens);
    let all_text = screens
        .iter()
        .map(Screen::joined)
        .collect::<Vec<_>>()
        .join("\n");

    let showed_hash = all_text.to_lowercase().contains("hash");
    let showed_text = alphanumeric(&rendered_payload(&screens)).contains(&alphanumeric(message));
    eprintln!(
        "\n--- governance message (len {}) ---\n{all_text}\n---",
        message.len()
    );
    eprintln!("hash={showed_hash} text={showed_text}");
}
