//! Governance actions that a signer can propose — client-side domain.
//!
//! These types mirror (a subset of) the protocol's `MultisigAction` without depending
//! on Strata crates. Translation to/from the canonical SSZ form lives in
//! `crate::infrastructure::action_codec`.

use std::num::NonZeroU8;
use std::str::FromStr;

use bitcoin::{Address, Network, ScriptBuf};

use crate::domain::authority::Authority;

/// An x-only (even-parity) secp256k1 public key (32 bytes, no 02/03 prefix).
///
/// Used for bridge operator keys. The ASM models these as `EvenPublicKey` (32 bytes)
/// while signer-set keys use `CompressedPublicKey` (33 bytes) — keeping them distinct
/// prevents codec errors from being silently swapped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvenPubKey([u8; 32]);

/// Failure to construct an `EvenPubKey`.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EvenPubKeyError {
    #[error("invalid hex: {0}")]
    Hex(String),
    #[error("expected 32 bytes, got {0}")]
    WrongLength(usize),
    #[error("invalid secp256k1 x-only public key: {0}")]
    InvalidPoint(String),
}

impl EvenPubKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn from_hex(s: &str) -> Result<Self, EvenPubKeyError> {
        let bytes = hex::decode(s).map_err(|e| EvenPubKeyError::Hex(e.to_string()))?;
        let len = bytes.len();
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| EvenPubKeyError::WrongLength(len))?;
        // Validate the bytes are a valid secp256k1 x-only public key (on the curve).
        // Without this, any 32-byte hex passes — including all-zeros or random bytes
        // that are not valid curve points — and the error only surfaces deep in the
        // codec at SSZ encode time.
        bitcoin::secp256k1::XOnlyPublicKey::from_slice(&arr)
            .map_err(|e| EvenPubKeyError::InvalidPoint(e.to_string()))?;
        Ok(Self(arr))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Update to the bridge operator set.
///
/// `add_members` are x-only public keys of operators to add.
/// `remove_members` are operator indices (u32) to remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorSetUpdate {
    pub add_members: Vec<EvenPubKey>,
    pub remove_members: Vec<u32>,
}

/// A compressed secp256k1 public key (33 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompressedPubKey([u8; 33]);

/// Failure to construct a `CompressedPubKey`.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PubKeyError {
    #[error("invalid hex: {0}")]
    Hex(String),
    #[error("expected 33 bytes, got {0}")]
    WrongLength(usize),
}

impl CompressedPubKey {
    /// Wraps a 33-byte array.
    pub fn new(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }

    /// Parses a hex-encoded compressed public key.
    pub fn from_hex(s: &str) -> Result<Self, PubKeyError> {
        let bytes = hex::decode(s).map_err(|e| PubKeyError::Hex(e.to_string()))?;
        let len = bytes.len();
        let arr: [u8; 33] = bytes
            .try_into()
            .map_err(|_| PubKeyError::WrongLength(len))?;
        Ok(Self(arr))
    }

    /// Hex-encoded representation.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Raw 33-byte slice.
    pub fn as_bytes(&self) -> &[u8; 33] {
        &self.0
    }
}

/// The bridge's safe harbour destination: the x-only key of a P2TR output.
///
/// Upstream models this as a `SafeHarbourAddress`, which wraps a BOSD `Descriptor` restricted to
/// taproot. This type deliberately holds the bare 32-byte key instead, for two reasons:
///
/// - the BOSD types belong to `infrastructure::action_codec`, which is by contract the only module
///   that imports protocol crates;
/// - `Descriptor::new_p2tr` takes `&[u8; 32]`, so no `bitcoin::Address` ever crosses between our
///   `bitcoin` and whichever one `bitcoin-bosd` was built against. The conversion has no shared
///   type in it and therefore cannot break on a version bump.
///
/// A P2TR script is `OP_1 PUSH32 <key>` — 34 bytes — and the BOSD wire form is the type tag `0x04`
/// followed by the same key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SafeHarbourDescriptor([u8; 32]);

/// BOSD type tag for a P2TR descriptor (`DescriptorType::P2tr`).
const BOSD_P2TR_TAG: u8 = 0x04;

/// Failure to construct a [`SafeHarbourDescriptor`].
///
/// One variant per rejection rather than one opaque string: the create form has to tell a signer
/// *what* was wrong with the address they pasted, and "invalid address" is the answer that sends
/// them to paste it again unchanged.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SafeHarbourDescriptorError {
    #[error("not a valid Bitcoin address: {0}")]
    Address(String),
    #[error("address is for {found}, but this deployment is on {expected}")]
    WrongNetwork { expected: Network, found: Network },
    #[error("safe harbour must be a taproot (P2TR) address")]
    NotP2tr,
    #[error("invalid hex: {0}")]
    Hex(String),
    #[error("expected 33 bytes (type tag + 32-byte key), got {0}")]
    WrongLength(usize),
    #[error("descriptor type tag must be 0x04 (P2TR), got {0:#04x}")]
    NotP2trTag(u8),
    #[error("invalid secp256k1 x-only public key: {0}")]
    InvalidPoint(String),
}

impl SafeHarbourDescriptor {
    /// Parses a bech32m P2TR address, requiring `network`.
    ///
    /// The network check is not a safety property — BOSD carries no network, so the same key yields
    /// the same descriptor bytes everywhere and only the human-readable prefix differs. It is
    /// rejected because an address from another network is near-conclusive evidence that the
    /// operator took it from the wrong wallet. See the spec's Constraint 4.
    pub fn from_address(s: &str, network: Network) -> Result<Self, SafeHarbourDescriptorError> {
        let unchecked = Address::from_str(s.trim())
            .map_err(|e| SafeHarbourDescriptorError::Address(e.to_string()))?;
        let address = unchecked.require_network(network).map_err(|_| {
            // `require_network` does not report what it found, and the message is the whole
            // point of this variant, so re-parse to name it.
            let found = Address::from_str(s.trim())
                .ok()
                .and_then(|a| {
                    [
                        Network::Bitcoin,
                        Network::Testnet,
                        Network::Signet,
                        Network::Regtest,
                    ]
                    .into_iter()
                    .find(|n| a.clone().require_network(*n).is_ok())
                })
                .unwrap_or(Network::Bitcoin);
            SafeHarbourDescriptorError::WrongNetwork {
                expected: network,
                found,
            }
        })?;

        let script = address.script_pubkey();
        let bytes = script.as_bytes();
        // A taproot output is exactly `OP_1 PUSH32 <32 bytes>`; anything else is another script
        // type, which upstream's `SafeHarbourAddress::try_from` refuses as well.
        if !script.is_p2tr() || bytes.len() != 34 {
            return Err(SafeHarbourDescriptorError::NotP2tr);
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes[2..34]);
        Self::from_key(key)
    }

    /// Parses the BOSD wire form: the type tag `0x04` followed by the 32-byte key.
    pub fn from_hex(s: &str) -> Result<Self, SafeHarbourDescriptorError> {
        let bytes =
            hex::decode(s.trim()).map_err(|e| SafeHarbourDescriptorError::Hex(e.to_string()))?;
        if bytes.len() != 33 {
            return Err(SafeHarbourDescriptorError::WrongLength(bytes.len()));
        }
        if bytes[0] != BOSD_P2TR_TAG {
            return Err(SafeHarbourDescriptorError::NotP2trTag(bytes[0]));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes[1..33]);
        Self::from_key(key)
    }

    /// Validates the key is on the curve and wraps it.
    ///
    /// Both entry paths funnel through here: bech32m does not check the curve, and BOSD does
    /// (`validate_xonly_pubkey`), so without this an address could be accepted by the form and
    /// rejected by the codec — an error surfacing two screens away from the field that caused it.
    fn from_key(key: [u8; 32]) -> Result<Self, SafeHarbourDescriptorError> {
        bitcoin::secp256k1::XOnlyPublicKey::from_slice(&key)
            .map_err(|e| SafeHarbourDescriptorError::InvalidPoint(e.to_string()))?;
        Ok(Self(key))
    }

    /// The x-only key of the taproot output.
    pub fn as_key(&self) -> &[u8; 32] {
        &self.0
    }

    /// The BOSD wire form: `0x04` followed by the key. This is what the device displays.
    pub fn to_bosd_bytes(&self) -> [u8; 33] {
        let mut out = [0u8; 33];
        out[0] = BOSD_P2TR_TAG;
        out[1..33].copy_from_slice(&self.0);
        out
    }

    /// The BOSD wire form as hex — the exact string in the signing message's details line.
    pub fn to_bosd_hex(&self) -> String {
        hex::encode(self.to_bosd_bytes())
    }

    /// Renders the destination as an address on `network`.
    ///
    /// Built from the script rather than through `p2tr_tweaked` so the full `Network` decides the
    /// prefix: `NetworkKind` cannot tell regtest (`bcrt1p`) from testnet (`tb1p`).
    pub fn to_address(&self, network: Network) -> String {
        let mut script = Vec::with_capacity(34);
        script.push(0x51); // OP_1
        script.push(0x20); // PUSH32
        script.extend_from_slice(&self.0);
        let script = ScriptBuf::from_bytes(script);
        Address::from_script(script.as_script(), network)
            .map(|a| a.to_string())
            // Unreachable: the script was just built as a well-formed P2TR output. Falling back to
            // the hex keeps a display path honest rather than panicking on a UI read.
            .unwrap_or_else(|_| self.to_bosd_hex())
    }
}

/// A change to a multisig authority's signer set and/or threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultisigUpdate {
    /// The authority being **modified** — the target, which travels in the action and is never
    /// inferred from the session. For the three self-rotating updates this equals the
    /// `Authority` of the `Proposal` that carries the action, but for
    /// `StrataSecurityCouncilMultisigUpdate` it does not: the council's membership is rotated by
    /// the Strata Administrator, so the proposal's authority is `StrataAdmin` while this field is
    /// `SecurityCouncil`. Deriving the authorizing role from this field is upstream's job, and it
    /// already does it. See `docs/specs/security-council-signer-update.md` Constraint 2.
    pub role: Authority,
    pub add_keys: Vec<CompressedPubKey>,
    pub remove_keys: Vec<CompressedPubKey>,
    pub new_threshold: NonZeroU8,
}

/// A verification key update action.
///
/// The `authority` determines the wire action variant:
/// - `StrataAdmin` → `OlStfVkUpdate` (OL STF predicate in CheckpointState)
/// - `AlpenAdmin`  → `EeStfVkUpdate` (EE predicate)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VkUpdate {
    pub authority: Authority,
    /// `PredicateTypeId` as raw byte: 0=NeverAccept, 1=AlwaysAccept, 10=Bip340Schnorr, 20=Sp1Groth16
    pub type_id: u8,
    /// Backend-specific condition bytes (empty for Never/Always, 32 for Schnorr, 356 for SP1).
    pub condition: Vec<u8>,
}

/// An update to the Strata sequencer's public key.
///
/// Authorized by the Strata Sequencer Manager multisig.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequencerKeyUpdate {
    pub new_pub_key: EvenPubKey,
}

/// A governance action that a signer can propose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    MultisigUpdate(MultisigUpdate),
    VkUpdate(VkUpdate),
    OperatorSetUpdate(OperatorSetUpdate),
    SequencerKeyUpdate(SequencerKeyUpdate),
    /// Set the bridge's safe harbour destination. Authorized by the **Strata Administrator**, not
    /// by the council: the council decides when the sweep fires, the administrator decides where
    /// the funds land, and one authority holding both could trigger a sweep and pick its
    /// destination. See `docs/specs/security-council-safe-harbour-address.md`.
    SafeHarbourAddressUpdate(SafeHarbourDescriptor),
    /// Activate the bridge safe harbour immediately. Authorized by the Strata Security Council and
    /// payload-less upstream — the sequence number travels with the proposal, not the action.
    Defcon1,
    /// Activate the bridge safe harbour after `confirmation_depths.defcon3` blocks. Same authority
    /// and same payload-less shape as `Defcon1`, and the same message relayed to the bridge — the
    /// delay, during which the council can still cancel it, is the whole difference.
    Defcon3,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_HEX: &str = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";
    // secp256k1 generator G x-coordinate (32 bytes, even parity)
    const EVEN_KEY_HEX: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    #[test]
    fn test_compressed_pubkey_from_hex_ok() {
        let pk = CompressedPubKey::from_hex(VALID_HEX).expect("valid 33-byte hex");
        assert_eq!(pk.to_hex(), VALID_HEX);
    }

    #[test]
    fn test_compressed_pubkey_rejects_short_length() {
        let short = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709e";
        let err = CompressedPubKey::from_hex(short).unwrap_err();
        assert!(matches!(err, PubKeyError::WrongLength(32)));
    }

    #[test]
    fn test_compressed_pubkey_rejects_invalid_hex() {
        let err = CompressedPubKey::from_hex("zz").unwrap_err();
        assert!(matches!(err, PubKeyError::Hex(_)));
    }

    #[test]
    fn test_action_builds() {
        let pk = CompressedPubKey::from_hex(VALID_HEX).unwrap();
        let update = MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: vec![pk],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).unwrap(),
        };
        let action = Action::MultisigUpdate(update.clone());
        match action {
            Action::MultisigUpdate(u) => assert_eq!(u, update),
            Action::VkUpdate(_)
            | Action::OperatorSetUpdate(_)
            | Action::SequencerKeyUpdate(_)
            | Action::SafeHarbourAddressUpdate(_)
            | Action::Defcon1
            | Action::Defcon3 => {
                panic!("unexpected variant")
            }
        }
    }

    #[test]
    fn test_even_pubkey_from_hex_ok() {
        let pk = EvenPubKey::from_hex(EVEN_KEY_HEX).expect("valid 32-byte x-only hex");
        assert_eq!(pk.to_hex(), EVEN_KEY_HEX);
    }

    #[test]
    fn test_even_pubkey_rejects_33_byte_hex() {
        // 33-byte compressed key (66 hex chars) — must be rejected
        let err = EvenPubKey::from_hex(VALID_HEX).unwrap_err();
        assert!(matches!(err, EvenPubKeyError::WrongLength(33)));
    }

    #[test]
    fn test_even_pubkey_rejects_invalid_hex() {
        let err = EvenPubKey::from_hex("zz").unwrap_err();
        assert!(matches!(err, EvenPubKeyError::Hex(_)));
    }

    #[test]
    fn test_even_pubkey_rejects_invalid_curve_point() {
        // 32 bytes of zeros — not a valid secp256k1 point (the identity/at-infinity
        // is not representable as a 32-byte x-coordinate).
        let err = EvenPubKey::from_hex(
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap_err();
        assert!(matches!(err, EvenPubKeyError::InvalidPoint(_)));
    }

    #[test]
    fn test_even_pubkey_rejects_value_exceeding_field_prime() {
        // 0xFFFF...FFFF (2^256 - 1) exceeds the secp256k1 field prime and cannot be
        // a valid x-coordinate.
        let err = EvenPubKey::from_hex(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
        .unwrap_err();
        assert!(matches!(err, EvenPubKeyError::InvalidPoint(_)));
    }

    // ─── SafeHarbourDescriptor ──────────────────────────────────────────────
    //
    // The conversion an operator's address goes through before it becomes the destination every
    // bridge satoshi would sweep to. It is the one step of this slice that is ours rather than
    // upstream's, and the one where a defect is invisible to a signer — so every rejection has a
    // row here, and both entry paths are proved to agree.

    /// x-only key of the secp256k1 generator point G. Upstream pins the same value in its own
    /// safe-harbour tests, and it is the address the local stack ships with.
    const G_XONLY_HEX: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    const G_BOSD_HEX: &str = "0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    fn g_address(network: bitcoin::Network) -> String {
        SafeHarbourDescriptor::from_hex(G_BOSD_HEX)
            .expect("generator point is a valid x-only key")
            .to_address(network)
    }

    #[test]
    fn safe_harbour_accepts_a_p2tr_address_on_the_active_network() {
        let address = g_address(bitcoin::Network::Regtest);
        assert!(
            address.starts_with("bcrt1p"),
            "expected a regtest taproot address, got {address}"
        );

        let parsed = SafeHarbourDescriptor::from_address(&address, bitcoin::Network::Regtest)
            .expect("a P2TR address on the active network is accepted");
        assert_eq!(parsed.as_key(), &hex::decode(G_XONLY_HEX).unwrap()[..]);
    }

    #[test]
    fn safe_harbour_rejects_an_address_from_another_network() {
        let mainnet = g_address(bitcoin::Network::Bitcoin);
        let err = SafeHarbourDescriptor::from_address(&mainnet, bitcoin::Network::Regtest)
            .expect_err("a mainnet address must not be accepted on regtest");
        assert_eq!(
            err,
            SafeHarbourDescriptorError::WrongNetwork {
                expected: bitcoin::Network::Regtest,
                found: bitcoin::Network::Bitcoin,
            }
        );
    }

    #[test]
    fn safe_harbour_rejects_a_non_taproot_address() {
        // P2WPKH over a 20-byte program: parses as an address, is not a taproot output. Upstream's
        // `SafeHarbourAddress::try_from` refuses the same value, one layer later. Built from raw
        // opcodes (`OP_0 PUSH20 <hash>`) so the fixture needs no hash traits in scope.
        let mut raw = vec![0x00, 0x14];
        raw.extend_from_slice(&[0xAA; 20]);
        let script = bitcoin::ScriptBuf::from_bytes(raw);
        let address =
            bitcoin::Address::from_script(script.as_script(), bitcoin::Network::Regtest).unwrap();
        let err =
            SafeHarbourDescriptor::from_address(&address.to_string(), bitcoin::Network::Regtest)
                .expect_err("a P2WPKH address must not be accepted");
        assert_eq!(err, SafeHarbourDescriptorError::NotP2tr);
    }

    #[test]
    fn safe_harbour_rejects_text_that_is_not_an_address() {
        let err = SafeHarbourDescriptor::from_address("not-an-address", bitcoin::Network::Regtest)
            .expect_err("garbage must not parse");
        assert!(matches!(err, SafeHarbourDescriptorError::Address(_)));
    }

    #[test]
    fn safe_harbour_accepts_the_bosd_wire_form() {
        let parsed = SafeHarbourDescriptor::from_hex(G_BOSD_HEX).expect("valid BOSD P2TR hex");
        assert_eq!(parsed.to_bosd_hex(), G_BOSD_HEX);
    }

    #[test]
    fn safe_harbour_rejects_a_descriptor_that_is_not_p2tr_tagged() {
        // Tag 0x03 with a 32-byte payload is a valid BOSD descriptor — P2WSH — and upstream
        // refuses it for the safe harbour. Same answer here, two layers earlier.
        let err = SafeHarbourDescriptor::from_hex(&format!("03{G_XONLY_HEX}"))
            .expect_err("a non-P2TR type tag must be rejected");
        assert_eq!(err, SafeHarbourDescriptorError::NotP2trTag(0x03));
    }

    #[test]
    fn safe_harbour_rejects_hex_of_the_wrong_length() {
        // The bare 32-byte key, without its type tag: the most likely paste mistake.
        let err = SafeHarbourDescriptor::from_hex(G_XONLY_HEX)
            .expect_err("32 bytes is a key, not a descriptor");
        assert_eq!(err, SafeHarbourDescriptorError::WrongLength(32));
    }

    #[test]
    fn safe_harbour_rejects_a_key_that_is_not_on_the_curve() {
        // bech32m does not check the curve and BOSD does, so without this check the form would
        // accept an address the codec then refuses — an error two screens from its cause.
        let err = SafeHarbourDescriptor::from_hex(&format!("04{}", "00".repeat(32)))
            .expect_err("the all-zero scalar is not a valid x-only key");
        assert!(matches!(err, SafeHarbourDescriptorError::InvalidPoint(_)));
    }

    /// The claim the two entry paths make together, which neither makes alone: an address and the
    /// descriptor hex of the same destination resolve to the same key.
    #[test]
    fn safe_harbour_address_and_hex_agree_on_the_same_destination() {
        let from_hex = SafeHarbourDescriptor::from_hex(G_BOSD_HEX).unwrap();
        let from_address = SafeHarbourDescriptor::from_address(
            &g_address(bitcoin::Network::Regtest),
            bitcoin::Network::Regtest,
        )
        .unwrap();
        assert_eq!(from_hex, from_address);
    }

    /// Only the prefix moves between networks: BOSD carries no network, so the descriptor bytes —
    /// the value the device displays and the chain stores — are identical everywhere.
    #[test]
    fn safe_harbour_descriptor_bytes_do_not_depend_on_the_network() {
        let descriptor = SafeHarbourDescriptor::from_hex(G_BOSD_HEX).unwrap();
        assert!(descriptor
            .to_address(bitcoin::Network::Bitcoin)
            .starts_with("bc1p"));
        assert!(descriptor
            .to_address(bitcoin::Network::Regtest)
            .starts_with("bcrt1p"));
        assert_eq!(descriptor.to_bosd_hex(), G_BOSD_HEX);
    }

    #[test]
    fn test_sequencer_key_update_builds() {
        let pk = EvenPubKey::from_hex(EVEN_KEY_HEX).unwrap();
        let update = SequencerKeyUpdate {
            new_pub_key: pk.clone(),
        };
        assert_eq!(update.new_pub_key, pk);
    }

    #[test]
    fn test_operator_set_update_builds() {
        let pk = EvenPubKey::from_hex(EVEN_KEY_HEX).unwrap();
        let update = OperatorSetUpdate {
            add_members: vec![pk],
            remove_members: vec![5],
        };
        assert_eq!(update.add_members.len(), 1);
        assert_eq!(update.remove_members, vec![5u32]);
    }
}

