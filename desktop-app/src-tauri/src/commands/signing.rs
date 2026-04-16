//! Tauri commands for the signing pipeline.
//!
//! Exposes the SPS-65 signing flow as IPC commands:
//!
//!   1. `compute_sighash` — SPS-65 tagged hash (seqno + action → 32-byte digest)
//!   2. `sign_sighash` — raw ECDSA compact sig (64 bytes; matches `verify_threshold`)
//!   3. `verify_threshold` — verify compact ECDSA sigs against a ThresholdConfig

use crate::state::AppState;
use desktop_app::signing::{self, SighashResult, SignatureResult, VerifyResult};
use tauri::State;

#[tauri::command]
pub async fn compute_sighash(
    _state: State<'_, AppState>,
    seqno: u64,
    action_hex: String,
) -> Result<SighashResult, String> {
    signing::compute_sighash(seqno, &action_hex)
}

#[tauri::command]
pub async fn sign_sighash(
    _state: State<'_, AppState>,
    secret_key_hex: String,
    sighash_hex: String,
) -> Result<SignatureResult, String> {
    signing::sign_sighash(&secret_key_hex, &sighash_hex)
}

#[tauri::command]
pub async fn verify_threshold(
    _state: State<'_, AppState>,
    public_keys_hex: Vec<String>,
    threshold: u32,
    signatures_hex: Vec<String>,
    sighash_hex: String,
) -> Result<VerifyResult, String> {
    signing::verify_threshold(&public_keys_hex, threshold, &signatures_hex, &sighash_hex)
}

