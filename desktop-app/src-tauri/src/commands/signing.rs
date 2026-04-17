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
