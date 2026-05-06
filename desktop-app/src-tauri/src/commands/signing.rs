use desktop_app::infrastructure::signing;

#[tauri::command]
pub(crate) fn compute_sighash(
    seqno: u64,
    action_hex: String,
) -> Result<signing::SighashResult, String> {
    signing::compute_sighash(seqno, &action_hex)
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
    signing::sign_sighash(&secret_key_hex, &sighash_hex)
}

#[tauri::command]
pub(crate) fn list_mnemonic_addresses(
    mnemonic: String,
    passphrase: Option<String>,
    count: Option<u32>,
) -> Result<Vec<signing::MnemonicAddressEntry>, String> {
    signing::list_mnemonic_addresses(
        &mnemonic,
        passphrase.as_deref().unwrap_or(""),
        count.unwrap_or(20),
    )
}

#[tauri::command]
pub(crate) fn sign_with_mnemonic_path(
    mnemonic: String,
    passphrase: Option<String>,
    derivation_path: String,
    sighash_hex: String,
) -> Result<signing::SignatureResult, String> {
    signing::sign_with_mnemonic_path(
        &mnemonic,
        passphrase.as_deref().unwrap_or(""),
        &derivation_path,
        &sighash_hex,
    )
}
