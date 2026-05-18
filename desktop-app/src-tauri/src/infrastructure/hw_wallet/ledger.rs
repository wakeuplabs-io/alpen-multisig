use std::convert::TryFrom;
use std::str::FromStr;

use async_trait::async_trait;
use bitcoin::address::KnownHrp;
use bitcoin::bip32::{ChildNumber, DerivationPath};
use bitcoin::hashes::Hash;
use bitcoin::sign_message::signed_msg_hash;
use ledger_apdu::APDUCommand as ZondaxCmd;
use ledger_bitcoin_client::apdu::{APDUCommand, StatusWord};
use ledger_bitcoin_client::async_client::{self, BitcoinClient};
use ledger_transport_hid::{hidapi::HidApi, LedgerHIDError, TransportNativeHID};
use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::Secp256k1;

use super::HwWalletInfo;
use crate::infrastructure::signing::SignatureResult;

const ADMIN_ID_PATH: &str = "m/84'/1'/73'/0/0";
const ADMIN_ID_PATH_PREFIX: &str = "m/84'/1'/73'/0/";

// ---------------------------------------------------------------------------
// HID transport wrapper
// ---------------------------------------------------------------------------

struct HidTransport(TransportNativeHID);

#[derive(Debug)]
struct HidError(LedgerHIDError);

impl std::fmt::Display for HidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ledger HID error: {}", self.0)
    }
}

#[async_trait]
impl async_client::Transport for HidTransport {
    type Error = HidError;

    async fn exchange(&self, cmd: &APDUCommand) -> Result<(StatusWord, Vec<u8>), HidError> {
        let zondax_cmd = ZondaxCmd {
            cla: cmd.cla,
            ins: cmd.ins,
            p1: cmd.p1,
            p2: cmd.p2,
            data: cmd.data.clone(),
        };
        let answer = self.0.exchange(&zondax_cmd).map_err(HidError)?;
        let retcode = answer.retcode();
        let status = StatusWord::try_from(retcode)
            .map_err(|_| HidError(LedgerHIDError::Comm("unknown status word")))?;
        Ok((status, answer.apdu_data().to_vec()))
    }
}

// ---------------------------------------------------------------------------
// Speculos transport (HTTP REST — for emulator testing)
// ---------------------------------------------------------------------------

// Set LEDGER_SPECULOS_URL to http://localhost:5000 when running Speculos.

struct SpeculosTransport {
    base_url: String,
    client: reqwest::Client,
}

impl SpeculosTransport {
    fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug)]
struct SpeculosError(String);

impl std::fmt::Display for SpeculosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Speculos error: {}", self.0)
    }
}

#[async_trait]
impl async_client::Transport for SpeculosTransport {
    type Error = SpeculosError;

    async fn exchange(&self, cmd: &APDUCommand) -> Result<(StatusWord, Vec<u8>), SpeculosError> {
        let apdu_hex = hex::encode(cmd.encode());
        let body = serde_json::json!({ "data": apdu_hex });

        eprintln!("[speculos] → {apdu_hex}");
        let raw_resp = self
            .client
            .post(format!("{}/apdu", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| SpeculosError(e.to_string()))?;
        let body_text = raw_resp
            .text()
            .await
            .map_err(|e| SpeculosError(e.to_string()))?;
        eprintln!("[speculos] ← {body_text}");
        let resp: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| SpeculosError(format!("JSON parse error: {e} — body: {body_text}")))?;

        let data_hex = resp["data"]
            .as_str()
            .ok_or_else(|| SpeculosError("missing 'data' field in response".into()))?;
        let raw = hex::decode(data_hex).map_err(|e| SpeculosError(e.to_string()))?;

        if raw.len() < 2 {
            return Err(SpeculosError("response too short".into()));
        }

        let retcode = u16::from_be_bytes([raw[raw.len() - 2], raw[raw.len() - 1]]);
        let status = StatusWord::try_from(retcode)
            .map_err(|_| SpeculosError(format!("unknown status word: {retcode:#06x}")))?;
        let data = raw[..raw.len() - 2].to_vec();

        Ok((status, data))
    }
}

// ---------------------------------------------------------------------------
// Core operations (generic over transport)
// ---------------------------------------------------------------------------

fn parse_path(path: &str) -> Result<DerivationPath, String> {
    DerivationPath::from_str(path).map_err(|e| format!("invalid derivation path: {e}"))
}

fn map_ledger_error(op: &str, raw: &str) -> String {
    if raw.contains("InsNotSupported") || raw.contains("ClaNotSupported") {
        "Bitcoin app not responding — open the Bitcoin app (v2.1.0+) on your Ledger and try again"
            .to_string()
    } else if raw.contains("NotSupported") {
        "Ledger rejected this path — update the Bitcoin app to the latest version via Ledger Live"
            .to_string()
    } else if raw.contains("Deny") {
        "Request rejected on device — confirm the operation on your Ledger".to_string()
    } else if raw.contains("BadState") {
        "Ledger is locked or busy — unlock your device and open the Bitcoin app".to_string()
    } else {
        format!("Ledger {op} failed: {raw}")
    }
}

async fn get_info_with<T>(client: &BitcoinClient<T>, path_str: &str) -> Result<HwWalletInfo, String>
where
    T: async_client::Transport,
    T::Error: std::fmt::Debug,
{
    let path = parse_path(path_str)?;

    // The Bitcoin app rejects GET_EXTENDED_PUBKEY for paths that contain unhardened components.
    // Request the account-level xpub (hardened prefix only) and derive the remainder in software.
    let steps: Vec<ChildNumber> = path.into_iter().copied().collect();
    let split = steps
        .iter()
        .rposition(|c| c.is_hardened())
        .map(|i| i + 1)
        .unwrap_or(0);
    let account_path = DerivationPath::from(steps[..split].to_vec());
    let suffix = steps[split..].to_vec();

    let account_xpub = client
        .get_extended_pubkey(&account_path, false)
        .await
        .map_err(|e| map_ledger_error("get_extended_pubkey", &format!("{e:?}")))?;

    let leaf_xpub = if suffix.is_empty() {
        account_xpub
    } else {
        let secp = Secp256k1::verification_only();
        account_xpub
            .derive_pub(&secp, &suffix)
            .map_err(|e| format!("BIP32 derivation failed: {e}"))?
    };

    let pubkey_hex = hex::encode(leaf_xpub.public_key.serialize());
    let compressed = bitcoin::CompressedPublicKey(leaf_xpub.public_key);
    let address = bitcoin::Address::p2wpkh(&compressed, KnownHrp::Mainnet);

    Ok(HwWalletInfo {
        device_label: "Ledger".to_string(),
        derivation_path: path_str.to_string(),
        address_sample: Some(address.to_string()),
        xpub_or_fingerprint: Some(format!("{}…", &pubkey_hex[..16.min(pubkey_hex.len())])),
        key_label: Some("Public key".to_string()),
    })
}

async fn sign_with<T>(
    client: &BitcoinClient<T>,
    message: &str,
    derivation_path: &str,
) -> Result<SignatureResult, String>
where
    T: async_client::Transport,
    T::Error: std::fmt::Debug,
{
    let path = parse_path(derivation_path)?;

    let (header, sig) = client
        .sign_message(message.as_bytes(), &path)
        .await
        .map_err(|e| map_ledger_error("sign_message", &format!("{e:?}")))?;

    // Header: 27 + 4*(segwit flag) + recid  →  recid = (header - 27) & 0x03
    let recid_byte = (header - 27) & 0x03;
    let mut out = [0u8; 65];
    out[..64].copy_from_slice(&sig.serialize_compact());
    out[64] = recid_byte;

    // Recover the public key from the signature so we don't need a second round-trip
    // (and a second on-device confirmation) just to export the xpub.
    let public_key_hex = recover_pubkey_from_message(message, &out)
        .map_err(|e| format!("public key recovery failed: {e}"))?;

    Ok(SignatureResult {
        public_key_hex,
        signature_hex: hex::encode(out),
    })
}

fn recover_pubkey_from_message(message: &str, sig_bytes: &[u8; 65]) -> Result<String, String> {
    let recid = RecoveryId::from_i32(sig_bytes[64] as i32)
        .map_err(|e| format!("invalid recovery id: {e}"))?;
    let rec_sig = RecoverableSignature::from_compact(&sig_bytes[..64], recid)
        .map_err(|e| format!("invalid signature encoding: {e}"))?;
    let msg_hash = signed_msg_hash(message);
    let secp_msg = secp256k1::Message::from_digest(*msg_hash.as_byte_array());
    let secp = Secp256k1::verification_only();
    let pubkey = secp
        .recover_ecdsa(&secp_msg, &rec_sig)
        .map_err(|e| format!("recovery failed: {e}"))?;
    Ok(hex::encode(pubkey.serialize()))
}

async fn list_with<T>(client: &BitcoinClient<T>, count: usize) -> Result<Vec<super::HwAddressEntry>, String>
where
    T: async_client::Transport,
    T::Error: std::fmt::Debug,
{
    // Fetch the account xpub once (hardened prefix only) and software-derive each index.
    let account_path = parse_path("m/84'/1'/73'")?;
    let account_xpub = client
        .get_extended_pubkey(&account_path, false)
        .await
        .map_err(|e| map_ledger_error("get_extended_pubkey", &format!("{e:?}")))?;

    let secp = Secp256k1::verification_only();
    let mut entries = Vec::with_capacity(count);

    for n in 0..count {
        let suffix = DerivationPath::from(vec![
            ChildNumber::from(0u32),
            ChildNumber::from(n as u32),
        ]);
        let leaf_xpub = account_xpub
            .derive_pub(&secp, &suffix)
            .map_err(|e| format!("BIP32 derivation failed at index {n}: {e}"))?;

        let public_key_hex = hex::encode(leaf_xpub.public_key.serialize());
        let compressed = bitcoin::CompressedPublicKey(leaf_xpub.public_key);
        let address = bitcoin::Address::p2wpkh(&compressed, KnownHrp::Mainnet);

        entries.push(super::HwAddressEntry {
            index: n as u32,
            derivation_path: format!("{ADMIN_ID_PATH_PREFIX}{n}"),
            address: address.to_string(),
            public_key_hex,
        });
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Public API — sync wrappers called from spawn_blocking in Tauri commands
// ---------------------------------------------------------------------------

pub fn connect(derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    let path_str = derivation_path.unwrap_or_else(|| ADMIN_ID_PATH.to_string());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        let transport = SpeculosTransport::new(url);
        let client = BitcoinClient::new(transport);
        rt.block_on(get_info_with(&client, &path_str))
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(get_info_with(&client, &path_str))
    }
}

pub fn list_addresses(count: usize) -> Result<Vec<super::HwAddressEntry>, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        let transport = SpeculosTransport::new(url);
        let client = BitcoinClient::new(transport);
        rt.block_on(list_with(&client, count))
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(list_with(&client, count))
    }
}

pub fn sign_admin_sps65_binding(
    message: &str,
    derivation_path: &str,
) -> Result<SignatureResult, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        let transport = SpeculosTransport::new(url);
        let client = BitcoinClient::new(transport);
        rt.block_on(sign_with(&client, message, derivation_path))
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(sign_with(&client, message, derivation_path))
    }
}
