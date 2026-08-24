//! Admin ID Verification Certificate commands (PRD 06 §3.c.i).
//!
//! Two thin wrappers, no device access and no wallet state: the signature itself comes from
//! the wallet adapter port that already signs the session challenge, so the Trezor, Ledger
//! and mnemonic paths stay in one place instead of being re-dispatched here.

use desktop_app::infrastructure::admin_id_certificate::{self, AdminIdCertificate};

/// Returns the exact string the modal displays in Step 1 and the signer signs.
#[tauri::command]
pub fn admin_id_certificate_message(admin_id: String) -> Result<String, String> {
    admin_id_certificate::certificate_message(&admin_id)
}

/// Encodes a signature over that message into a certificate, or fails.
///
/// The app never surfaces a certificate it has not verified: a signature whose recovered key
/// does not belong to `admin_id` is rejected here rather than shown and copied.
#[tauri::command]
pub fn build_admin_id_certificate(
    admin_id: String,
    signature_hex: String,
) -> Result<AdminIdCertificate, String> {
    admin_id_certificate::build_certificate(&admin_id, &signature_hex)
}
