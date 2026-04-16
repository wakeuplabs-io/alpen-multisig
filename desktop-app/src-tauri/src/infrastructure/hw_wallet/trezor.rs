use std::str::FromStr;

use bitcoin::address::KnownHrp;
use bitcoin::bip32::{DerivationPath, Xpub};
use bitcoin::key::TweakedPublicKey;
use bitcoin::secp256k1::XOnlyPublicKey;
use bitcoin::Network;
use trezor_client::{InputScriptType, Trezor, TrezorMessage, TrezorResponse};

use super::{HwAddressEntry, HwWalletInfo};

/// Product default derivation path (BIP86 Taproot).
const DEFAULT_PATH: &str = "m/86'/0'/73'/0/0";

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

/// Connect: read the P2TR address at the given derivation path (default `m/86'/0'/73'/0/0`).
pub fn connect(derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    let path_str = derivation_path.unwrap_or_else(|| DEFAULT_PATH.to_string());
    let path = parse_path(&path_str)?;

    let mut trezor = open_trezor()?;

    let xpub = resolve(
        trezor
            .get_public_key(
                &path,
                InputScriptType::SPENDTAPROOT,
                Network::Bitcoin,
                false,
            )
            .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
    )?;

    let pubkey_hex = hex::encode(xpub.public_key.serialize());

    let xonly = XOnlyPublicKey::from(xpub.public_key);
    let tweaked = TweakedPublicKey::dangerous_assume_tweaked(xonly);
    let address = bitcoin::Address::p2tr_tweaked(tweaked, KnownHrp::Mainnet);

    Ok(HwWalletInfo {
        device_label: "Trezor".to_string(),
        derivation_path: path_str,
        address_sample: Some(address.to_string()),
        xpub_or_fingerprint: Some(format!("{}…", &pubkey_hex[..16.min(pubkey_hex.len())])),
        key_label: Some("Public key".to_string()),
    })
}

/// UC-1: Fetch the first `count` P2TR addresses at `m/86'/0'/73'/0/{n}` (BIP86 Taproot).
///
/// Opens a single HID session and loops `count` `get_public_key` calls with
/// `InputScriptType::SPENDTAPROOT`. Returns on the first error encountered.
pub fn list_addresses(count: usize) -> Result<Vec<HwAddressEntry>, String> {
    let mut trezor = open_trezor()?;
    let mut entries = Vec::with_capacity(count);

    for n in 0..count {
        let path_str = format!("m/86'/0'/73'/0/{n}");
        let path = parse_path(&path_str)?;

        let xpub: Xpub = resolve(
            trezor
                .get_public_key(
                    &path,
                    InputScriptType::SPENDTAPROOT,
                    Network::Bitcoin,
                    false,
                )
                .map_err(|e| format!("Trezor get_public_key at {path_str} failed: {e}"))?,
        )?;

        let compressed_bytes = xpub.public_key.serialize();
        let public_key_hex = hex::encode(compressed_bytes);

        // BIP86 key-path P2TR: strip the parity byte and treat as an x-only key.
        // `dangerous_assume_tweaked` is correct here — BIP86 prescribes no script-tree tweak
        // for the standard key-path spend, so the x-only key IS the tweaked output key.
        let xonly = XOnlyPublicKey::from(xpub.public_key);
        let tweaked = TweakedPublicKey::dangerous_assume_tweaked(xonly);
        let address = bitcoin::Address::p2tr_tweaked(tweaked, KnownHrp::Mainnet);

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
        trezor
            .get_public_key(&path, InputScriptType::SPENDTAPROOT, Network::Bitcoin, true)
            .map_err(|e| format!("Trezor verify_address at {derivation_path} failed: {e}"))?,
    )?;

    Ok(())
}
