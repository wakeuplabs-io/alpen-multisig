use std::str::FromStr;

use bitcoin::address::KnownHrp;
use bitcoin::bip32::{DerivationPath, Xpub};
use bitcoin::Network;
use trezor_client::{protos, utils, InputScriptType, Trezor, TrezorMessage, TrezorResponse};

use super::{HwAddressEntry, HwWalletInfo};
use crate::infrastructure::signing::SignatureResult;

/// BIP-84 path for Admin ID (P2WPKH message signing, non-Payout-Admin multisigs).
const ADMIN_ID_PATH: &str = "m/84'/0'/73'/0/0";

/// BIP-84 path template for Admin ID (P2WPKH message signing).
const ADMIN_ID_PATH_PREFIX: &str = "m/84'/0'/73'/0/";

fn open_trezor() -> Result<Trezor, String> {
    let mut attempts = Vec::with_capacity(2);
    let mut saw_invalid_protocol = false;
    for debug in [false, true] {
        let mut trezor = match trezor_client::unique(debug) {
            Ok(device) => device,
            Err(e) => {
                attempts.push(format!("debug={debug}: discovery failed ({e})"));
                continue;
            }
        };

        match trezor.init_device(None) {
            Ok(_) => return Ok(trezor),
            Err(e) => {
                if e.to_string().contains("Failure_InvalidProtocol") {
                    saw_invalid_protocol = true;
                }
                attempts.push(format!("debug={debug}: init failed ({e})"));
            }
        }
    }

    let mut hint = "Ensure trezord/emulator are healthy, then reconnect the device.".to_string();
    if saw_invalid_protocol {
        hint.push_str(
            " If you are testing with an emulator, point this app directly to the emulator UDP port \
(21324) instead of the Trezor Bridge endpoint."
        );
    }

    Err(format!(
        "Trezor init failed on both transport modes. \
{} Details: {}",
        hint,
        attempts.join(" | ")
    ))
}

fn parse_path(path: &str) -> Result<DerivationPath, String> {
    DerivationPath::from_str(path).map_err(|e| format!("Invalid derivation path: {e}"))
}

/// Wrapper around `sign_message` that converts the Trezor response to the app's 65-byte
/// recoverable format `[r[32] | s[32] | recid]`, bypassing the library's
/// `parse_recoverable_signature` which fails on SegWit signatures.
///
/// Trezor returns `[header | r[32] | s[32]]` where `header = 27 + flags + recid`. For native
/// P2WPKH (BIP-84) `header = 39 + recid`. The library computes `39 - 31 = 8` and calls
/// `RecoveryId::from_i32(8)` which fails. The correct extraction is `(header - 27) & 0x03`.
fn sign_message_recoverable<'a>(
    trezor: &'a mut Trezor,
    message: &str,
    path: &DerivationPath,
    script_type: InputScriptType,
    network: Network,
) -> Result<TrezorResponse<'a, [u8; 65], protos::MessageSignature>, String> {
    let mut req = protos::SignMessage::new();
    req.address_n = utils::convert_path(path);
    req.set_message(message.as_bytes().to_vec());
    req.set_coin_name(utils::coin_name(network).map_err(|e| format!("coin_name: {e}"))?);
    req.set_script_type(script_type);
    trezor
        .call(
            req,
            Box::new(|_, m: protos::MessageSignature| {
                let sig = m.signature();
                if sig.len() != 65 || sig[0] < 27 {
                    return Err(trezor_client::Error::MalformedSignature);
                }
                let recid = (sig[0] - 27) & 0x03;
                let mut out = [0u8; 65];
                out[..64].copy_from_slice(&sig[1..]);
                out[64] = recid;
                Ok(out)
            }),
        )
        .map_err(|e: trezor_client::Error| e.to_string())
}

/// Wrapper around `get_public_key` that sets `ignore_xpub_magic = true` so Trezor returns
/// standard xpub version bytes instead of SLIP-0132 zpub/Zpub bytes (which `bitcoin::Xpub`
/// refuses to decode).
fn get_xpub<'a>(
    trezor: &'a mut Trezor,
    path: &DerivationPath,
    script_type: InputScriptType,
    network: Network,
    show_display: bool,
) -> Result<TrezorResponse<'a, Xpub, protos::PublicKey>, String> {
    let mut req = protos::GetPublicKey::new();
    req.address_n = utils::convert_path(path);
    req.set_show_display(show_display);
    req.set_coin_name(utils::coin_name(network).map_err(|e| format!("coin_name: {e}"))?);
    req.set_script_type(script_type);
    req.set_ignore_xpub_magic(true);
    trezor
        .call(req, Box::new(|_, m: protos::PublicKey| Ok(m.xpub().parse()?)))
        .map_err(|e: trezor_client::Error| e.to_string())
}

/// Drive a TrezorResponse to completion, handling ButtonRequests along the way.
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

/// Connect: read the P2WPKH Admin ID address at the BIP-84 derivation path.
pub fn connect(derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    let path_str = derivation_path.unwrap_or_else(|| ADMIN_ID_PATH.to_string());
    let path = parse_path(&path_str)?;

    let mut trezor = open_trezor()?;

    let xpub = resolve(
        get_xpub(&mut trezor, &path, InputScriptType::SPENDWITNESS, Network::Bitcoin, false)
            .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
    )?;

    let pubkey_hex = hex::encode(xpub.public_key.serialize());
    let compressed = bitcoin::CompressedPublicKey(xpub.public_key);
    let address = bitcoin::Address::p2wpkh(&compressed, KnownHrp::Mainnet);

    Ok(HwWalletInfo {
        device_label: "Trezor".to_string(),
        derivation_path: path_str,
        address_sample: Some(address.to_string()),
        xpub_or_fingerprint: Some(format!("{}…", &pubkey_hex[..16.min(pubkey_hex.len())])),
        key_label: Some("Public key".to_string()),
    })
}

/// Fetch the first `count` P2WPKH Admin ID addresses at `m/84'/0'/73'/0/{n}` (BIP-84).
///
/// Opens a single HID session and loops `count` `get_public_key` calls with
/// `InputScriptType::SPENDWITNESS`. Returns on the first error encountered.
pub fn list_addresses(count: usize) -> Result<Vec<HwAddressEntry>, String> {
    let mut trezor = open_trezor()?;
    let mut entries = Vec::with_capacity(count);

    for n in 0..count {
        let path_str = format!("{ADMIN_ID_PATH_PREFIX}{n}");
        let path = parse_path(&path_str)?;

        let xpub: Xpub = resolve(
            get_xpub(&mut trezor, &path, InputScriptType::SPENDWITNESS, Network::Bitcoin, false)
                .map_err(|e| format!("Trezor get_public_key at {path_str} failed: {e}"))?,
        )?;

        let public_key_hex = hex::encode(xpub.public_key.serialize());
        let compressed = bitcoin::CompressedPublicKey(xpub.public_key);
        let address = bitcoin::Address::p2wpkh(&compressed, KnownHrp::Mainnet);

        entries.push(HwAddressEntry {
            index: n as u32,
            derivation_path: path_str,
            address: address.to_string(),
            public_key_hex,
        });
    }

    Ok(entries)
}

pub fn verify_address_on_device(derivation_path: String) -> Result<(), String> {
    let path = parse_path(&derivation_path)?;
    let mut trezor = open_trezor()?;

    resolve(
        get_xpub(&mut trezor, &path, InputScriptType::SPENDWITNESS, Network::Bitcoin, true)
            .map_err(|e| format!("Trezor verify_address at {derivation_path} failed: {e}"))?,
    )?;

    Ok(())
}

/// Signs the canonical SPS-65 signing message on Trezor using Bitcoin `signMessage`.
///
/// `message` must be the human-readable string produced by
/// `SigningMessage::for_action(action, seqno).as_str()`. Trezor will compute
/// `Hash256(prefix || message)` internally, which matches `compute_sighash()` exactly.
///
/// Uses `SPENDWITNESS` (BIP-84 P2WPKH) which is required for `m/84'` Admin ID paths.
pub fn sign_admin_sps65_binding(
    message: &str,
    derivation_path: &str,
) -> Result<SignatureResult, String> {
    let path = parse_path(derivation_path)?;
    let mut trezor = open_trezor()?;

    let xpub: Xpub = resolve(
        get_xpub(&mut trezor, &path, InputScriptType::SPENDWITNESS, Network::Bitcoin, false)
            .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
    )?;

    let recoverable_sig = resolve(
        sign_message_recoverable(&mut trezor, message, &path, InputScriptType::SPENDWITNESS, Network::Bitcoin)
            .map_err(|e| format!("Trezor sign_message failed: {e}"))?,
    )?;

    Ok(SignatureResult {
        public_key_hex: hex::encode(xpub.public_key.serialize()),
        signature_hex: hex::encode(recoverable_sig),
    })
}
