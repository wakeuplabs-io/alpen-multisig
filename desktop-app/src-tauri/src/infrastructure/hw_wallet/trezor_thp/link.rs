//! The raw 64-byte packet pipe to a THP device: USB for hardware, UDP for the emulator.
//!
//! `trezor-client` has both links already, but they are private to its V1 transport and its
//! `AvailableDevice` does not say which one it found, so the two are opened here directly.

use std::io::{self, ErrorKind};
use std::net::UdpSocket;
use std::time::Duration;

use rusb::{DeviceHandle, GlobalContext};

pub(super) const PACKET_LEN: usize = 64;

/// USB id shared by every current Trezor model (Safe 3, Safe 5, Safe 7, Model T).
const TREZOR_USB_ID: (u16, u16) = (0x1209, 0x53C1);
/// The normal (non-debug) interface and its endpoints.
const USB_INTERFACE: u8 = 0;
const USB_ENDPOINT_OUT: u8 = 0x01;
const USB_ENDPOINT_IN: u8 = 0x81;
const USB_WRITE_TIMEOUT: Duration = Duration::from_secs(1);

/// Where the emulator listens, the same address `trezor-client` probes.
const EMULATOR_ADDR: &str = "127.0.0.1:21324";
const EMULATOR_PING_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) enum PacketLink {
    Usb(DeviceHandle<GlobalContext>),
    Udp(UdpSocket),
}

impl PacketLink {
    /// Opens the plugged-in Trezor, or the emulator if no device is on USB.
    pub(super) fn open() -> io::Result<Self> {
        if let Some(handle) = open_usb().map_err(io::Error::other)? {
            return Ok(Self::Usb(handle));
        }
        if let Some(socket) = open_emulator()? {
            return Ok(Self::Udp(socket));
        }
        Err(io::Error::new(
            ErrorKind::NotFound,
            "Trezor device not found",
        ))
    }

    pub(super) fn send(&mut self, packet: &[u8]) -> io::Result<()> {
        match self {
            Self::Usb(handle) => handle
                .write_interrupt(USB_ENDPOINT_OUT, packet, USB_WRITE_TIMEOUT)
                .map(|_| ())
                .map_err(io::Error::other),
            Self::Udp(socket) => socket.send(packet).map(|_| ()),
        }
    }

    /// The next packet, or `None` if nothing arrived within `timeout`.
    pub(super) fn recv(&mut self, timeout: Duration) -> io::Result<Option<Vec<u8>>> {
        let mut packet = vec![0u8; PACKET_LEN];
        let read = match self {
            Self::Usb(handle) => match handle.read_interrupt(USB_ENDPOINT_IN, &mut packet, timeout)
            {
                Err(rusb::Error::Timeout) => return Ok(None),
                other => other.map_err(io::Error::other)?,
            },
            Self::Udp(socket) => {
                socket.set_read_timeout(Some(timeout))?;
                match socket.recv(&mut packet) {
                    Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                        return Ok(None)
                    }
                    other => other?,
                }
            }
        };
        packet.truncate(read);
        Ok(Some(packet))
    }
}

fn open_usb() -> rusb::Result<Option<DeviceHandle<GlobalContext>>> {
    let Some(device) = rusb::devices()?.iter().find(|device| {
        device
            .device_descriptor()
            .is_ok_and(|d| (d.vendor_id(), d.product_id()) == TREZOR_USB_ID)
    }) else {
        return Ok(None);
    };
    let handle = device.open()?;
    handle.claim_interface(USB_INTERFACE)?;
    Ok(Some(handle))
}

/// The emulator answers a raw `PINGPING` with `PONGPONG` outside of any protocol.
fn open_emulator() -> io::Result<Option<UdpSocket>> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    socket.connect(EMULATOR_ADDR)?;
    socket.set_read_timeout(Some(EMULATOR_PING_TIMEOUT))?;
    socket.send(b"PINGPING")?;
    let mut reply = [0u8; PACKET_LEN];
    match socket.recv(&mut reply) {
        Ok(n) if &reply[..n] == b"PONGPONG" => Ok(Some(socket)),
        Ok(_) => Ok(None),
        // Nothing listening: a refused port or a silent one both mean no emulator.
        Err(_) => Ok(None),
    }
}
