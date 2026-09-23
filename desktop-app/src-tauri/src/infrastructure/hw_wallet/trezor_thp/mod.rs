//! THP (Trezor Host Protocol) underneath `trezor-client`.
//!
//! Safe 7 (T3W1) firmware speaks only THP, while `trezor-client` speaks Protocol V1. Rather than
//! fork the client, this module gives it a [`Transport`] that carries the same protobuf messages
//! over an encrypted THP channel, so every call in `trezor.rs` is reused unchanged.
//!
//! The channel's keys and nonces live on the host and must outlive a single operation — opening
//! a fresh one each time would mean a new handshake and a new pairing. So the channel lives in a
//! process-wide slot, and the transport handed to `Trezor` is only a session id into it.
//!
//! `trezor-thp` does the framing, ACKs, retransmission and the Noise handshake. Pairing is left
//! to the application: CodeEntry, where the device shows a 6-digit code the signer types into
//! the app. That spans two IPC calls, so a channel can sit in the slot waiting for its code.
//! The pairing is not remembered across app runs (Phase 3 of `docs/specs/trezor-safe7-thp.md`).

mod cpace;
mod link;

use std::io;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use protobuf::{Enum, Message};
use rand::RngCore;
use sha2::{Digest, Sha256};
use trezor_client::client::trezor_with_transport;
use trezor_client::protos::{
    Failure, MessageType, ThpCodeEntryChallenge, ThpCodeEntryCommitment, ThpCodeEntryCpaceHostTag,
    ThpCodeEntryCpaceTrezor, ThpCodeEntrySecret, ThpDeviceProperties, ThpEndRequest,
    ThpEndResponse, ThpMessageType, ThpPairingMethod, ThpPairingRequest, ThpPairingRequestApproved,
    ThpSelectMethod,
};
use trezor_client::transport::{error::Error as TransportError, ProtoMessage, Transport};
use trezor_client::{Model, Trezor};
use trezor_thp::channel::buffered::Buffered;
use trezor_thp::channel::host::{Channel, Mux};
use trezor_thp::channel::PacketInResult;
use trezor_thp::credential::NullCredentialStore;
use trezor_thp::error::TransportError as TransportErrorCode;
use trezor_thp::{Backend, ChannelIO};

use link::{PacketLink, PACKET_LEN};

/// Shown on the device when it asks the signer to allow this host: "Allow {app} on {host}".
const APP_NAME: &str = "Strata Multisig";

/// How long to wait for an ACK before retransmitting, and how many times to try.
const ACK_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_RETRANSMISSIONS: u32 = 10;
/// How often a read wakes up while the device waits on the signer. There is no overall limit,
/// matching the V1 transport: a confirmation takes as long as the signer takes.
const READ_POLL: Duration = Duration::from_secs(1);
/// How long to let a busy device finish with another channel before sending again.
const BUSY_BACKOFF: Duration = Duration::from_millis(500);

const MESSAGE_TYPE_FAILURE: u16 = 3;
const MESSAGE_TYPE_BUTTON_REQUEST: u16 = 26;
const MESSAGE_TYPE_BUTTON_ACK: u16 = 27;

struct Crypto;

impl Backend for Crypto {
    type DH = trezor_noise_rust_crypto::X25519;
    type Cipher = trezor_noise_rust_crypto::Aes256Gcm;
    type Hash = trezor_noise_rust_crypto::Sha256;

    fn random_bytes(dest: &mut [u8]) {
        rand::rngs::OsRng.fill_bytes(dest);
    }
}

fn thp_error(e: trezor_thp::Error) -> io::Error {
    // trezor-thp derives Debug only under debug_assertions, so `{e:?}` breaks release builds.
    let kind = match e {
        trezor_thp::Error::UnexpectedInput => "unexpected input",
        trezor_thp::Error::NotReady => "not ready",
        trezor_thp::Error::MalformedData => "malformed data",
        trezor_thp::Error::InvalidChecksum => "invalid checksum",
        trezor_thp::Error::InsufficientBuffer => "insufficient buffer",
        trezor_thp::Error::CryptoError => "crypto error",
    };
    io::Error::other(format!("THP channel error: {kind}"))
}

/// A channel in any of its phases (allocation, handshake, established) and the link under it.
struct Wire<C> {
    link: PacketLink,
    channel: Buffered<C>,
    /// A complete incoming message is waiting for [`Wire::read`]. It can arrive while
    /// [`Wire::write`] is still waiting for its ACK, when the device piggybacks the two.
    pending: bool,
    /// `TRANSPORT_BUSY` answers since the last message got through.
    busy_retries: u32,
}

impl<C: ChannelIO> Wire<C> {
    fn map<D: ChannelIO>(
        self,
        next: impl FnOnce(C) -> Result<D, trezor_thp::Error>,
    ) -> io::Result<Wire<D>> {
        Ok(Wire {
            link: self.link,
            channel: self.channel.map(next).map_err(thp_error)?,
            pending: self.pending,
            busy_retries: self.busy_retries,
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        while self.channel.packet_out_ready() {
            let packet = self.channel.packet_out().map_err(thp_error)?;
            self.link.send(&packet)?;
        }
        Ok(())
    }

    fn ingest(&mut self, packet: &[u8]) -> io::Result<()> {
        let result = self
            .channel
            .packet_in(packet)
            .check_failed()
            .map_err(thp_error)?;
        match result {
            // The device dropped our message while it was busy with another channel — typically
            // winding down one a previous run left open. Recoverable by design: send it again.
            // The handshake never waits for an ACK, so nothing else would.
            PacketInResult::TransportError {
                error: TransportErrorCode::TransportBusy,
            } => {
                self.busy_retries += 1;
                if self.busy_retries > MAX_RETRANSMISSIONS {
                    return Err(io::Error::other(
                        "The Trezor is busy with another application. Close it and try again.",
                    ));
                }
                std::thread::sleep(BUSY_BACKOFF);
                return self.channel.message_retransmit().map_err(thp_error);
            }
            PacketInResult::TransportError {
                error: TransportErrorCode::DeviceLocked,
            } => {
                return Err(io::Error::other(
                    "The Trezor is locked. Unlock it and try again.",
                ))
            }
            PacketInResult::TransportError { error } => {
                return Err(io::Error::other(format!(
                    "Trezor reported a transport error: {}",
                    error.as_str()
                )))
            }
            _ => {}
        }
        if result.got_ack() || result.got_message() || result.got_channel() {
            self.busy_retries = 0;
        }
        if result.got_message() || result.got_channel() {
            self.pending = true;
        }
        Ok(())
    }

    fn write(&mut self, session_id: u8, message_type: u16, message: &[u8]) -> io::Result<()> {
        self.channel
            .message_in(session_id, message_type, message)
            .map_err(thp_error)?;
        let mut retransmissions = 0;
        loop {
            self.flush()?;
            // Ready again once the ACK arrived — or at once while the channel is being set up,
            // where the library tracks delivery through the handshake itself.
            if self.channel.message_in_ready() {
                return Ok(());
            }
            match self.link.recv(ACK_TIMEOUT)? {
                Some(packet) => self.ingest(&packet)?,
                None if retransmissions < MAX_RETRANSMISSIONS => {
                    retransmissions += 1;
                    self.channel.message_retransmit().map_err(thp_error)?;
                }
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "the Trezor stopped acknowledging messages",
                    ))
                }
            }
        }
    }

    fn read(&mut self) -> io::Result<(u8, u16, Vec<u8>)> {
        while !self.pending {
            if let Some(packet) = self.link.recv(READ_POLL)? {
                self.ingest(&packet)?;
            }
            // Duplicate messages get ACKed again by the library; send whatever it queued.
            self.flush()?;
        }
        self.pending = false;
        let message = self.channel.message_out().map_err(thp_error)?;
        self.flush()?;
        Ok(message)
    }

    /// Sends a message on session 0 and returns the answer, acknowledging any on-device
    /// confirmation in between. A `Failure` becomes a `PermissionDenied` error carrying the
    /// device's message, so a caller can tell the device refusing from the link failing.
    fn call_confirmed(&mut self, message_type: u16, message: &[u8]) -> io::Result<(u16, Vec<u8>)> {
        self.write(0, message_type, message)?;
        loop {
            let (_, reply_type, reply) = self.read()?;
            match reply_type {
                MESSAGE_TYPE_BUTTON_REQUEST => self.write(0, MESSAGE_TYPE_BUTTON_ACK, &[])?,
                MESSAGE_TYPE_FAILURE => {
                    let failure = Failure::parse_from_bytes(&reply).unwrap_or_default();
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("Trezor refused pairing: {}", failure.message()),
                    ));
                }
                _ => return Ok((reply_type, reply)),
            }
        }
    }
}

type HostChannel = Channel<Crypto>;

struct ThpConnection {
    wire: Wire<HostChannel>,
    /// The next session id to hand out. Session 0 is the seedless one; ids run 1..=255.
    next_session_id: u8,
    /// Set while the device shows a pairing code and waits for the signer to type it. The
    /// channel carries no application message until [`submit_pairing_code`] clears it.
    pairing: Option<CodeEntry>,
}

/// What the host must remember between showing the pairing code and receiving it.
struct CodeEntry {
    challenge: [u8; CHALLENGE_LEN],
    /// SHA-256 of the secret the device reveals once the code is proven, sent up front so it
    /// cannot pick the secret after seeing our tag.
    commitment: Vec<u8>,
    trezor_public_key: [u8; 32],
}

const CHALLENGE_LEN: usize = 16;

fn thp_type(message_type: ThpMessageType) -> u16 {
    message_type.value() as u16
}

/// The body of `reply` if it is a `want`, parsed; an error otherwise.
fn expect_reply<M: Message>((got, body): (u16, Vec<u8>), want: ThpMessageType) -> io::Result<M> {
    if got != thp_type(want) {
        return Err(io::Error::other(format!(
            "unexpected Trezor reply during pairing: message type {got}"
        )));
    }
    M::parse_from_bytes(&body).map_err(io::Error::other)
}

fn encode(message: &impl Message) -> io::Result<Vec<u8>> {
    message.write_to_bytes().map_err(io::Error::other)
}

fn open_channel() -> io::Result<ThpConnection> {
    let mut mux = Mux::<Crypto>::new();
    mux.request_channel(true);
    let mut channel = Buffered::new(mux);
    channel.set_packet_len(PACKET_LEN);
    let mut wire = Wire {
        link: PacketLink::open()?,
        channel,
        pending: false,
        busy_retries: 0,
    };

    wire.write(0, 0, &[])?;
    wire.read()?;
    let mut wire = wire.map(|mux| mux.complete(NullCredentialStore))?;

    let properties = ThpDeviceProperties::parse_from_bytes(wire.channel.device_properties())
        .map_err(io::Error::other)?;
    if let (Some(major), Some(minor)) = (
        properties.protocol_version_major,
        properties.protocol_version_minor,
    ) {
        wire.channel
            .set_device_protocol_version(major.min(255) as u8, minor.min(255) as u8);
    }
    // The Noise XX handshake is two request/response rounds.
    for _ in 0..2 {
        wire.write(0, 0, &[])?;
        wire.read()?;
    }
    if !wire.channel.handshake_done() {
        return Err(io::Error::other(
            "the THP handshake with the Trezor did not complete",
        ));
    }
    let mut wire = wire.map(|open| open.complete())?;

    let pairing = start_code_entry(&mut wire, &properties)?;
    Ok(ThpConnection {
        wire,
        next_session_id: 1,
        pairing: Some(pairing),
    })
}

/// Pairs with CodeEntry, the one method release firmware offers: the device shows a 6-digit
/// code and the host proves it knows it. The signer types the code in between, so this stops
/// once the code is on screen and [`finish_code_entry`] completes it on a later call.
///
/// Debug builds and the emulator also offer SkipPairing; it is deliberately not used, so the
/// emulator runs the same path as a device in the signer's hands.
fn start_code_entry(
    wire: &mut Wire<HostChannel>,
    properties: &ThpDeviceProperties,
) -> io::Result<CodeEntry> {
    let offers_code_entry = properties
        .pairing_methods
        .iter()
        .any(|m| m.enum_value() == Ok(ThpPairingMethod::CodeEntry));
    if !offers_code_entry {
        return Err(io::Error::other(
            "This Trezor offers no pairing method this app supports (code entry).",
        ));
    }

    let mut request = ThpPairingRequest::new();
    // The device names the machine it pairs with, as trezorlib does; it refuses an empty name.
    let host_name = gethostname::gethostname().to_string_lossy().into_owned();
    request.set_host_name(if host_name.is_empty() {
        "this computer".to_string()
    } else {
        host_name
    });
    request.set_app_name(APP_NAME.to_string());
    let _: ThpPairingRequestApproved = expect_reply(
        wire.call_confirmed(
            thp_type(ThpMessageType::ThpMessageType_ThpPairingRequest),
            &encode(&request)?,
        )?,
        ThpMessageType::ThpMessageType_ThpPairingRequestApproved,
    )?;

    let mut select = ThpSelectMethod::new();
    select.set_selected_pairing_method(ThpPairingMethod::CodeEntry);
    let commitment: ThpCodeEntryCommitment = expect_reply(
        wire.call_confirmed(
            thp_type(ThpMessageType::ThpMessageType_ThpSelectMethod),
            &encode(&select)?,
        )?,
        ThpMessageType::ThpMessageType_ThpCodeEntryCommitment,
    )?;

    let mut challenge = [0u8; CHALLENGE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut challenge);
    let mut challenge_message = ThpCodeEntryChallenge::new();
    challenge_message.set_challenge(challenge.to_vec());
    let trezor_cpace: ThpCodeEntryCpaceTrezor = expect_reply(
        wire.call_confirmed(
            thp_type(ThpMessageType::ThpMessageType_ThpCodeEntryChallenge),
            &encode(&challenge_message)?,
        )?,
        ThpMessageType::ThpMessageType_ThpCodeEntryCpaceTrezor,
    )?;
    let trezor_public_key = trezor_cpace
        .cpace_trezor_public_key()
        .try_into()
        .map_err(|_| io::Error::other("the Trezor sent a malformed pairing key"))?;

    Ok(CodeEntry {
        challenge,
        commitment: commitment.commitment().to_vec(),
        trezor_public_key,
    })
}

/// The code the device shows for this pairing: SHA-256 over the method, the handshake hash,
/// the revealed secret and our challenge, reduced to six decimal digits.
fn expected_code(handshake_hash: &[u8], secret: &[u8], challenge: &[u8]) -> String {
    let hash = Sha256::new()
        .chain_update([ThpPairingMethod::CodeEntry as u8])
        .chain_update(handshake_hash)
        .chain_update(secret)
        .chain_update(challenge)
        .finalize();
    // `int.from_bytes(hash, "big") % 1_000_000`, folded byte by byte to stay in a u64.
    let code = hash
        .iter()
        .fold(0u64, |acc, byte| (acc * 256 + u64::from(*byte)) % 1_000_000);
    format!("{code:06}")
}

fn finish_code_entry(
    wire: &mut Wire<HostChannel>,
    pairing: &CodeEntry,
    code: &str,
) -> io::Result<()> {
    let handshake_hash = *wire.channel.handshake_hash();
    let (host_public_key, shared_secret) =
        cpace::cpace(code.as_bytes(), &handshake_hash, &pairing.trezor_public_key);
    let mut tag = ThpCodeEntryCpaceHostTag::new();
    tag.set_cpace_host_public_key(host_public_key.to_vec());
    tag.set_tag(Sha256::digest(shared_secret).to_vec());
    let reply = wire
        .call_confirmed(
            thp_type(ThpMessageType::ThpMessageType_ThpCodeEntryCpaceHostTag),
            &encode(&tag)?,
        )
        .map_err(|e| match e.kind() {
            io::ErrorKind::PermissionDenied => io::Error::other(
                "That code does not match the one on the Trezor. Connect again to get a new code.",
            ),
            _ => e,
        })?;
    let secret: ThpCodeEntrySecret =
        expect_reply(reply, ThpMessageType::ThpMessageType_ThpCodeEntrySecret)?;

    // The device proves the code was its own: the secret matches what it committed to before
    // seeing our tag, and the code derives from it. Either failing means something other than
    // our Trezor answered.
    if Sha256::digest(secret.secret())[..] != pairing.commitment[..]
        || expected_code(&handshake_hash, secret.secret(), &pairing.challenge) != code
    {
        return Err(io::Error::other(
            "The Trezor could not prove the pairing code it showed. Do not use this device; \
disconnect it and try again.",
        ));
    }

    let _: ThpEndResponse = expect_reply(
        wire.call_confirmed(
            thp_type(ThpMessageType::ThpMessageType_ThpEndRequest),
            &encode(&ThpEndRequest::new())?,
        )?,
        ThpMessageType::ThpMessageType_ThpEndResponse,
    )?;
    wire.channel.end_pairing();
    Ok(())
}

static CONNECTION: Mutex<Option<ThpConnection>> = Mutex::new(None);

fn connection_slot() -> MutexGuard<'static, Option<ThpConnection>> {
    CONNECTION.lock().unwrap_or_else(|e| e.into_inner())
}

/// Opens the channel unless one is already up. Returns `true` when it opened one just now,
/// which means every session id handed out before is gone. A new channel is left waiting for
/// its pairing code (see [`awaiting_pairing_code`]).
pub fn ensure_channel() -> Result<bool, String> {
    let mut slot = connection_slot();
    if slot.is_some() {
        return Ok(false);
    }
    *slot = Some(open_channel().map_err(|e| format!("Trezor THP connection failed: {e}"))?);
    Ok(true)
}

/// Whether the open channel is waiting for the code the device is showing.
pub fn awaiting_pairing_code() -> bool {
    connection_slot()
        .as_ref()
        .is_some_and(|connection| connection.pairing.is_some())
}

/// Completes the pairing the device is showing a code for.
///
/// A malformed code is refused without touching the device, so the signer can retype it. Any
/// other failure — a wrong code included — drops the channel: the device has abandoned that
/// pairing, and the next connect starts a new one with a new code.
pub fn submit_pairing_code(code: &str) -> Result<(), String> {
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err("The pairing code is the 6 digits shown on the Trezor.".to_string());
    }
    let mut slot = connection_slot();
    let Some(connection) = slot.as_mut() else {
        return Err(NO_PAIRING_IN_PROGRESS.to_string());
    };
    let Some(pairing) = connection.pairing.take() else {
        return Err(NO_PAIRING_IN_PROGRESS.to_string());
    };
    finish_code_entry(&mut connection.wire, &pairing, &code).map_err(|e| {
        *slot = None;
        e.to_string()
    })
}

const NO_PAIRING_IN_PROGRESS: &str = "No Trezor pairing is in progress. Connect again.";

/// Drops the channel, so the next [`ensure_channel`] opens a fresh one.
pub fn close_channel() {
    *connection_slot() = None;
}

/// A session id not yet used on the current channel. The firmware refuses to create a session
/// on an id that already holds one.
pub fn next_session_id() -> Result<u8, String> {
    let mut slot = connection_slot();
    let connection = slot.as_mut().ok_or("no Trezor THP channel is open")?;
    let id = connection.next_session_id;
    connection.next_session_id = id.checked_add(1).unwrap_or(1);
    Ok(id)
}

/// A `Trezor` whose messages travel on `session_id` of the open channel.
pub fn client(session_id: u8) -> Trezor {
    trezor_with_transport(Model::Trezor, Box::new(ThpTransport { session_id }))
}

struct ThpTransport {
    session_id: u8,
}

/// Runs `io` against the open channel. Any I/O error leaves the channel's state unknown, so
/// the channel is dropped and the next operation opens a new one.
fn with_connection<T>(
    io: impl FnOnce(&mut ThpConnection) -> io::Result<T>,
) -> Result<T, TransportError> {
    let mut slot = connection_slot();
    let connection = slot.as_mut().ok_or(TransportError::DeviceDisconnected)?;
    let result = io(connection);
    if result.is_err() {
        *slot = None;
    }
    result.map_err(TransportError::IO)
}

impl Transport for ThpTransport {
    fn session_begin(&mut self) -> Result<(), TransportError> {
        Ok(())
    }

    fn session_end(&mut self) -> Result<(), TransportError> {
        Ok(())
    }

    fn write_message(&mut self, message: ProtoMessage) -> Result<(), TransportError> {
        let message_type = message.message_type().value() as u16;
        with_connection(|c| {
            c.wire
                .write(self.session_id, message_type, message.payload())
        })
    }

    fn read_message(&mut self) -> Result<ProtoMessage, TransportError> {
        let (session_id, message_type, payload) = with_connection(|c| c.wire.read())?;
        if session_id != self.session_id {
            return Err(TransportError::DeviceBadSessionId);
        }
        let message_type = MessageType::from_i32(message_type.into())
            .ok_or(TransportError::InvalidMessageType(message_type.into()))?;
        Ok(ProtoMessage::new(message_type, payload))
    }
}
