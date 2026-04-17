use std::str::FromStr;

use bitcoin::address::KnownHrp;
use bitcoin::bip32::{DerivationPath, Xpub};
use bitcoin::hashes::Hash;
use bitcoin::key::TweakedPublicKey;
use bitcoin::opcodes::all::OP_RETURN;
use bitcoin::psbt::{Input, Psbt, PsbtSighashType};
use bitcoin::script::Builder;
use bitcoin::secp256k1::ecdsa::Signature as EcdsaSignature;
use bitcoin::secp256k1::Message;
use bitcoin::secp256k1::XOnlyPublicKey;
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::Network;
use bitcoin::{absolute::LockTime, key::CompressedPublicKey, transaction::Version};
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness};
use trezor_client::client::handle_interaction;
use trezor_client::{InputScriptType, Trezor, TrezorMessage, TrezorResponse};

use super::{HwAddressEntry, HwWalletInfo};
use crate::infrastructure::signing::SignatureResult;

/// Product default derivation path (BIP86 Taproot).
const DEFAULT_PATH: &str = "m/86'/0'/73'/0/0";
const DUMMY_INPUT_VALUE: Amount = Amount::from_sat(100_000);
const DUMMY_FEE: Amount = Amount::from_sat(500);

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

fn decode_sighash_32(sighash_hex: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(sighash_hex).map_err(|e| format!("invalid sighash hex: {e}"))?;
    if bytes.len() != 32 {
        return Err(format!("sighash must be 32 bytes, got {}", bytes.len()));
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes);
    Ok(digest)
}

fn build_admin_commitment_psbt(
    xpub: &Xpub,
    path: &DerivationPath,
    admin_sighash: [u8; 32],
) -> Result<(Psbt, ScriptBuf), String> {
    let compressed_pk = CompressedPublicKey(xpub.public_key);
    let witness_script = ScriptBuf::new_p2wpkh(&compressed_pk.wpubkey_hash());
    let change_value = DUMMY_INPUT_VALUE
        .checked_sub(DUMMY_FEE)
        .ok_or("fee exceeds dummy input value")?;

    let op_return = TxOut {
        value: Amount::ZERO,
        script_pubkey: Builder::new()
            .push_opcode(OP_RETURN)
            .push_slice(admin_sighash)
            .into_script(),
    };
    let change_out = TxOut {
        value: change_value,
        script_pubkey: witness_script.clone(),
    };

    let unsigned_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: Txid::from_slice([0x11u8; 32].as_slice()).expect("valid txid length"),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![op_return, change_out],
    };

    let mut psbt = Psbt::from_unsigned_tx(unsigned_tx).map_err(|e| format!("PSBT from tx: {e}"))?;
    psbt.inputs[0] = Input {
        witness_utxo: Some(TxOut {
            value: DUMMY_INPUT_VALUE,
            script_pubkey: witness_script.clone(),
        }),
        bip32_derivation: [(xpub.public_key, (xpub.fingerprint(), path.clone()))]
            .into_iter()
            .collect(),
        sighash_type: Some(PsbtSighashType::from(EcdsaSighashType::All)),
        ..Default::default()
    };

    Ok((psbt, witness_script))
}

fn sign_tx_with_trezor(trezor: &mut Trezor, psbt: &Psbt) -> Result<Vec<u8>, String> {
    let mut response = trezor
        .sign_tx(psbt, Network::Bitcoin)
        .map_err(|e| format!("Trezor sign_tx failed: {e}"))?;
    let mut collected_signature = None;

    loop {
        let progress =
            handle_interaction(response).map_err(|e| format!("Trezor sign_tx interaction: {e}"))?;
        if let Some((_, sig)) = progress.get_signature() {
            collected_signature = Some(sig.to_vec());
        }
        if progress.finished() {
            return collected_signature.ok_or_else(|| {
                "Trezor sign_tx finished without returning a signature".to_string()
            });
        }
        response = progress
            .ack_psbt(psbt, Network::Bitcoin)
            .map_err(|e| format!("Trezor PSBT ack failed: {e}"))?;
    }
}

fn parse_ecdsa_signature(sig_bytes: &[u8]) -> Result<[u8; 64], String> {
    if sig_bytes.len() == 64 {
        let mut compact = [0u8; 64];
        compact.copy_from_slice(sig_bytes);
        return Ok(compact);
    }
    if sig_bytes.len() == 65 {
        let mut compact = [0u8; 64];
        compact.copy_from_slice(&sig_bytes[..64]);
        return Ok(compact);
    }
    if sig_bytes.first() == Some(&0x30) && sig_bytes.len() > 1 {
        let der = &sig_bytes[..sig_bytes.len() - 1];
        let signature =
            EcdsaSignature::from_der(der).map_err(|e| format!("invalid DER signature: {e}"))?;
        return Ok(signature.serialize_compact());
    }
    Err(format!(
        "unexpected signature format ({} bytes), expected compact or DER+sighash",
        sig_bytes.len()
    ))
}

fn verify_binding_signature(
    signature_compact: [u8; 64],
    xpub: &Xpub,
    tx: &Transaction,
    prev_script: &ScriptBuf,
) -> Result<(), String> {
    let sighash = SighashCache::new(tx.clone())
        .p2wpkh_signature_hash(
            0,
            prev_script.as_script(),
            DUMMY_INPUT_VALUE,
            EcdsaSighashType::All,
        )
        .map_err(|e| format!("could not compute segwit sighash: {e:?}"))?;
    let message = Message::from_digest_slice(&sighash.to_byte_array())
        .map_err(|e| format!("sighash message: {e}"))?;
    let signature = EcdsaSignature::from_compact(&signature_compact)
        .map_err(|e| format!("invalid compact signature: {e}"))?;
    bitcoin::secp256k1::SECP256K1
        .verify_ecdsa(&message, &signature, &xpub.public_key)
        .map_err(|e| format!("signature verification failed: {e}"))
}

/// Signs an SPS-65 digest through a synthetic Bitcoin tx binding approved on Trezor.
pub fn sign_admin_sps65_binding(
    sighash_hex: &str,
    derivation_path: &str,
) -> Result<SignatureResult, String> {
    let path = parse_path(derivation_path)?;
    let admin_sighash = decode_sighash_32(sighash_hex)?;
    let mut trezor = open_trezor()?;

    let xpub: Xpub = resolve(
        trezor
            .get_public_key(
                &path,
                InputScriptType::SPENDTAPROOT,
                Network::Bitcoin,
                false,
            )
            .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
    )?;

    let (psbt, prev_script) = build_admin_commitment_psbt(&xpub, &path, admin_sighash)?;
    let signature_raw = sign_tx_with_trezor(&mut trezor, &psbt)?;
    let signature_compact = parse_ecdsa_signature(&signature_raw)?;
    verify_binding_signature(signature_compact, &xpub, &psbt.unsigned_tx, &prev_script)?;

    Ok(SignatureResult {
        public_key_hex: hex::encode(xpub.public_key.serialize()),
        signature_hex: hex::encode(signature_compact),
    })
}
