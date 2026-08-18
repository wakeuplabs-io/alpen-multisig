//! Admin ID Verification Certificate (PRD 06 §3.c.i).
//!
//! Turns an Admin ID and a raw recoverable signature into a verified, base64-encoded
//! certificate — and refuses everything else. The certificate is the signature over
//! `Admin ID: <address>` in Bitcoin Core's `signmessage` encoding, so anyone holding it
//! can recover the compressed public key behind the Admin ID without asking a hardware
//! signer to render a raw key, which no supported device does.
//!
//! Nothing here touches a device: the signature arrives from the wallet adapter port that
//! already signs the session challenge on Trezor, Ledger and the mnemonic dev signer alike.

use std::str::FromStr;

use bitcoin::hashes::Hash;
use bitcoin::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use bitcoin::secp256k1::{Message, SECP256K1};
use bitcoin::sign_message::{signed_msg_hash, MessageSignature};
use bitcoin::{Address, AddressType, PublicKey};

/// A certificate the app has already verified against the Admin ID it names.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminIdCertificate {
    /// The exact string that was signed — line 1 of the copied block.
    pub message: String,
    /// Base64, Bitcoin Core `signmessage` encoding — line 2 of the copied block.
    pub certificate: String,
    /// The compressed public key recovered from the certificate itself.
    pub public_key_hex: String,
}

/// Renders the message a certificate signs. The `Admin ID: ` prefix is part of the signed
/// bytes, not decoration: changing it invalidates every certificate ever issued.
fn render_certificate_message(admin_id: &str) -> String {
    format!("Admin ID: {admin_id}")
}

/// The message the modal displays and the signer signs, for an Admin ID it has validated.
///
/// The address is re-rendered from its parsed form, so the string shown, the string signed
/// and the string verified are the same bytes even if the caller passed padding or an
/// uppercase bech32 form.
pub fn certificate_message(admin_id: &str) -> Result<String, String> {
    Ok(render_certificate_message(
        &parse_admin_id(admin_id)?.to_string(),
    ))
}

/// Encodes a 65-byte `[r||s||recid]` signature into a certificate for `admin_id`, after
/// proving the certificate belongs to that Admin ID.
///
/// The proof is a recovery, not a re-derivation of the address string: an Admin ID may be
/// rendered on any network (`bc1…` from Trezor, `tb1…` from Ledger on test paths), so the
/// recovered key is compared against the address's own witness program.
pub fn build_certificate(
    admin_id: &str,
    signature_hex: &str,
) -> Result<AdminIdCertificate, String> {
    let address = parse_admin_id(admin_id)?;
    let signature = parse_recoverable_signature(signature_hex)?;

    let message = render_certificate_message(&address.to_string());
    let hash = signed_msg_hash(&message);
    let recovered = SECP256K1
        .recover_ecdsa(&Message::from_digest(hash.to_byte_array()), &signature)
        .map_err(|e| format!("could not recover a public key from the signature: {e}"))?;

    let public_key = PublicKey::new(recovered);
    if !address.is_related_to_pubkey(&public_key) {
        return Err(
            "the signature was not made by the Admin ID's key — refusing to issue a certificate"
                .to_string(),
        );
    }

    Ok(AdminIdCertificate {
        message,
        certificate: MessageSignature {
            signature,
            compressed: true,
        }
        .to_base64(),
        public_key_hex: hex::encode(recovered.serialize()),
    })
}

/// Accepts only what PRD 06 §3.b.ii.2 calls an Admin ID: a P2WPKH address, on any network.
fn parse_admin_id(admin_id: &str) -> Result<Address, String> {
    let address = Address::from_str(admin_id.trim())
        .map_err(|e| format!("Admin ID is not a bitcoin address: {e}"))?
        .assume_checked();
    if address.address_type() != Some(AddressType::P2wpkh) {
        return Err("Admin ID must be a P2WPKH address".to_string());
    }
    Ok(address)
}

fn parse_recoverable_signature(signature_hex: &str) -> Result<RecoverableSignature, String> {
    let bytes = hex::decode(signature_hex).map_err(|e| format!("invalid signature hex: {e}"))?;
    if bytes.len() != 65 {
        return Err(format!(
            "invalid signature length: expected 65 bytes, got {}",
            bytes.len()
        ));
    }
    let recid = RecoveryId::from_i32(bytes[64] as i32)
        .map_err(|e| format!("invalid recovery id byte: {e}"))?;
    RecoverableSignature::from_compact(&bytes[..64], recid)
        .map_err(|e| format!("invalid recoverable signature: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::address::KnownHrp;
    use bitcoin::secp256k1::{PublicKey as SecpPublicKey, SecretKey};
    use bitcoin::CompressedPublicKey;

    /// Builds the 65-byte `[r||s||recid]` signature the wallet adapters hand us, exactly as
    /// `signing.rs:166-186` and the two hardware adapters produce it.
    fn sign(message: &str, sk: &SecretKey) -> String {
        let hash = signed_msg_hash(message);
        let sig = SECP256K1.sign_ecdsa_recoverable(&Message::from_digest(hash.to_byte_array()), sk);
        let (recid, compact) = sig.serialize_compact();
        let mut out = [0u8; 65];
        out[..64].copy_from_slice(&compact);
        out[64] = recid.to_i32() as u8;
        hex::encode(out)
    }

    fn admin_id_for(sk: &SecretKey) -> String {
        let pk = SecpPublicKey::from_secret_key(SECP256K1, sk);
        Address::p2wpkh(&CompressedPublicKey(pk), KnownHrp::Mainnet).to_string()
    }

    fn key(byte: u8) -> SecretKey {
        SecretKey::from_slice(&[byte; 32]).expect("valid key")
    }

    #[test]
    fn message_format_is_stable() {
        // The signed bytes are a contract: the device displays this string and every
        // certificate ever issued verifies against it. A stray space breaks all of them.
        assert_eq!(
            render_certificate_message("bc1q5lvgztw04yl7addhh63yry2tsuw5vxj9fxadlp"),
            "Admin ID: bc1q5lvgztw04yl7addhh63yry2tsuw5vxj9fxadlp"
        );
    }

    #[test]
    fn displayed_message_is_validated_and_normalised() {
        let admin_id = admin_id_for(&key(3));

        assert_eq!(
            certificate_message(&format!("  {admin_id}  ")).expect("valid Admin ID"),
            render_certificate_message(&admin_id)
        );
        assert!(certificate_message("not-an-address").is_err());
    }

    #[test]
    fn certificate_uses_the_compressed_header_the_wireframe_pins() {
        let sk = key(7);
        let admin_id = admin_id_for(&sk);
        let signature = sign(&render_certificate_message(&admin_id), &sk);

        let cert = build_certificate(&admin_id, &signature).expect("valid signature");

        let decoded = MessageSignature::from_base64(&cert.certificate).expect("base64");
        let header = decoded.serialize()[0];
        assert!(
            (31..=34).contains(&header),
            "expected a 31 + recid header, got {header}"
        );
    }

    #[test]
    fn recovered_key_re_derives_the_admin_id() {
        // This is req. 3.c.i itself: the certificate alone must yield the compressed key.
        let sk = key(11);
        let admin_id = admin_id_for(&sk);
        let signature = sign(&render_certificate_message(&admin_id), &sk);

        let cert = build_certificate(&admin_id, &signature).expect("valid signature");

        let decoded = MessageSignature::from_base64(&cert.certificate).expect("base64");
        let recovered = decoded
            .recover_pubkey(SECP256K1, signed_msg_hash(&cert.message))
            .expect("recoverable");
        assert_eq!(
            hex::encode(recovered.inner.serialize()),
            cert.public_key_hex
        );
        assert_eq!(
            Address::p2wpkh(&CompressedPublicKey(recovered.inner), KnownHrp::Mainnet).to_string(),
            admin_id
        );
    }

    #[test]
    fn rejects_a_signature_over_a_different_message() {
        let sk = key(13);
        let admin_id = admin_id_for(&sk);
        let signature = sign("Admin ID: something else entirely", &sk);

        let err = build_certificate(&admin_id, &signature).unwrap_err();
        assert!(err.contains("not made by the Admin ID's key"), "{err}");
    }

    #[test]
    fn rejects_a_signature_from_a_different_key() {
        let admin_id = admin_id_for(&key(17));
        let signature = sign(&render_certificate_message(&admin_id), &key(19));

        let err = build_certificate(&admin_id, &signature).unwrap_err();
        assert!(err.contains("not made by the Admin ID's key"), "{err}");
    }

    #[test]
    fn rejects_malformed_signatures_without_panicking() {
        let admin_id = admin_id_for(&key(23));

        assert!(build_certificate(&admin_id, "zz")
            .unwrap_err()
            .contains("hex"));
        assert!(build_certificate(&admin_id, "aabb")
            .unwrap_err()
            .contains("expected 65 bytes"));
    }

    #[test]
    fn rejects_an_admin_id_that_is_not_p2wpkh() {
        let sk = key(29);
        let pk = SecpPublicKey::from_secret_key(SECP256K1, &sk);
        let legacy = Address::p2pkh(CompressedPublicKey(pk), bitcoin::Network::Bitcoin).to_string();
        let signature = sign(&render_certificate_message(&legacy), &sk);

        let err = build_certificate(&legacy, &signature).unwrap_err();
        assert!(err.contains("must be a P2WPKH address"), "{err}");
        assert!(build_certificate("not-an-address", &signature)
            .unwrap_err()
            .contains("not a bitcoin address"));
    }

    #[test]
    fn accepts_an_admin_id_on_a_test_network() {
        // Ledger derives the Admin ID at m/84'/1'/73'/0/0 and renders `tb1…`.
        let sk = key(31);
        let pk = SecpPublicKey::from_secret_key(SECP256K1, &sk);
        let admin_id = Address::p2wpkh(&CompressedPublicKey(pk), KnownHrp::Testnets).to_string();
        let signature = sign(&render_certificate_message(&admin_id), &sk);

        build_certificate(&admin_id, &signature).expect("test-network Admin IDs are valid");
    }
}
