use bitcoin::bip32::DerivationPath;
use bitcoin::secp256k1::ecdsa::RecoverableSignature;
use std::str::FromStr;
use trezor_client::{InputScriptType, Trezor, TrezorMessage, TrezorResponse};

use super::HwWalletInfo;
use crate::signing::SignatureResult;

// ─── BIP-86 first receive address — Taproot (P2TR, bc1p...) ─────────────────
const DEFAULT_PATH: &str = "m/86'/0'/0'/0/0";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn open_trezor() -> Result<Trezor, String> {
    let mut trezor = trezor_client::unique(false)
        .map_err(|_| "Trezor not found. Connect the device.".to_string())?;
    trezor
        .init_device(None)
        .map_err(|e| format!("Trezor init failed: {e}"))?;
    Ok(trezor)
}

fn parse_path(path: &str) -> Result<DerivationPath, String> {
    DerivationPath::from_str(path).map_err(|e| format!("Invalid derivation path: {e}"))
}

/// Drive a TrezorResponse to completion, handling ButtonRequests along the way.
///
/// When the device sends a ButtonRequest it is showing a confirmation screen and
/// waiting for the user to press the physical button (or the emulator button).
/// We respond with ButtonAck so the device knows the client is ready, then block
/// until the user confirms and the device sends the final response.
fn resolve<'a, T, R: TrezorMessage>(mut response: TrezorResponse<'a, T, R>) -> Result<T, String> {
    loop {
        match response {
            TrezorResponse::Ok(data) => return Ok(data),
            TrezorResponse::Failure(f) => return Err(format!("Device failure: {:?}", f)),
            TrezorResponse::ButtonRequest(req) => {
                response = req.ack().map_err(|e| format!("ButtonAck failed: {e}"))?;
            }
            TrezorResponse::PinMatrixRequest(_) => {
                return Err("PIN entry not supported in this build.".to_string());
            }
            TrezorResponse::PassphraseRequest(_) => {
                return Err("Passphrase entry not supported in this build.".to_string());
            }
        }
    }
}

// ─── Public interface ─────────────────────────────────────────────────────────

/// Connect: read the compressed public key and address at the derivation path.
/// Mirrors TrezorConnect's getAddress + getPublicKey calls from the JS adapter.
pub fn connect(derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    let path_str = derivation_path.unwrap_or_else(|| DEFAULT_PATH.to_string());
    let path = parse_path(&path_str)?;

    let mut trezor = open_trezor()?;

    let xpub = resolve(
        trezor
            .get_public_key(
                &path,
                InputScriptType::SPENDTAPROOT,
                bitcoin::Network::Bitcoin,
                false,
            )
            .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
    )?;

    let pubkey_hex = hex::encode(xpub.public_key.serialize());

    let address: bitcoin::Address = resolve(
        trezor
            .get_address(
                &path,
                InputScriptType::SPENDTAPROOT,
                bitcoin::Network::Bitcoin,
                false,
            )
            .map_err(|e| format!("Trezor get_address failed: {e}"))?,
    )?;

    Ok(HwWalletInfo {
        device_label: "Trezor".to_string(),
        derivation_path: path_str,
        address_sample: Some(address.to_string()),
        xpub_or_fingerprint: Some(format!("{}…", &pubkey_hex[..16.min(pubkey_hex.len())])),
        key_label: Some("Public key".to_string()),
    })
}

/// Sign a message using Bitcoin Signed Message format (BIP-137 ECDSA).
/// The device shows the message on screen — the user confirms by pressing the
/// button on the physical device or emulator.
pub fn sign_message(sighash_hex: &str, derivation_path: &str) -> Result<SignatureResult, String> {
    let path = parse_path(derivation_path)?;

    let mut trezor = open_trezor()?;

    let xpub = resolve(
        trezor
            .get_public_key(
                &path,
                InputScriptType::SPENDTAPROOT,
                bitcoin::Network::Bitcoin,
                false,
            )
            .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
    )?;

    let public_key_hex = hex::encode(xpub.public_key.serialize());

    let (_address, recoverable_sig): (bitcoin::Address, RecoverableSignature) = resolve(
        trezor
            .sign_message(
                sighash_hex.to_string(),
                &path,
                InputScriptType::SPENDTAPROOT,
                bitcoin::Network::Bitcoin,
            )
            .map_err(|e| format!("Trezor sign_message failed: {e}"))?,
    )?;

    // Compact r+s (64 bytes)
    let (_, compact): (_, [u8; 64]) = recoverable_sig.serialize_compact();
    let signature_hex = hex::encode(compact);

    Ok(SignatureResult {
        public_key_hex,
        signature_hex,
    })
}
