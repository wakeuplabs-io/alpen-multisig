use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

use bitcoin::address::KnownHrp;
use bitcoin::bip32::{ChildNumber, DerivationPath, Fingerprint, Xpub};
use bitcoin::Network;
use trezor_client::{protos, utils, InputScriptType, Trezor, TrezorMessage, TrezorResponse};

use super::{AddressScriptType, HwWalletInfo};
use crate::infrastructure::signing::SignatureResult;

/// BIP-84 path for Admin ID (P2WPKH message signing, non-Payout-Admin multisigs).
const ADMIN_ID_PATH: &str = "m/84'/0'/73'/0/0";

/// Which wallet behind the seed the signer asked for.
///
/// One Trezor seed backs unlimited wallets: the standard one, plus a distinct wallet per
/// passphrase. The passphrase is a per-session parameter rather than device state, so the
/// wallet is chosen on every connection by how the host answers `PassphraseRequest` —
/// there is no "current wallet" the device remembers between sessions.
///
/// Neither answer sends a secret from this machine: [`Standard`](Self::Standard) sends an
/// empty string, which is the absence of one, and [`Hidden`](Self::Hidden) hands entry to
/// the device keypad. Verified on the emulator in `issues/evidence/G5-B0-PROTOCOL.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WalletKind {
    /// The wallet derived from the seed alone.
    #[default]
    Standard,
    /// A wallet derived from the seed plus a passphrase typed on the device.
    Hidden,
}

/// The device session, and which wallet it belongs to.
///
/// Every Trezor operation opens its own transport (`open_trezor`), so without the session id
/// the device would treat each one as a fresh session and re-derive the seed — which means
/// re-prompting for the passphrase on the device keypad on *every* call. Resuming the
/// session keeps the firmware's cached seed (`APP_COMMON_SEED`) alive, so the signer enters
/// the passphrase once per connection.
///
/// The kind is stored *with* the id rather than beside it because they are one fact — which
/// wallet we are talking to — and they must never disagree. If the firmware evicts our
/// session mid-flow ([`SessionOutcome::Lost`]), the next `PassphraseRequest` has to receive
/// the same answer as the first, or a later operation would silently run against the other
/// wallet.
#[derive(Debug, Default, Clone)]
struct SessionState {
    /// The id from the last successful `Initialize`, if the device reported a usable one.
    id: Option<Vec<u8>>,
    /// The wallet the signer chose on the connection this session belongs to.
    kind: WalletKind,
}

/// A process-wide `static` rather than Tauri managed state, following the precedent of
/// `LEDGER_DEVICE_LOCK` in `ledger.rs`: managed state would have to be threaded through
/// six commands and would break their `spawn_blocking(move || ...)` bodies. It also has to
/// reach `sign_taproot_psbt`, which runs behind a `DeviceSignFn` with no parameter path
/// back to the UI.
static TREZOR_SESSION: OnceLock<Mutex<SessionState>> = OnceLock::new();

/// `trezor_client::unique()` claims the USB device, so two overlapping operations fight over
/// it: the loser reports a confusing "init failed on both transport modes" and, worse, both
/// would race to write [`TREZOR_SESSION`], leaving one session orphaned and the signer facing
/// an extra passphrase prompt. Mirrors `LEDGER_DEVICE_LOCK` in `ledger.rs`.
static TREZOR_DEVICE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// An open device, held for as long as the operation using it.
///
/// The lock guard travels with the device rather than wrapping each entry point, so an
/// operation cannot lose exclusivity halfway through a multi-message exchange like `SignTx`.
/// Derefs to [`Trezor`], so call sites use it as if it were the device itself.
struct TrezorDevice {
    trezor: Trezor,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl std::ops::Deref for TrezorDevice {
    type Target = Trezor;

    fn deref(&self) -> &Trezor {
        &self.trezor
    }
}

impl std::ops::DerefMut for TrezorDevice {
    fn deref_mut(&mut self) -> &mut Trezor {
        &mut self.trezor
    }
}

/// The session slot. A poisoned lock is recovered rather than swallowed: dropping the write
/// silently would leave a stale session id in place, and `start_session` reporting success
/// without forgetting anything is the one outcome that must not happen.
fn session_store() -> &'static Mutex<SessionState> {
    TREZOR_SESSION.get_or_init(|| Mutex::new(SessionState::default()))
}

fn session_slot() -> std::sync::MutexGuard<'static, SessionState> {
    session_store().lock().unwrap_or_else(|e| e.into_inner())
}

/// What the device did with the session we asked it to resume.
///
/// The firmware never *fails* an `Initialize` carrying an unknown session id: it silently
/// starts a fresh, empty session and returns a different id (`cache_codec.py:91-128`). The
/// only way to tell the two apart is to compare the id it returned against the one we sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionOutcome {
    /// We asked for nothing; this is a new session.
    Started,
    /// The device resumed the session we asked for — its seed cache is still warm.
    Resumed,
    /// We asked to resume and got a *different* session back. The passphrase cache is
    /// empty, so the device will prompt again before it derives any key.
    Lost,
}

/// Pure classification of an `Initialize` round-trip. Split out from `open_trezor` so the
/// silent-downgrade rule is testable without a device.
fn session_outcome(requested: Option<&[u8]>, returned: &[u8]) -> SessionOutcome {
    match requested {
        None => SessionOutcome::Started,
        Some(asked) if asked == returned => SessionOutcome::Resumed,
        Some(_) => SessionOutcome::Lost,
    }
}

/// Length of a firmware session id (`_SESSION_ID_LENGTH` in the Trezor firmware).
const SESSION_ID_LEN: usize = 32;

/// The session id the device reported, if it sent a usable one. Anything that is not exactly
/// 32 bytes is treated as absent: the firmware answers an off-length id the same way it
/// answers a missing one — with a fresh, empty session — so storing it would silently cost a
/// passphrase prompt on every single operation.
fn reported_session_id(trezor: &Trezor) -> Option<Vec<u8>> {
    let id = trezor.features()?.session_id();
    (id.len() == SESSION_ID_LEN).then(|| id.to_vec())
}

/// Begins a connection for `kind`: drops the remembered session so the next operation starts
/// a clean one and the device asks for the passphrase again, and records which wallet every
/// `PassphraseRequest` from here on should be answered for.
///
/// Both writes happen under one lock acquisition. Split into two, a connect could publish the
/// new kind while the previous session id was still readable, and an operation racing between
/// them would resume the old wallet's session under the new wallet's answer.
fn start_session(kind: WalletKind) {
    *session_slot() = SessionState { id: None, kind };
}

/// The wallet the current connection is for. Defaults to [`WalletKind::Standard`], so an
/// operation arriving before any connect gets the wallet that needs no secret.
fn current_wallet_kind() -> WalletKind {
    session_slot().kind
}

/// Records the session the device just handed back.
///
/// `Lost` is not an error: the signer simply re-enters the passphrase on the device before
/// the next key is derived, so there is no way to end up on the standard wallet without
/// the device saying so. It is worth logging because a session that keeps getting lost
/// means the passphrase prompt keeps coming back, which reads as a bug from the signer's
/// side. (`_MAX_SESSIONS_COUNT = 10` in the firmware, so another client can evict ours.)
fn remember_session(requested: Option<&[u8]>, trezor: &Trezor) {
    let Some(returned) = reported_session_id(trezor) else {
        return;
    };
    if session_outcome(requested, &returned) == SessionOutcome::Lost {
        eprintln!("trezor: device session was not resumed; the passphrase will be requested again");
    }
    // Only the id changes: the wallet kind belongs to the connection, and a re-derivation
    // after a lost session must be answered for the same wallet the signer chose.
    session_slot().id = Some(returned);
}

fn open_trezor() -> Result<TrezorDevice, String> {
    let guard = TREZOR_DEVICE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let mut attempts = Vec::with_capacity(2);
    let mut saw_invalid_protocol = false;
    let requested = session_slot().id.clone();

    for debug in [false, true] {
        let mut trezor = match trezor_client::unique(debug) {
            Ok(device) => device,
            Err(e) => {
                attempts.push(format!("debug={debug}: discovery failed ({e})"));
                continue;
            }
        };

        match trezor.init_device(requested.clone()) {
            Ok(_) => {
                remember_session(requested.as_deref(), &trezor);
                return Ok(TrezorDevice {
                    trezor,
                    _guard: guard,
                });
            }
            Err(e) => {
                if e.to_string().contains("Failure_InvalidProtocol") {
                    saw_invalid_protocol = true;
                }
                attempts.push(format!("debug={debug}: init failed ({e})"));
            }
        }
    }

    // The remembered session is deliberately kept. Failing to reach the device says nothing
    // about whether its session is still valid, and a stale id costs nothing: the firmware
    // answers it with a fresh session, which `session_outcome` reports as `Lost`. Dropping it
    // here would mean a transient probe failure re-prompts the signer for the passphrase.
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
    req.set_coin_name(
        utils::coin_name(trezor_coin_network(network)).map_err(|e| format!("coin_name: {e}"))?,
    );
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
    req.set_coin_name(
        utils::coin_name(trezor_coin_network(network)).map_err(|e| format!("coin_name: {e}"))?,
    );
    req.set_script_type(script_type);
    req.set_ignore_xpub_magic(true);
    trezor
        .call(
            req,
            Box::new(|_, m: protos::PublicKey| Ok(m.xpub().parse()?)),
        )
        .map_err(|e: trezor_client::Error| e.to_string())
}

/// Turns a device `Failure` into something a signer can act on.
///
/// The one case worth naming is a device with no keypad for the passphrase: it rejects
/// on-device entry outright, and the raw protobuf dump says nothing about what to do. This
/// app never sends a passphrase from the host, so such a device cannot open a hidden wallet
/// here at all — the signer needs to be told that, not handed `Failure { code: ... }`.
fn describe_failure(failure: &protos::Failure) -> String {
    if failure.message().contains("incapable of passphrase entry") {
        return "This Trezor cannot take a passphrase on its own keypad, and Strata Multisig \
never asks for one on this computer. Disable the passphrase on the device to use its standard \
wallet, or connect a Trezor model with on-device passphrase entry."
            .to_string();
    }
    format!("Device failure: {:?}", failure)
}

/// Drive a TrezorResponse to completion, handling ButtonRequests and PassphraseRequests.
///
/// A `PassphraseRequest` is answered for the wallet the signer chose on this connection
/// ([`current_wallet_kind`]), and **neither answer carries a secret typed on this machine**:
///
/// - [`WalletKind::Standard`] → `ack_passphrase("")`. The empty string is the absence of a
///   passphrase, and it is what opens the wallet derived from the seed alone. It is sent
///   explicitly rather than as `ack(false)`, which sets no field at all.
/// - [`WalletKind::Hidden`] → `ack(true)`, which asks the firmware to prompt on its own
///   screen. It must not carry a passphrase alongside it or the device answers `DataError`.
///
/// So a keylogger on this machine has nothing to capture in either branch, while the signer
/// still chooses which wallet to open — the two answers produce two different wallets from
/// one seed (measured in `issues/evidence/G5-B0-PROTOCOL.md`).
///
/// This deliberately does not consult `PassphraseRequest::on_device()`: that field was
/// deprecated in firmware 2.3.0 (`reserved 1` in messages-common.proto) and modern devices
/// never set it, so branching on it would silently mean "always ask the host".
///
/// Devices without a keypad (Trezor One / T1B1) reject `on_device` with `Failure_DataError`,
/// which [`describe_failure`] turns into an actionable message.
fn resolve<'a, T, R: TrezorMessage>(mut response: TrezorResponse<'a, T, R>) -> Result<T, String> {
    loop {
        match response {
            TrezorResponse::Ok(data) => return Ok(data),
            TrezorResponse::Failure(f) => return Err(describe_failure(&f)),
            TrezorResponse::ButtonRequest(req) => {
                response = req.ack().map_err(|e| format!("ButtonAck failed: {e}"))?;
            }
            TrezorResponse::PinMatrixRequest(_) => {
                return Err("PIN entry not supported in this build.".to_string());
            }
            TrezorResponse::PassphraseRequest(req) => {
                response = match current_wallet_kind() {
                    WalletKind::Standard => req.ack_passphrase(String::new()),
                    WalletKind::Hidden => req.ack(true),
                }
                .map_err(|e| format!("PassphraseAck failed: {e}"))?;
            }
        }
    }
}

/// The message shown when a hidden wallet is asked for on a device that has the passphrase
/// switched off. Without this the connection would succeed against the *standard* wallet and
/// report success: the firmware simply never emits `PassphraseRequest`, so nothing the host
/// sends can produce a prompt (case 4 in `issues/evidence/G5-B0-PROTOCOL.md`).
const PASSPHRASE_DISABLED_ON_DEVICE: &str =
    "This Trezor has the passphrase switched off, so it cannot open a hidden wallet. Enable \
Passphrase in the device's own settings and connect again, or use the standard wallet.";

/// Connect: read the P2WPKH Admin ID address at the BIP-84 derivation path.
///
/// `kind` selects which wallet behind the seed to open, and holds for every later operation
/// on this connection — see [`SessionState`].
pub fn connect(derivation_path: Option<String>, kind: WalletKind) -> Result<HwWalletInfo, String> {
    let path_str = derivation_path.unwrap_or_else(|| ADMIN_ID_PATH.to_string());
    let path = parse_path(&path_str)?;

    // A connection starts its own device session, so the signer is asked for the passphrase
    // once per connect and a session from an earlier one is never inherited. Doing it here
    // rather than on disconnect keeps it ordered: disconnect is fire-and-forget from the UI
    // and could otherwise land *after* the next connect and wipe the session it just made.
    start_session(kind);

    let mut trezor = open_trezor()?;

    // Refuse a hidden wallet the device cannot give us, rather than silently handing back the
    // standard one. Checked against `has_passphrase_protection` too: the firmware omits the
    // field entirely while the device is locked, and absent must not read as "switched off".
    if kind == WalletKind::Hidden {
        let features = trezor
            .features()
            .ok_or("Trezor returned no device features.".to_string())?;
        if features.has_passphrase_protection() && !features.passphrase_protection() {
            return Err(PASSPHRASE_DISABLED_ON_DEVICE.to_string());
        }
    }

    let xpub = resolve(
        get_xpub(
            &mut trezor,
            &path,
            InputScriptType::SPENDWITNESS,
            Network::Bitcoin,
            false,
        )
        .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
    )?;

    let pubkey_hex = hex::encode(xpub.public_key.serialize());
    let compressed = bitcoin::CompressedPublicKey(xpub.public_key);
    let address = bitcoin::Address::p2wpkh(&compressed, KnownHrp::Mainnet);

    Ok(HwWalletInfo {
        device_label: "Trezor".to_string(),
        derivation_path: path_str,
        address_sample: Some(address.to_string()),
        public_key_hex: Some(pubkey_hex.clone()),
        xpub_or_fingerprint: Some(format!("{}…", &pubkey_hex[..16.min(pubkey_hex.len())])),
        key_label: Some("Public key".to_string()),
    })
}

/// Trezor `InputScriptType` for an address script type — taproot (BIP-86) vs
/// native witness pubkey hash (BIP-84). Pure mapping; no device contact.
fn input_script_type(script: AddressScriptType) -> InputScriptType {
    match script {
        AddressScriptType::Taproot => InputScriptType::SPENDTAPROOT,
        AddressScriptType::WitnessPubkeyHash => InputScriptType::SPENDWITNESS,
    }
}

/// Confirms the address at `derivation_path` on the Trezor screen, using the
/// script type (P2TR receive / P2WPKH Admin ID) and network the session runs on.
///
/// Uses `GetAddress` with `show_display = true` so the device renders the actual
/// address for the signer to compare, instead of just the public key.
pub fn verify_address_on_device(
    derivation_path: String,
    script: AddressScriptType,
    network: Network,
) -> Result<String, String> {
    let path = parse_path(&derivation_path)?;
    let mut trezor = open_trezor()?;
    let coin =
        utils::coin_name(trezor_coin_network(network)).map_err(|e| format!("coin_name: {e}"))?;

    let mut req = protos::GetAddress::new();
    req.address_n = utils::convert_path(&path);
    req.set_coin_name(coin);
    req.set_show_display(true);
    req.set_script_type(input_script_type(script));
    let resp = trezor
        .call(
            req,
            Box::new(|_, m: protos::Address| Ok(m.address().to_string())),
        )
        .map_err(|e: trezor_client::Error| {
            format!("Trezor verify_address at {derivation_path} failed: {e}")
        })?;
    // The device confirmed: return the exact address it rendered so the caller can
    // compare it against what the app shows (#412).
    resolve(resp)
}

/// Returns the BIP-86 (Taproot) account xpub for the given derivation path.
///
/// Uses `SPENDTAPROOT` script type so Trezor derives the correct key material
/// for a P2TR wallet. `ignore_xpub_magic = true` (set inside `get_xpub`) ensures
/// standard `xpub` version bytes are returned instead of SLIP-0132 `Xpub` bytes.
pub fn get_account_xpub(path: &str, network: Network) -> Result<String, String> {
    let derivation_path = parse_path(path)?;
    let mut trezor = open_trezor()?;
    let xpub = resolve(get_xpub(
        &mut trezor,
        &derivation_path,
        InputScriptType::SPENDTAPROOT,
        trezor_coin_network(network),
        false,
    )?)?;
    Ok(xpub.to_string())
}

/// Returns the master fingerprint (first 4 bytes of hash160 of master public key) from the Trezor.
/// Obtained by requesting the master xpub at path `m/` and reading the root_fingerprint from the response.
pub fn get_master_fingerprint() -> Result<u32, String> {
    let mut trezor = open_trezor()?;
    let master_path = DerivationPath::from_str("m/").map_err(|e| format!("Invalid path: {e}"))?;

    // Call GetPublicKey on master path to get root_fingerprint from the response
    let mut req = protos::GetPublicKey::new();
    req.address_n = utils::convert_path(&master_path);
    req.set_show_display(false);
    req.set_coin_name(utils::coin_name(Network::Bitcoin).map_err(|e| format!("coin_name: {e}"))?);
    req.set_script_type(InputScriptType::SPENDTAPROOT);
    req.set_ignore_xpub_magic(true);

    let response = trezor
        .call(
            req,
            Box::new(|_, m: protos::PublicKey| {
                // root_fingerprint is available on the PublicKey response
                Ok(m.root_fingerprint())
            }),
        )
        .map_err(|e: trezor_client::Error| e.to_string())?;

    resolve(response)
}

/// Signs an arbitrary human-readable message on Trezor using Bitcoin `signMessage`.
///
/// Three callers pass three different strings — the SPS-65 signing message, the session
/// authentication challenge, and the Admin ID Verification Certificate message — and the
/// device treats all of them the same way: it computes `Hash256(prefix || message)`
/// internally, which matches `compute_sighash()` for the SPS-65 case exactly.
///
/// Uses `SPENDWITNESS` (BIP-84 P2WPKH) which is required for `m/84'` Admin ID paths.
pub fn sign_bitcoin_message(
    message: &str,
    derivation_path: &str,
) -> Result<SignatureResult, String> {
    let path = parse_path(derivation_path)?;
    let mut trezor = open_trezor()?;

    let xpub: Xpub = resolve(
        get_xpub(
            &mut trezor,
            &path,
            InputScriptType::SPENDWITNESS,
            Network::Bitcoin,
            false,
        )
        .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
    )?;

    let recoverable_sig = resolve(
        sign_message_recoverable(
            &mut trezor,
            message,
            &path,
            InputScriptType::SPENDWITNESS,
            Network::Bitcoin,
        )
        .map_err(|e| format!("Trezor sign_message failed: {e}"))?,
    )?;

    Ok(SignatureResult {
        public_key_hex: hex::encode(xpub.public_key.serialize()),
        signature_hex: hex::encode(recoverable_sig),
    })
}

/// Reads the device root fingerprint from the currently open session (xpub at `m/`),
/// so we can verify the connected Trezor matches the session before signing.
fn read_root_fingerprint(trezor: &mut Trezor) -> Result<u32, String> {
    let master = DerivationPath::from_str("m/").map_err(|e| format!("invalid path: {e}"))?;
    let mut req = protos::GetPublicKey::new();
    req.address_n = utils::convert_path(&master);
    req.set_show_display(false);
    req.set_coin_name(utils::coin_name(Network::Bitcoin).map_err(|e| format!("coin_name: {e}"))?);
    req.set_script_type(InputScriptType::SPENDTAPROOT);
    req.set_ignore_xpub_magic(true);
    let resp = trezor
        .call(
            req,
            Box::new(|_, m: protos::PublicKey| Ok(m.root_fingerprint())),
        )
        .map_err(|e: trezor_client::Error| e.to_string())?;
    resolve(resp)
}

/// Full BIP-32 derivation path (as Trezor `address_n`) for the wallet-owned key in
/// a taproot PSBT input/output, matched by the device fingerprint. `None` for
/// outputs that are not wallet-owned (e.g. the send recipient).
fn wallet_address_n(
    origins: &std::collections::BTreeMap<
        bitcoin::secp256k1::XOnlyPublicKey,
        (
            Vec<bitcoin::taproot::TapLeafHash>,
            (Fingerprint, DerivationPath),
        ),
    >,
    expected_fp: Fingerprint,
) -> Option<Vec<u32>> {
    origins
        .values()
        .find(|(_, (fp, _))| *fp == expected_fp)
        .map(|(_, (_, path))| utils::convert_path(path))
}

/// Builds the `TxAck` for a TXINPUT request, marking the wallet input as a taproot
/// key-path spend (`SPENDTAPROOT`) with its full derivation path and witness amount.
fn ack_input(
    req: &protos::TxRequest,
    psbt: &bitcoin::psbt::Psbt,
    expected_fp: Fingerprint,
) -> Result<protos::TxAck, String> {
    if req.details.has_tx_hash() {
        return Err("Trezor requested a previous tx; a taproot spend needs none".to_string());
    }
    let idx = req.details.request_index() as usize;
    let txin = psbt
        .unsigned_tx
        .input
        .get(idx)
        .ok_or_else(|| format!("TxRequest input index {idx} out of range"))?;
    let psbt_input = psbt
        .inputs
        .get(idx)
        .ok_or_else(|| format!("PSBT input {idx} missing"))?;
    let txout = psbt_input
        .witness_utxo
        .as_ref()
        .ok_or_else(|| format!("PSBT input {idx} has no witness_utxo"))?;
    let address_n = wallet_address_n(&psbt_input.tap_key_origins, expected_fp)
        .ok_or_else(|| format!("PSBT input {idx} has no taproot key origin for the device"))?;

    let mut data_input = protos::tx_ack::transaction_type::TxInputType::new();
    data_input.set_prev_hash(utils::to_rev_bytes(txin.previous_output.txid.as_raw_hash()).to_vec());
    data_input.set_prev_index(txin.previous_output.vout);
    data_input.set_sequence(txin.sequence.to_consensus_u32());
    data_input.set_script_type(InputScriptType::SPENDTAPROOT);
    data_input.set_amount(txout.value.to_sat());
    data_input.address_n = address_n;

    let mut msg = protos::TxAck::new();
    msg.tx.mut_or_insert_default().inputs.push(data_input);
    Ok(msg)
}

/// Builds the `TxAck` for a TXOUTPUT request: wallet-owned outputs (change) are
/// `PAYTOTAPROOT` with their derivation path; all others pay to the literal address.
fn ack_output(
    req: &protos::TxRequest,
    psbt: &bitcoin::psbt::Psbt,
    network: Network,
    expected_fp: Fingerprint,
) -> Result<protos::TxAck, String> {
    if req.details.has_tx_hash() {
        return Err(
            "Trezor requested a previous tx output; a taproot spend needs none".to_string(),
        );
    }
    let idx = req.details.request_index() as usize;
    let txout = psbt
        .unsigned_tx
        .output
        .get(idx)
        .ok_or_else(|| format!("TxRequest output index {idx} out of range"))?;
    let psbt_output = psbt
        .outputs
        .get(idx)
        .ok_or_else(|| format!("PSBT output {idx} missing"))?;

    use protos::OutputScriptType;
    let mut data_output = protos::tx_ack::transaction_type::TxOutputType::new();
    data_output.set_amount(txout.value.to_sat());
    match wallet_address_n(&psbt_output.tap_key_origins, expected_fp) {
        Some(address_n) => {
            data_output.address_n = address_n;
            data_output.set_script_type(OutputScriptType::PAYTOTAPROOT);
        }
        None => {
            let address = utils::address_from_script(&txout.script_pubkey, network)
                .ok_or_else(|| format!("output {idx} script is not a standard address"))?;
            data_output.set_address(address.to_string());
            data_output.set_script_type(OutputScriptType::PAYTOADDRESS);
        }
    }

    let mut msg = protos::TxAck::new();
    msg.tx.mut_or_insert_default().outputs.push(data_output);
    Ok(msg)
}

/// Builds the `TxAck` for a TXMETA request of the tx being signed.
fn ack_meta(psbt: &bitcoin::psbt::Psbt) -> protos::TxAck {
    let tx = &psbt.unsigned_tx;
    let mut msg = protos::TxAck::new();
    let meta = msg.tx.mut_or_insert_default();
    meta.set_version(tx.version.0 as u32);
    meta.set_lock_time(tx.lock_time.to_consensus_u32());
    meta.set_inputs_cnt(tx.input.len() as u32);
    meta.set_outputs_cnt(tx.output.len() as u32);
    msg
}

/// Applies the device-returned 64-byte Schnorr signatures onto the matching PSBT
/// inputs as taproot key-path signatures (SIGHASH_DEFAULT).
fn apply_taproot_signatures(
    psbt: &mut bitcoin::psbt::Psbt,
    signatures: &[(usize, Vec<u8>)],
) -> Result<(), String> {
    for (idx, sig) in signatures {
        let input = psbt
            .inputs
            .get_mut(*idx)
            .ok_or_else(|| format!("device returned a signature for unknown input {idx}"))?;
        let schnorr = bitcoin::secp256k1::schnorr::Signature::from_slice(sig)
            .map_err(|e| format!("invalid taproot signature for input {idx}: {e}"))?;
        input.tap_key_sig = Some(bitcoin::taproot::Signature {
            signature: schnorr,
            sighash_type: bitcoin::sighash::TapSighashType::Default,
        });
    }
    Ok(())
}

/// trezor-client's `coin_name()` only knows Bitcoin and Testnet.
/// Regtest and Signet are Testnet-compatible for signing purposes.
fn trezor_coin_network(network: Network) -> Network {
    match network {
        Network::Bitcoin => Network::Bitcoin,
        _ => Network::Testnet,
    }
}

/// Trezor validates every BIP-86 path's coin type against the SLIP-44 id of the coin it
/// signs under: a `Testnet` coin requires coin type `1'`, a `Bitcoin` coin requires `0'`.
/// A mismatch is rejected on-device with `Forbidden key path` (`Failure_DataError`).
///
/// The Admin Wallet account path is device-specific (Trezor uses coin type `0'` on *every*
/// network — see `admin_wallet_account_path`), so the signing coin must follow the path's
/// coin type, not the session network, or the commit funding broadcast fails to sign.
fn trezor_coin_for_path(path: &DerivationPath) -> Option<Network> {
    match path.into_iter().nth(1) {
        Some(ChildNumber::Hardened { index: 0 }) => Some(Network::Bitcoin),
        Some(ChildNumber::Hardened { .. }) => Some(Network::Testnet),
        _ => None,
    }
}

/// Picks the Trezor coin to sign under from the wallet-owned input paths in `psbt`, keeping
/// the path↔coin invariant the device enforces. Falls back to the session network's coin when
/// no wallet origin is present (should not happen for an Admin Wallet PSBT).
fn trezor_signing_coin(
    psbt: &bitcoin::psbt::Psbt,
    expected_fp: Fingerprint,
    network: Network,
) -> Network {
    psbt.inputs
        .iter()
        .flat_map(|input| input.tap_key_origins.values())
        .find(|(_, (fp, _))| *fp == expected_fp)
        .and_then(|(_, (_, path))| trezor_coin_for_path(path))
        .unwrap_or_else(|| trezor_coin_network(network))
}

/// Drives the Trezor `SignTx` flow for a taproot key-path spend, collecting the
/// signatures and applying them to `psbt`. trezor-client 0.1.5's built-in flow
/// classifies P2TR inputs as `EXTERNAL` (won't sign), so we ack each request
/// ourselves with the correct taproot script types.
fn sign_taproot_psbt(
    trezor: &mut Trezor,
    psbt: &mut bitcoin::psbt::Psbt,
    expected_fp: Fingerprint,
    network: Network,
) -> Result<(), String> {
    use protos::tx_request::RequestType;

    // The coin must match the coin type embedded in the wallet path, not the session network,
    // or Trezor rejects the spend with "Forbidden key path" (see `trezor_signing_coin`).
    let coin_net = trezor_signing_coin(psbt, expected_fp, network);
    let mut signatures: Vec<(usize, Vec<u8>)> = Vec::new();
    let mut progress = resolve(
        trezor
            .sign_tx(psbt, coin_net)
            .map_err(|e| format!("Trezor sign_tx failed: {e}"))?,
    )?;

    loop {
        if let Some((index, sig)) = progress.get_signature() {
            signatures.push((index, sig.to_vec()));
        }
        if progress.finished() {
            break;
        }
        let ack = match progress.tx_request().request_type() {
            RequestType::TXINPUT => ack_input(progress.tx_request(), psbt, expected_fp)?,
            RequestType::TXOUTPUT => {
                ack_output(progress.tx_request(), psbt, coin_net, expected_fp)?
            }
            RequestType::TXMETA => ack_meta(psbt),
            other => {
                return Err(format!(
                    "unsupported Trezor TxRequest type {other:?} for a taproot spend"
                ))
            }
        };
        progress = resolve(
            progress
                .ack_msg(ack)
                .map_err(|e| format!("Trezor TxAck failed: {e}"))?,
        )?;
    }

    apply_taproot_signatures(psbt, &signatures)
}

/// Taproot key-path PSBT signing for the Admin Wallet (BIP-86, P2TR), mirroring the
/// Ledger adapter: verify the device fingerprint matches the session, sign every
/// wallet-owned input on device, and apply the signatures back onto the PSBT.
///
/// Must run on a **blocking thread** (`tokio::task::spawn_blocking`). `account_xpub`
/// is unused — Trezor derives keys from the per-input `address_n` it receives — but
/// kept in the signature for parity with the Ledger entry point.
pub fn sign_admin_wallet_psbt(
    psbt: &mut bitcoin::psbt::Psbt,
    _account_xpub: &str,
    master_fingerprint: u32,
    network: Network,
) -> Result<(), String> {
    let mut trezor = open_trezor()?;

    let actual = read_root_fingerprint(&mut trezor)?;
    if actual != master_fingerprint {
        return Err(format!(
            "wrong Trezor device: expected fingerprint 0x{master_fingerprint:08X}, got 0x{actual:08X}"
        ));
    }

    let expected_fp = Fingerprint::from(master_fingerprint.to_le_bytes());
    sign_taproot_psbt(&mut trezor, psbt, expected_fp, network)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default must be the wallet that needs no secret. An operation reaching `resolve`
    /// before any connect — or after a panic wiped the intent — has to answer for the
    /// standard wallet, never open a hidden one the signer did not ask for.
    #[test]
    fn a_session_with_no_connect_yet_is_for_the_standard_wallet() {
        assert_eq!(SessionState::default().kind, WalletKind::Standard);
        assert_eq!(WalletKind::default(), WalletKind::Standard);
    }

    /// Both halves of the session transition, in one test on purpose: they share the
    /// process-wide [`TREZOR_SESSION`], and `cargo test` runs test fns on parallel threads,
    /// so splitting them would let one clobber the other's state.
    #[test]
    fn a_connect_publishes_its_wallet_and_keeps_it_across_a_new_session_id() {
        // Connecting must publish the requested wallet and drop the previous session id in
        // the same step. A stale id alongside a new kind is the silent-wrong-wallet case: the
        // device would resume the old wallet's warm seed cache and never prompt, so the signer
        // would get the previous wallet while the app reported the new one.
        for kind in [WalletKind::Hidden, WalletKind::Standard] {
            session_slot().id = Some(vec![7u8; SESSION_ID_LEN]);

            start_session(kind);

            assert_eq!(current_wallet_kind(), kind);
            assert_eq!(
                session_slot().id,
                None,
                "a stale session id survived {kind:?}"
            );
        }

        // A session lost mid-flow makes the device prompt again, and that prompt has to be
        // answered for the wallet the signer chose — so storing the replacement id must leave
        // the kind alone.
        start_session(WalletKind::Hidden);
        session_slot().id = Some(vec![9u8; SESSION_ID_LEN]);
        assert_eq!(current_wallet_kind(), WalletKind::Hidden);

        // Leave the slot as a fresh standard session, so nothing later in this process
        // inherits a hidden-wallet intent from a test.
        start_session(WalletKind::Standard);
    }

    #[test]
    fn session_without_a_request_is_a_fresh_start() {
        assert_eq!(session_outcome(None, &[1, 2, 3]), SessionOutcome::Started);
    }

    #[test]
    fn session_is_resumed_only_when_the_device_echoes_the_same_id() {
        let id = [7u8; 32];
        assert_eq!(session_outcome(Some(&id), &id), SessionOutcome::Resumed);
    }

    /// The firmware answers an unknown session id with a *new* empty session instead of an
    /// error, so a resumed session and a silently restarted one look identical apart from
    /// the id. Comparing the ids is the only signal there is.
    #[test]
    fn a_different_id_means_the_session_was_lost_not_resumed() {
        let asked = [7u8; 32];
        let got = [9u8; 32];
        assert_eq!(session_outcome(Some(&asked), &got), SessionOutcome::Lost);
        // A truncated echo is not a resume either.
        assert_eq!(
            session_outcome(Some(&asked), &asked[..16]),
            SessionOutcome::Lost
        );
    }

    #[test]
    fn trezor_coin_network_maps_regtest_and_signet_to_testnet() {
        assert_eq!(trezor_coin_network(Network::Bitcoin), Network::Bitcoin);
        assert_eq!(trezor_coin_network(Network::Testnet), Network::Testnet);
        assert_eq!(trezor_coin_network(Network::Regtest), Network::Testnet);
        assert_eq!(trezor_coin_network(Network::Signet), Network::Testnet);
    }

    #[test]
    fn trezor_coin_for_path_follows_account_coin_type() {
        // Coin type 0' (Trezor Admin Wallet on every network) must sign under the Bitcoin coin;
        // coin type 1' (Ledger-style test nets) under Testnet. Mismatching the path's coin type
        // is what triggers the on-device "Forbidden key path" rejection on the broadcast.
        assert_eq!(
            trezor_coin_for_path(&parse_path("m/86'/0'/73'/0/0").unwrap()),
            Some(Network::Bitcoin)
        );
        assert_eq!(
            trezor_coin_for_path(&parse_path("m/86'/1'/73'/0/0").unwrap()),
            Some(Network::Testnet)
        );
        // No coin-type level → no opinion, caller falls back to the session network.
        assert_eq!(trezor_coin_for_path(&parse_path("m/86'").unwrap()), None);
    }

    #[test]
    fn input_script_type_maps_taproot_and_witness() {
        assert_eq!(
            input_script_type(AddressScriptType::Taproot),
            InputScriptType::SPENDTAPROOT
        );
        assert_eq!(
            input_script_type(AddressScriptType::WitnessPubkeyHash),
            InputScriptType::SPENDWITNESS
        );
    }
}
