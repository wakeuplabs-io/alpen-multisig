use desktop_app::infrastructure::dev_secrets;
use desktop_app::infrastructure::signing;

#[tauri::command]
pub(crate) fn compute_sighash(
    seqno: u64,
    action_hex: String,
) -> Result<signing::SighashResult, String> {
    signing::compute_sighash(seqno, &action_hex)
}

/// Reports whether mnemonic / raw-key signing over IPC is enabled (debug build, or
/// release with ALLOW_DEV_MNEMONIC_SIGNING=1). Lets the UI hide the Mnemonic option when off.
#[tauri::command]
pub(crate) fn dev_mnemonic_signing_enabled() -> bool {
    dev_secrets::dev_mnemonic_signing_ipc_enabled()
}

/// Canonical human-readable SPS-65 signing message for an action — the exact string the
/// device signs. Used by the UI to show what the connected device displays (Trezor renders
/// this text; Ledger renders its SHA-256 as the "Message hash"), instead of the BIP-137
/// sighash which never appears on-device.
#[tauri::command]
pub(crate) fn render_signing_message(seqno: u64, action_hex: String) -> Result<String, String> {
    signing::render_signing_message(seqno, &action_hex)
}

#[tauri::command]
pub(crate) fn verify_threshold(
    public_keys_hex: Vec<String>,
    threshold: u32,
    signatures_hex: Vec<String>,
    sighash_hex: String,
) -> Result<signing::VerifyResult, String> {
    signing::verify_threshold(&public_keys_hex, threshold, &signatures_hex, &sighash_hex)
}

#[tauri::command]
pub(crate) fn sign_action_sighash(
    secret_key_hex: String,
    sighash_hex: String,
) -> Result<signing::SignatureResult, String> {
    dev_secrets::ensure_dev_mnemonic_signing_allowed()?;
    signing::sign_sighash(&secret_key_hex, &sighash_hex)
}

#[tauri::command]
pub(crate) fn list_mnemonic_addresses(
    mnemonic: String,
    passphrase: Option<String>,
    count: Option<u32>,
) -> Result<Vec<signing::MnemonicAddressEntry>, String> {
    dev_secrets::ensure_dev_mnemonic_signing_allowed()?;
    signing::list_mnemonic_addresses(
        &mnemonic,
        passphrase.as_deref().unwrap_or(""),
        count.unwrap_or(20),
    )
}

#[tauri::command]
pub(crate) fn sign_message_with_mnemonic_path(
    mnemonic: String,
    passphrase: Option<String>,
    derivation_path: String,
    message: String,
) -> Result<signing::SignatureResult, String> {
    dev_secrets::ensure_dev_mnemonic_signing_allowed()?;
    signing::sign_message_with_mnemonic_path(
        &mnemonic,
        passphrase.as_deref().unwrap_or(""),
        &derivation_path,
        &message,
    )
}

#[tauri::command]
pub(crate) fn sign_with_mnemonic_path(
    mnemonic: String,
    passphrase: Option<String>,
    derivation_path: String,
    sighash_hex: String,
) -> Result<signing::SignatureResult, String> {
    dev_secrets::ensure_dev_mnemonic_signing_allowed()?;
    signing::sign_with_mnemonic_path(
        &mnemonic,
        passphrase.as_deref().unwrap_or(""),
        &derivation_path,
        &sighash_hex,
    )
}
