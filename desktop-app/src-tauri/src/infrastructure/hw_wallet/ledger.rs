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

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::{AddressScriptType, HwWalletInfo};
use crate::infrastructure::signing::SignatureResult;

/// Ledger HID and Speculos HTTP only handle one APDU exchange at a time.
static LEDGER_DEVICE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn with_ledger_device<R>(f: impl FnOnce() -> Result<R, String>) -> Result<R, String> {
    let lock = LEDGER_DEVICE_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock
        .lock()
        .map_err(|e| format!("Ledger device lock poisoned: {e}"))?;
    f()
}

const ADMIN_ID_PATH: &str = "m/84'/1'/73'/0/0";

/// Empty label for `sign_psbt` / `get_wallet_address` (matches ledger_bitcoin_client test vectors).
const LEDGER_ADMIN_WALLET_POLICY_NAME: &str = "";
/// Non-empty fallback name only when physical Ledger requires `register_wallet`.
const LEDGER_REGISTER_FALLBACK_POLICY_NAME: &str = "A";

/// Bitcoin app PSBT review/sign flow for headless Speculos (POST /automation — no emulator patch).
const SPECULOS_SIGN_AUTOMATION: &str = r#"{
  "version": 1,
  "rules": [
    {
      "text": "Sign transaction",
      "actions": [
        ["button", 1, true], ["button", 2, true],
        ["button", 1, false], ["button", 2, false]
      ]
    },
    { "text": "Review transaction", "actions": [["button", 2, true], ["button", 2, false]] },
    { "text": "Loading transaction", "actions": [["button", 2, true], ["button", 2, false]] },
    { "regexp": "To \\(", "actions": [["button", 2, true], ["button", 2, false]] },
    { "text": "Amount", "actions": [["button", 2, true], ["button", 2, false]] },
    { "text": "Fees", "actions": [["button", 2, true], ["button", 2, false]] }
  ]
}"#;

async fn upload_speculos_sign_automation(base_url: &str) -> bool {
    let url = format!("{}/automation", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(SPECULOS_SIGN_AUTOMATION)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            eprintln!("[speculos] automation rules loaded via POST {url}");
            true
        }
        Ok(resp) => {
            eprintln!(
                "[speculos] automation upload failed: HTTP {}",
                resp.status()
            );
            false
        }
        Err(e) => {
            eprintln!("[speculos] automation upload error: {e}");
            false
        }
    }
}

/// Whether Speculos PSBT approval should be auto-clicked via `/automation` rules.
///
/// Default: enabled (keeps the integration test / CI non-interactive). Set
/// `LEDGER_SPECULOS_AUTO_APPROVE=0` (or `false`/`no`) to **require manual approval**
/// on the Speculos screen — useful to observe the real on-device interaction and the
/// frontend "Confirm on your device" / timeout flow, mirroring a physical Ledger.
fn speculos_auto_approve_enabled() -> bool {
    match std::env::var("LEDGER_SPECULOS_AUTO_APPROVE") {
        Ok(v) => !(v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("no")),
        Err(_) => true,
    }
}

/// Loads Speculos `/automation` rules for headless PSBT approval via REST API,
/// unless manual approval was requested (`LEDGER_SPECULOS_AUTO_APPROVE=0`).
async fn with_speculos_auto_approve<F, Fut, T>(f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    if let Ok(base) = std::env::var("LEDGER_SPECULOS_URL") {
        if speculos_auto_approve_enabled() {
            let _ = upload_speculos_sign_automation(&base).await;
        } else {
            eprintln!(
                "[speculos] auto-approve disabled (LEDGER_SPECULOS_AUTO_APPROVE=0) — \
                 approve the transaction manually on the Speculos screen"
            );
        }
    }
    f().await
}

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
// Speculos transport (TCP APDU preferred; HTTP REST fallback)
// ---------------------------------------------------------------------------
//
// `sign_psbt` uses many APDU round-trips (InterruptedExecution).
// Prefer native Speculos TCP APDU; HTTP `/apdu` remains as fallback.

#[derive(Debug)]
struct SpeculosError(String);

impl std::fmt::Display for SpeculosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Speculos error: {}", self.0)
    }
}

fn parse_speculos_apdu_response(raw: &[u8]) -> Result<(StatusWord, Vec<u8>), SpeculosError> {
    if raw.len() < 2 {
        return Err(SpeculosError("response too short".into()));
    }
    let retcode = u16::from_be_bytes([raw[raw.len() - 2], raw[raw.len() - 1]]);
    let status = StatusWord::try_from(retcode)
        .map_err(|_| SpeculosError(format!("unknown status word: {retcode:#06x}")))?;
    Ok((status, raw[..raw.len() - 2].to_vec()))
}

/// Native Speculos APDU socket (length-prefixed framing, same as ledger-live-http-proxy).
struct SpeculosTcpTransport {
    stream: tokio::sync::Mutex<tokio::net::TcpStream>,
}

impl SpeculosTcpTransport {
    async fn connect(addr: &str) -> Result<Self, SpeculosError> {
        let stream = tokio::net::TcpStream::connect(addr)
            .await
            .map_err(|e| SpeculosError(format!("TCP connect to {addr} failed: {e}")))?;
        Ok(Self {
            stream: tokio::sync::Mutex::new(stream),
        })
    }
}

#[async_trait]
impl async_client::Transport for SpeculosTcpTransport {
    type Error = SpeculosError;

    async fn exchange(&self, cmd: &APDUCommand) -> Result<(StatusWord, Vec<u8>), SpeculosError> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let apdu = cmd.encode();

        let mut stream = self.stream.lock().await;
        let len = (apdu.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .map_err(|e| SpeculosError(format!("TCP write failed: {e}")))?;
        stream
            .write_all(&apdu)
            .await
            .map_err(|e| SpeculosError(format!("TCP write failed: {e}")))?;

        let mut size_buf = [0u8; 4];
        stream
            .read_exact(&mut size_buf)
            .await
            .map_err(|e| SpeculosError(format!("TCP read failed: {e}")))?;
        let size = u32::from_be_bytes(size_buf) as usize;
        let mut packet = vec![0u8; size.saturating_add(2)];
        stream
            .read_exact(&mut packet)
            .await
            .map_err(|e| SpeculosError(format!("TCP read failed: {e}")))?;

        parse_speculos_apdu_response(&packet)
    }
}

struct SpeculosHttpTransport {
    base_url: String,
    client: reqwest::Client,
}

impl SpeculosHttpTransport {
    fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { base_url, client }
    }
}

#[async_trait]
impl async_client::Transport for SpeculosHttpTransport {
    type Error = SpeculosError;

    async fn exchange(&self, cmd: &APDUCommand) -> Result<(StatusWord, Vec<u8>), SpeculosError> {
        let apdu_hex = hex::encode(cmd.encode());
        let body = serde_json::json!({ "data": apdu_hex });

        let raw_resp = self
            .client
            .post(format!("{}/apdu", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| SpeculosError(e.to_string()))?;
        let status = raw_resp.status();
        let bytes = raw_resp.bytes().await.map_err(|e| {
            SpeculosError(format!(
                "error reading Speculos HTTP body (approve on device if prompted): {e}"
            ))
        })?;
        if !status.is_success() {
            return Err(SpeculosError(format!(
                "Speculos HTTP {}: {}",
                status,
                String::from_utf8_lossy(&bytes)
            )));
        }
        let body_preview = String::from_utf8_lossy(&bytes);
        let resp: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| SpeculosError(format!("JSON parse error: {e} — body: {body_preview}")))?;
        if let Some(err) = resp.get("error").and_then(|v| v.as_str()) {
            if !err.is_empty() {
                return Err(SpeculosError(format!("Speculos APDU error: {err}")));
            }
        }

        let data_hex = resp["data"]
            .as_str()
            .ok_or_else(|| SpeculosError("missing 'data' field in response".into()))?;
        let raw = hex::decode(data_hex).map_err(|e| SpeculosError(e.to_string()))?;
        parse_speculos_apdu_response(&raw)
    }
}

enum SpeculosTransportEnum {
    Tcp(SpeculosTcpTransport),
    Http(SpeculosHttpTransport),
}

#[async_trait]
impl async_client::Transport for SpeculosTransportEnum {
    type Error = SpeculosError;

    async fn exchange(&self, cmd: &APDUCommand) -> Result<(StatusWord, Vec<u8>), SpeculosError> {
        match self {
            Self::Tcp(t) => t.exchange(cmd).await,
            Self::Http(h) => h.exchange(cmd).await,
        }
    }
}

async fn bitcoin_client_for_speculos(
    http_base: &str,
) -> Result<BitcoinClient<SpeculosTransportEnum>, SpeculosError> {
    let tcp_addr =
        std::env::var("LEDGER_SPECULOS_APDU_ADDR").unwrap_or_else(|_| "127.0.0.1:9999".to_string());
    if let Ok(tcp) = SpeculosTcpTransport::connect(&tcp_addr).await {
        eprintln!("[speculos] using TCP APDU at {tcp_addr}");
        return Ok(BitcoinClient::new(SpeculosTransportEnum::Tcp(tcp)));
    }
    eprintln!("[speculos] TCP connect to {tcp_addr} failed; using HTTP {http_base}/apdu");
    Ok(BitcoinClient::new(SpeculosTransportEnum::Http(
        SpeculosHttpTransport::new(http_base.to_string()),
    )))
}

// ---------------------------------------------------------------------------
// Core operations (generic over transport)
// ---------------------------------------------------------------------------

fn parse_path(path: &str) -> Result<DerivationPath, String> {
    DerivationPath::from_str(path).map_err(|e| format!("invalid derivation path: {e}"))
}

/// HRP for a connect address sample, derived from the path's coin type so the app value
/// matches what the device renders: coin type `1'` (test nets) → `tb`; otherwise `bc`.
/// The Ledger has no regtest app, so coin type `1'` shows as testnet (`tb`), not `bcrt`.
fn hrp_from_path(path: &DerivationPath) -> KnownHrp {
    if matches!(
        path.into_iter().nth(1),
        Some(ChildNumber::Hardened { index: 1 })
    ) {
        KnownHrp::Testnets
    } else {
        KnownHrp::Mainnet
    }
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
    } else if raw.contains("UnexpectedResult") {
        format!(
            "Ledger {op} returned an unexpected response (often caused by concurrent Speculos/Ledger requests). \
             Retry once; if it persists, restart Speculos and use only one wallet action at a time. Detail: {raw}"
        )
    } else if raw.contains("decoding response body") || raw.contains("reading Speculos HTTP body") {
        format!(
            "Ledger {op} lost the Speculos connection while waiting for on-device approval. \
             Restart Speculos with ./scripts/ledger-up.sh, then retry. Detail: {raw}"
        )
    } else if raw.contains("UnicodeDecodeError") || raw.contains("invalid start byte") {
        format!(
            "Ledger {op}: Speculos crashed while showing wallet registration on screen. \
             Restart ./scripts/ledger-up.sh and retry."
        )
    } else if raw.contains("error sending request")
        || raw.contains("Connection refused")
        || raw.contains("connection refused")
        || raw.contains("tcp connect")
    {
        let url = std::env::var("LEDGER_SPECULOS_URL")
            .unwrap_or_else(|_| "http://localhost:5001".to_string());
        format!(
            "Cannot reach Ledger emulator at {url} — Speculos is not running. \
             Start it: ./scripts/ledger-up.sh <path-to-bitcoin_testnet_*.elf>, then retry."
        )
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
    let address = bitcoin::Address::p2wpkh(&compressed, hrp_from_path(&path));

    Ok(HwWalletInfo {
        device_label: "Ledger".to_string(),
        derivation_path: path_str.to_string(),
        address_sample: Some(address.to_string()),
        public_key_hex: Some(pubkey_hex.clone()),
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

// ---------------------------------------------------------------------------
// Public API — sync wrappers called from spawn_blocking in Tauri commands
// ---------------------------------------------------------------------------

pub fn connect(derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    with_ledger_device(|| connect_unlocked(derivation_path))
}

fn connect_unlocked(derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    let path_str = derivation_path.unwrap_or_else(|| ADMIN_ID_PATH.to_string());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        rt.block_on(async {
            let client = bitcoin_client_for_speculos(&url)
                .await
                .map_err(|e| e.to_string())?;
            get_info_with(&client, &path_str).await
        })
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(get_info_with(&client, &path_str))
    }
}

/// Signs an arbitrary human-readable message on Ledger using its `sign_message` call —
/// the SPS-65 signing message, the session challenge, or the Admin ID Verification
/// Certificate message. The device hashes it as BIP-137 either way.
pub fn sign_bitcoin_message(
    message: &str,
    derivation_path: &str,
) -> Result<SignatureResult, String> {
    with_ledger_device(|| sign_bitcoin_message_unlocked(message, derivation_path))
}

fn sign_bitcoin_message_unlocked(
    message: &str,
    derivation_path: &str,
) -> Result<SignatureResult, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        rt.block_on(async {
            let client = bitcoin_client_for_speculos(&url)
                .await
                .map_err(|e| e.to_string())?;
            sign_with(&client, message, derivation_path).await
        })
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(sign_with(&client, message, derivation_path))
    }
}

/// Returns the BIP-86 (Taproot) account xpub for the given derivation path.
///
/// Uses `get_extended_pubkey` on the hardened account path directly.
/// Supports Speculos emulator via `LEDGER_SPECULOS_URL` environment variable.
pub fn get_account_xpub(path: &str) -> Result<String, String> {
    with_ledger_device(|| get_account_xpub_unlocked(path))
}

fn get_account_xpub_unlocked(path: &str) -> Result<String, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    let derivation_path =
        DerivationPath::from_str(path).map_err(|e| format!("Invalid path: {e}"))?;

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        rt.block_on(async {
            let client = bitcoin_client_for_speculos(&url)
                .await
                .map_err(|e| e.to_string())?;
            client
                .get_extended_pubkey(&derivation_path, false)
                .await
                .map(|xpub| xpub.to_string())
                .map_err(|e| map_ledger_error("get_extended_pubkey", &format!("{e:?}")))
        })
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(async {
            client
                .get_extended_pubkey(&derivation_path, false)
                .await
                .map(|xpub| xpub.to_string())
                .map_err(|e| map_ledger_error("get_extended_pubkey", &format!("{e:?}")))
        })
    }
}

/// Returns the master fingerprint from the Ledger device.
/// Uses the BitcoinClient's native get_master_fingerprint method.
fn admin_wallet_account_origin_path(network: bitcoin::Network) -> &'static str {
    match network {
        bitcoin::Network::Bitcoin => "86'/0'/73'",
        _ => "86'/1'/73'",
    }
}

fn build_admin_wallet_policy(
    account_xpub: &str,
    master_fingerprint: u32,
    network: bitcoin::Network,
) -> Result<ledger_bitcoin_client::wallet::WalletPolicy, String> {
    build_admin_wallet_policy_named(
        account_xpub,
        master_fingerprint,
        network,
        LEDGER_ADMIN_WALLET_POLICY_NAME,
    )
}

fn build_admin_wallet_policy_named(
    account_xpub: &str,
    master_fingerprint: u32,
    network: bitcoin::Network,
    policy_name: &str,
) -> Result<ledger_bitcoin_client::wallet::WalletPolicy, String> {
    use ledger_bitcoin_client::wallet::{Version, WalletPolicy, WalletPubKey};
    use std::str::FromStr;

    let fingerprint = bitcoin::bip32::Fingerprint::from(master_fingerprint.to_le_bytes());
    let key_str = format!(
        "[{}/{}]{}",
        fingerprint,
        admin_wallet_account_origin_path(network),
        account_xpub
    );
    let key = WalletPubKey::from_str(&key_str)
        .map_err(|e| format!("invalid wallet pubkey for Ledger policy: {e}"))?;
    Ok(WalletPolicy::new(
        policy_name.to_string(),
        Version::V2,
        "tr(@0/**)".to_string(),
        vec![key],
    ))
}

fn apply_ledger_signatures(
    psbt: &mut bitcoin::psbt::Psbt,
    yielded: Vec<(usize, ledger_bitcoin_client::SignPsbtYieldedObject)>,
) -> Result<(), String> {
    use ledger_bitcoin_client::psbt::PartialSignature;
    use ledger_bitcoin_client::SignPsbtYieldedObject;

    for (input_index, obj) in yielded {
        let SignPsbtYieldedObject::Partial(partial) = obj else {
            continue;
        };
        let input = psbt
            .inputs
            .get_mut(input_index)
            .ok_or_else(|| format!("Ledger returned signature for unknown input {input_index}"))?;
        match partial {
            PartialSignature::TapScriptSig(_pk, None, sig) => {
                input.tap_key_sig = Some(sig);
            }
            PartialSignature::TapScriptSig(pk, Some(leaf), sig) => {
                input.tap_script_sigs.insert((pk, leaf), sig);
            }
            PartialSignature::Sig(pk, sig) => {
                input.partial_sigs.insert(pk, sig);
            }
        }
    }
    Ok(())
}

/// Ledger PSBT commit funding — **not** the same protocol as login/proposal `sign_message`.
///
/// | Flow | Ledger API | Emulator screens | APDU round-trips |
/// |------|------------|------------------|------------------|
/// | Session auth / approve proposal | `sign_message` (BIP-137 text) | “Sign message” | Few |
/// | Broadcast commit (this fn) | `sign_psbt` (zero HMAC; no `register_wallet` on Speculos) | “Review transaction” | Many (`InterruptedExecution`) |
async fn sign_psbt_with_client<T>(
    client: &BitcoinClient<T>,
    psbt: &mut bitcoin::psbt::Psbt,
    account_xpub: &str,
    master_fingerprint: u32,
    network: bitcoin::Network,
) -> Result<(), String>
where
    T: async_client::Transport + Sync,
    T::Error: std::fmt::Debug,
{
    let wallet_policy = build_admin_wallet_policy(account_xpub, master_fingerprint, network)?;

    eprintln!("[ledger] broadcast commit: verify master fingerprint");
    let actual_fp = client
        .get_master_fingerprint()
        .await
        .map_err(|e| map_ledger_error("get_master_fingerprint", &format!("{e:?}")))?;
    let actual = u32::from_le_bytes(actual_fp.to_bytes());
    if actual != master_fingerprint {
        return Err(format!(
            "wrong Ledger device: expected fingerprint 0x{master_fingerprint:08X}, got 0x{actual:08X}"
        ));
    }

    let is_speculos = std::env::var("LEDGER_SPECULOS_URL").is_ok();

    eprintln!(
        "[ledger] broadcast commit: sign_psbt (zero HMAC — standard tr(@0/**) policy, no register_wallet)"
    );
    match client.sign_psbt(psbt, &wallet_policy, None).await {
        Ok(yielded) => {
            eprintln!("[ledger] broadcast commit: sign_psbt OK");
            apply_ledger_signatures(psbt, yielded)
        }
        Err(first) if !is_speculos => {
            let raw = format!("{first:?}");
            eprintln!(
                "[ledger] broadcast commit: sign_psbt failed ({raw}); trying register_wallet"
            );
            let register_policy = build_admin_wallet_policy_named(
                account_xpub,
                master_fingerprint,
                network,
                LEDGER_REGISTER_FALLBACK_POLICY_NAME,
            )?;
            let (_id, hmac) = client
                .register_wallet(&register_policy)
                .await
                .map_err(|e| map_ledger_error("register_wallet", &format!("{e:?}")))?;
            let yielded = client
                .sign_psbt(psbt, &register_policy, Some(&hmac))
                .await
                .map_err(|e| map_ledger_error("sign_psbt", &format!("{e:?}")))?;
            eprintln!("[ledger] broadcast commit: sign_psbt OK (after register_wallet)");
            apply_ledger_signatures(psbt, yielded)
        }
        Err(e) => Err(map_ledger_error("sign_psbt", &format!("{e:?}"))),
    }
}

/// Taproot key-path PSBT signing for Admin Wallet commit funding (Ledger / Speculos).
///
/// Must run on a **blocking thread** (`tokio::task::spawn_blocking`). Do not call from an async
/// Tokio worker — that deadlocks when this function creates its own runtime.
pub fn sign_admin_wallet_psbt(
    psbt: &mut bitcoin::psbt::Psbt,
    account_xpub: &str,
    master_fingerprint: u32,
    network: bitcoin::Network,
) -> Result<(), String> {
    with_ledger_device(|| {
        sign_admin_wallet_psbt_unlocked(psbt, account_xpub, master_fingerprint, network)
    })
}

fn sign_admin_wallet_psbt_unlocked(
    psbt: &mut bitcoin::psbt::Psbt,
    account_xpub: &str,
    master_fingerprint: u32,
    network: bitcoin::Network,
) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    eprintln!("[ledger] sign_admin_wallet_psbt: sign on device (zero HMAC policy)");

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        eprintln!("[ledger] using Speculos at {url}");
        rt.block_on(async {
            let client = bitcoin_client_for_speculos(&url)
                .await
                .map_err(|e| e.to_string())?;
            with_speculos_auto_approve(|| async {
                sign_psbt_with_client(&client, psbt, account_xpub, master_fingerprint, network)
                    .await
            })
            .await
        })
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(sign_psbt_with_client(
            &client,
            psbt,
            account_xpub,
            master_fingerprint,
            network,
        ))
    }
}

/// Ledger descriptor template for a single-sig address script type. These are
/// **standard** wallet policies (`tr`/`wpkh`) that `get_wallet_address` accepts with a
/// zero HMAC — no `register_wallet` round trip is needed.
fn address_template(script: AddressScriptType) -> &'static str {
    match script {
        AddressScriptType::Taproot => "tr(@0/**)",
        AddressScriptType::WitnessPubkeyHash => "wpkh(@0/**)",
    }
}

/// Splits a full verify path (e.g. `m/86'/1'/73'/0/3`) into the hardened account prefix
/// (`m/86'/1'/73'`), its origin string (`86'/1'/73'`), and the `change`/`address_index`
/// from the trailing 2-level unhardened suffix. Mirrors the split in [`get_info_with`].
fn split_verify_path(path: &DerivationPath) -> Result<(DerivationPath, String, bool, u32), String> {
    let steps: Vec<ChildNumber> = path.into_iter().copied().collect();
    let split = steps
        .iter()
        .rposition(|c| c.is_hardened())
        .map(|i| i + 1)
        .unwrap_or(0);
    let suffix = &steps[split..];
    if suffix.len() != 2 {
        return Err(format!(
            "Ledger verify-address requires a path ending in change/index (e.g. .../0/0), got {path}"
        ));
    }
    let normal = |c: &ChildNumber| -> Result<u32, String> {
        match c {
            ChildNumber::Normal { index } => Ok(*index),
            ChildNumber::Hardened { .. } => Err(format!(
                "Ledger verify-address change/index must be non-hardened, got {path}"
            )),
        }
    };
    let change_val = normal(&suffix[0])?;
    if change_val > 1 {
        return Err(format!(
            "Ledger verify-address change must be 0 (receive) or 1 (change), got {change_val}"
        ));
    }
    let address_index = normal(&suffix[1])?;
    let account_steps = &steps[..split];
    let account_path = DerivationPath::from(account_steps.to_vec());
    let account_origin = account_steps
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join("/");
    Ok((account_path, account_origin, change_val == 1, address_index))
}

/// Builds a standard single-sig Ledger wallet policy from an account xpub, its origin
/// path, and a descriptor template — used by verify-on-device for both `tr` and `wpkh`.
fn build_single_sig_policy(
    account_xpub: &str,
    account_origin: &str,
    master_fingerprint: u32,
    template: &str,
) -> Result<ledger_bitcoin_client::wallet::WalletPolicy, String> {
    use ledger_bitcoin_client::wallet::{Version, WalletPolicy, WalletPubKey};
    use std::str::FromStr;

    let fingerprint = bitcoin::bip32::Fingerprint::from(master_fingerprint.to_le_bytes());
    let key_str = format!("[{fingerprint}/{account_origin}]{account_xpub}");
    let key = WalletPubKey::from_str(&key_str)
        .map_err(|e| format!("invalid wallet pubkey for Ledger verify policy: {e}"))?;
    Ok(WalletPolicy::new(
        LEDGER_ADMIN_WALLET_POLICY_NAME.to_string(),
        Version::V2,
        template.to_string(),
        vec![key],
    ))
}

/// Renders the address at `derivation_path` on the Ledger screen via `get_wallet_address`
/// (`display = true`) using a standard single-sig policy for `script`, so the signer
/// compares an actual **address** (not the extended public key).
async fn verify_address_with<T>(
    client: &BitcoinClient<T>,
    derivation_path: &str,
    script: AddressScriptType,
) -> Result<String, String>
where
    T: async_client::Transport + Sync,
    T::Error: std::fmt::Debug,
{
    let path = parse_path(derivation_path)?;
    let (account_path, account_origin, change, address_index) = split_verify_path(&path)?;

    let account_xpub = client
        .get_extended_pubkey(&account_path, false)
        .await
        .map_err(|e| map_ledger_error("get_extended_pubkey", &format!("{e:?}")))?
        .to_string();
    let fp = client
        .get_master_fingerprint()
        .await
        .map_err(|e| map_ledger_error("get_master_fingerprint", &format!("{e:?}")))?;
    let master_fingerprint = u32::from_le_bytes(fp.to_bytes());

    let policy = build_single_sig_policy(
        &account_xpub,
        &account_origin,
        master_fingerprint,
        address_template(script),
    )?;

    client
        .get_wallet_address(&policy, None, change, address_index, true)
        .await
        // Return the exact string the device rendered so the caller can compare it
        // against what the app shows (#412). The device already applied its own HRP.
        .map(|address| address.assume_checked().to_string())
        .map_err(|e| map_ledger_error("verify_address", &format!("{e:?}")))
}

/// Confirms the address at `derivation_path` on the Ledger screen.
///
/// `network` is accepted for dispatch parity with the Trezor adapter but is intentionally
/// unused: the device renders the address with the HRP of the path's coin type (its own
/// Bitcoin/Testnet app), which cannot be forced to regtest. The path itself encodes
/// BIP-86 vs BIP-84, and `script` selects the matching `tr`/`wpkh` policy template.
pub fn verify_address_on_device(
    derivation_path: String,
    script: AddressScriptType,
    _network: bitcoin::Network,
) -> Result<String, String> {
    with_ledger_device(|| verify_address_on_device_unlocked(&derivation_path, script))
}

fn verify_address_on_device_unlocked(
    derivation_path: &str,
    script: AddressScriptType,
) -> Result<String, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        rt.block_on(async {
            let client = bitcoin_client_for_speculos(&url)
                .await
                .map_err(|e| e.to_string())?;
            verify_address_with(&client, derivation_path, script).await
        })
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(verify_address_with(&client, derivation_path, script))
    }
}

pub fn get_master_fingerprint() -> Result<u32, String> {
    with_ledger_device(get_master_fingerprint_unlocked)
}

fn get_master_fingerprint_unlocked() -> Result<u32, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    if let Ok(url) = std::env::var("LEDGER_SPECULOS_URL") {
        rt.block_on(async {
            let client = bitcoin_client_for_speculos(&url)
                .await
                .map_err(|e| e.to_string())?;
            let fp = client
                .get_master_fingerprint()
                .await
                .map_err(|e| map_ledger_error("get_master_fingerprint", &format!("{e:?}")))?;
            Ok(u32::from_le_bytes(fp.to_bytes()))
        })
    } else {
        let hidapi = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| format!("Ledger not found or locked: {e}"))?;
        let client = BitcoinClient::new(HidTransport(transport));
        rt.block_on(async {
            let fp = client
                .get_master_fingerprint()
                .await
                .map_err(|e| map_ledger_error("get_master_fingerprint", &format!("{e:?}")))?;
            Ok(u32::from_le_bytes(fp.to_bytes()))
        })
    }
}

#[cfg(test)]
mod policy_name_tests {
    use super::{LEDGER_ADMIN_WALLET_POLICY_NAME, LEDGER_REGISTER_FALLBACK_POLICY_NAME};

    #[test]
    fn admin_wallet_policy_name_is_empty_for_sign_psbt() {
        assert!(LEDGER_ADMIN_WALLET_POLICY_NAME.is_empty());
        assert!(LEDGER_REGISTER_FALLBACK_POLICY_NAME.is_ascii());
        assert!(!LEDGER_REGISTER_FALLBACK_POLICY_NAME.is_empty());
    }
}

#[cfg(test)]
mod verify_helper_tests {
    use super::*;
    use bitcoin::address::KnownHrp;

    #[test]
    fn address_template_maps_script_to_descriptor() {
        assert_eq!(address_template(AddressScriptType::Taproot), "tr(@0/**)");
        assert_eq!(
            address_template(AddressScriptType::WitnessPubkeyHash),
            "wpkh(@0/**)"
        );
    }

    #[test]
    fn split_verify_path_extracts_account_origin_change_index() {
        let path = parse_path("m/86'/1'/73'/0/3").unwrap();
        let (_account_path, origin, change, index) = split_verify_path(&path).unwrap();
        assert_eq!(origin, "86'/1'/73'");
        assert!(!change);
        assert_eq!(index, 3);

        let change_path = parse_path("m/84'/0'/73'/1/0").unwrap();
        let (_, origin, change, index) = split_verify_path(&change_path).unwrap();
        assert_eq!(origin, "84'/0'/73'");
        assert!(change);
        assert_eq!(index, 0);
    }

    #[test]
    fn split_verify_path_rejects_wrong_suffix_length() {
        // Account path with no change/index suffix.
        assert!(split_verify_path(&parse_path("m/86'/1'/73'").unwrap()).is_err());
        // Three trailing unhardened levels.
        assert!(split_verify_path(&parse_path("m/86'/1'/73'/0/0/0").unwrap()).is_err());
    }

    #[test]
    fn split_verify_path_rejects_out_of_range_change() {
        assert!(split_verify_path(&parse_path("m/86'/1'/73'/2/0").unwrap()).is_err());
    }

    #[test]
    fn hrp_from_path_follows_coin_type() {
        assert_eq!(
            hrp_from_path(&parse_path("m/84'/1'/73'/0/0").unwrap()),
            KnownHrp::Testnets
        );
        assert_eq!(
            hrp_from_path(&parse_path("m/84'/0'/73'/0/0").unwrap()),
            KnownHrp::Mainnet
        );
    }
}
