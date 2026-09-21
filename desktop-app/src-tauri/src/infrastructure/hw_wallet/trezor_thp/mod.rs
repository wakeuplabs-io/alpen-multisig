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
//! to the application: here, only SkipPairing, which the emulator and debug builds offer.
//! Release firmware requires CodeEntry, which is Phase 2 of `docs/specs/trezor-safe7-thp.md`.

mod link;

use std::io;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use protobuf::{Enum, Message};
use rand::RngCore;
use trezor_client::client::trezor_with_transport;
use trezor_client::protos::{
    Failure, MessageType, ThpDeviceProperties, ThpMessageType, ThpPairingMethod, ThpPairingRequest,
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
    io::Error::other(format!("THP channel error: {e:?}"))
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
    /// confirmation in between. A `Failure` becomes an error carrying the device's message.
    fn call_confirmed(&mut self, message_type: u16, message: &[u8]) -> io::Result<(u16, Vec<u8>)> {
        self.write(0, message_type, message)?;
        loop {
            let (_, reply_type, reply) = self.read()?;
            match reply_type {
                MESSAGE_TYPE_BUTTON_REQUEST => self.write(0, MESSAGE_TYPE_BUTTON_ACK, &[])?,
                MESSAGE_TYPE_FAILURE => {
                    let failure = Failure::parse_from_bytes(&reply).unwrap_or_default();
                    return Err(io::Error::other(format!(
                        "Trezor refused pairing: {}",
                        failure.message()
                    )));
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
}

fn thp_type(message_type: ThpMessageType) -> u16 {
    message_type.value() as u16
}

fn expect_reply(got: u16, want: ThpMessageType) -> io::Result<()> {
    if got == thp_type(want) {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "unexpected Trezor reply during pairing: message type {got}"
        )))
    }
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

    pair(&mut wire, &properties)?;
    wire.channel.end_pairing();
    Ok(ThpConnection {
        wire,
        next_session_id: 1,
    })
}

fn pair(wire: &mut Wire<HostChannel>, properties: &ThpDeviceProperties) -> io::Result<()> {
    let offers_skip = properties
        .pairing_methods
        .iter()
        .any(|m| m.enum_value() == Ok(ThpPairingMethod::SkipPairing));
    if !offers_skip {
        return Err(io::Error::other(
            "This Trezor asks to be paired with a code shown on its screen, which this build \
does not support yet.",
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
    let (reply, _) = wire.call_confirmed(
        thp_type(ThpMessageType::ThpMessageType_ThpPairingRequest),
        &encode(&request)?,
    )?;
    expect_reply(
        reply,
        ThpMessageType::ThpMessageType_ThpPairingRequestApproved,
    )?;

    let mut select = ThpSelectMethod::new();
    select.set_selected_pairing_method(ThpPairingMethod::SkipPairing);
    let (reply, _) = wire.call_confirmed(
        thp_type(ThpMessageType::ThpMessageType_ThpSelectMethod),
        &encode(&select)?,
    )?;
    expect_reply(reply, ThpMessageType::ThpMessageType_ThpEndResponse)
}

static CONNECTION: Mutex<Option<ThpConnection>> = Mutex::new(None);

fn connection_slot() -> MutexGuard<'static, Option<ThpConnection>> {
    CONNECTION.lock().unwrap_or_else(|e| e.into_inner())
}

/// Opens the channel unless one is already up. Returns `true` when it opened one just now,
/// which means every session id handed out before is gone.
pub fn ensure_channel() -> Result<bool, String> {
    let mut slot = connection_slot();
    if slot.is_some() {
        return Ok(false);
    }
    *slot = Some(open_channel().map_err(|e| format!("Trezor THP connection failed: {e}"))?);
    Ok(true)
}

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
