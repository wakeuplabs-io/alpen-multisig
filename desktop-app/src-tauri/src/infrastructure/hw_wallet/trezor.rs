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

/// The device session id from the last successful `Initialize`.
///
/// Every Trezor operation opens its own transport (`open_trezor`), so without this the
/// device would treat each one as a fresh session and re-derive the seed — which means
/// re-prompting for the passphrase on the device keypad on *every* call. Resuming the
/// session keeps the firmware's cached seed (`APP_COMMON_SEED`) alive, so the signer
/// enters the passphrase once per connection.
///
/// A process-wide `static` rather than Tauri managed state, following the precedent of
/// `LEDGER_DEVICE_LOCK` in `ledger.rs`: managed state would have to be threaded through
/// six commands and would break their `spawn_blocking(move || ...)` bodies.
static TREZOR_SESSION: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();

fn session_store() -> &'static Mutex<Option<Vec<u8>>> {
    TREZOR_SESSION.get_or_init(|| Mutex::new(None))
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

/// The session id the device reported, if it sent one. Empty ids are treated as absent:
/// the firmware uses 32-byte ids, and an empty value carries no session to resume.
fn reported_session_id(trezor: &Trezor) -> Option<Vec<u8>> {
    let id = trezor.features()?.session_id();
    (!id.is_empty()).then(|| id.to_vec())
}

/// Drops the remembered session, so the next operation starts a clean one and the device
/// asks for the passphrase again. Call this when the signer disconnects the wallet.
pub fn forget_session() {
    if let Ok(mut slot) = session_store().lock() {
        *slot = None;
    }
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
    if let Ok(mut slot) = session_store().lock() {
        *slot = Some(returned);
    }
}

fn open_trezor() -> Result<Trezor, String> {
    let mut attempts = Vec::with_capacity(2);
    let mut saw_invalid_protocol = false;
    let requested = session_store().lock().ok().and_then(|slot| slot.clone());

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
                return Ok(trezor);
            }
            Err(e) => {
                if e.to_string().contains("Failure_InvalidProtocol") {
                    saw_invalid_protocol = true;
                }
                attempts.push(format!("debug={debug}: init failed ({e})"));
            }
        }
    }

    // Both transports failed: the device is gone or unhealthy, so whatever session we were
    // holding is stale. Drop it rather than replaying it once the device comes back.
    forget_session();

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

/// Drive a TrezorResponse to completion, handling ButtonRequests and PassphraseRequests.
fn resolve<'a, T, R: TrezorMessage>(
    mut response: TrezorResponse<'a, T, R>,
    passphrase: &str,
) -> Result<T, String> {
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
            TrezorResponse::PassphraseRequest(req) => {
                response = if req.on_device() {
                    req.ack(true)
                } else {
                    req.ack_passphrase(passphrase.to_string())
                }
                .map_err(|e| format!("PassphraseAck failed: {e}"))?;
            }
        }
    }
}

/// Connect: read the P2WPKH Admin ID address at the BIP-84 derivation path.
pub fn connect(derivation_path: Option<String>, passphrase: &str) -> Result<HwWalletInfo, String> {
    let path_str = derivation_path.unwrap_or_else(|| ADMIN_ID_PATH.to_string());
    let path = parse_path(&path_str)?;

    let mut trezor = open_trezor()?;

    let xpub = resolve(
        get_xpub(
            &mut trezor,
            &path,
            InputScriptType::SPENDWITNESS,
            Network::Bitcoin,
            false,
        )
        .map_err(|e| format!("Trezor get_public_key failed: {e}"))?,
        passphrase,
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
    resolve(resp, "")
}

/// Returns the BIP-86 (Taproot) account xpub for the given derivation path.
///
/// Uses `SPENDTAPROOT` script type so Trezor derives the correct key material
/// for a P2TR wallet. `ignore_xpub_magic = true` (set inside `get_xpub`) ensures
/// standard `xpub` version bytes are returned instead of SLIP-0132 `Xpub` bytes.
pub fn get_account_xpub(path: &str, passphrase: &str, network: Network) -> Result<String, String> {
    let derivation_path = parse_path(path)?;
    let mut trezor = open_trezor()?;
    let xpub = resolve(
        get_xpub(
            &mut trezor,
            &derivation_path,
            InputScriptType::SPENDTAPROOT,
            trezor_coin_network(network),
            false,
        )?,
        passphrase,
    )?;
    Ok(xpub.to_string())
}

/// Returns the master fingerprint (first 4 bytes of hash160 of master public key) from the Trezor.
/// Obtained by requesting the master xpub at path `m/` and reading the root_fingerprint from the response.
pub fn get_master_fingerprint(passphrase: &str) -> Result<u32, String> {
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

    resolve(response, passphrase)
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
    passphrase: &str,
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
        passphrase,
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
        passphrase,
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
    resolve(resp, "")
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
        "",
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
            "",
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
