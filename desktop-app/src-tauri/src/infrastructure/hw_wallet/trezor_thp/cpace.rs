//! CPace over X25519 (draft-irtf-cfrg-cpace), the key exchange behind THP CodeEntry pairing.
//!
//! A port of trezorlib's `thp/cpace.py` and the Elligator2 map in `thp/curve25519.py`. No stable
//! Rust crate exposes Elligator2 on Curve25519 (curve25519-dalek keeps it crate-private), so the
//! map is written here over `crypto-bigint` and checked against the official vectors below.

use crypto_bigint::modular::constant_mod::Residue;
use crypto_bigint::subtle::{ConditionallySelectable, ConstantTimeEq};
use crypto_bigint::{impl_modulus, Encoding, U256};
use rand::RngCore;
use sha2::{Digest, Sha512};

impl_modulus!(
    P25519,
    U256,
    "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffed"
);

type Fe = Residue<P25519, { U256::LIMBS }>;

/// Montgomery coefficient A of Curve25519.
const J: u64 = 486662;
/// sqrt(-1) mod p.
const SQRT_M1: U256 =
    U256::from_be_hex("2b8324804fc1df0b2b4d00993dfbd7a72f431806ad2fe478c4ee1b274a0ea0b0");
/// (p - 5) / 8.
const C4: U256 =
    U256::from_be_hex("0ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd");
/// p - 2, the exponent of a Fermat inversion.
const P_MINUS_2: U256 =
    U256::from_be_hex("7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffeb");

const DSI: &[u8] = b"CPace255";
/// SHA-512 block size, which the generator string is padded towards.
const HASH_BLOCK_SIZE: usize = 128;

fn fe(n: u64) -> Fe {
    Fe::new(&U256::from_u64(n))
}

/// `decodeUCoordinate` from RFC 7748: the top bit is masked, the rest reduced mod p.
fn decode_coordinate(bytes: &[u8; 32]) -> Fe {
    let mut bytes = *bytes;
    bytes[31] &= 0x7f;
    Fe::new(&U256::from_le_bytes(bytes))
}

/// `map_to_curve_elligator2_curve25519` from RFC 9380 §G.2.1, x-coordinate only.
fn elligator2(point: &[u8; 32]) -> [u8; 32] {
    let j = fe(J);
    let u = decode_coordinate(point);
    let tv1 = fe(2) * (u * u);
    let xd = tv1 + fe(1);
    let x1n = -j;
    let tv2 = xd * xd;
    let gxd = tv2 * xd;
    let gx1 = ((j * tv1) * x1n + tv2) * x1n;
    let tv3 = gxd * gxd;
    let tv2 = tv3 * tv3;
    let tv3 = tv3 * gxd * gx1;
    let tv2 = tv2 * tv3;
    let y11 = tv2.pow(&C4) * tv3;
    let y12 = y11 * Fe::new(&SQRT_M1);
    let e1 = (y11 * y11 * gxd).ct_eq(&gx1);
    let y1 = Fe::conditional_select(&y12, &y11, e1);
    let x2n = x1n * tv1;
    let e3 = (y1 * y1 * gxd).ct_eq(&gx1);
    let xn = Fe::conditional_select(&x2n, &x1n, e3);
    (xn * xd.pow(&P_MINUS_2)).retrieve().to_le_bytes()
}

/// Length-prefixed concatenation (`lv_cat`). Every field here is under 128 bytes, so the
/// LEB128 length is a single byte.
fn lv_cat(fields: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for field in fields {
        debug_assert!(field.len() < 0x80);
        out.push(field.len() as u8);
        out.extend_from_slice(field);
    }
    out
}

fn generator_string(prs: &[u8], ci: &[u8], sid: &[u8]) -> Vec<u8> {
    let zero_pad = HASH_BLOCK_SIZE.saturating_sub((1 + DSI.len()) + (1 + prs.len()) + 1);
    lv_cat(&[DSI, prs, &vec![0u8; zero_pad], ci, sid])
}

fn generator(prs: &[u8], ci: &[u8], sid: &[u8]) -> [u8; 32] {
    let hashed = Sha512::digest(generator_string(prs, ci, sid));
    let mut first_half = [0u8; 32];
    first_half.copy_from_slice(&hashed[..32]);
    elligator2(&first_half)
}

/// Runs the host side of CPace with the pairing code as the shared password and the THP
/// handshake hash as the channel identifier. Returns the host's public key, to send to the
/// device, and the shared secret.
pub(super) fn cpace(
    code: &[u8],
    handshake_hash: &[u8],
    trezor_public_key: &[u8; 32],
) -> ([u8; 32], [u8; 32]) {
    let mut private_key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut private_key);
    let public_key = x25519_dalek::x25519(private_key, generator(code, handshake_hash, b""));
    let shared_secret = x25519_dalek::x25519(private_key, *trezor_public_key);
    (public_key, shared_secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes32(hex_str: &str) -> [u8; 32] {
        hex::decode(hex_str).unwrap().try_into().unwrap()
    }

    /// https://elligator.org/vectors/curve25519_direct.vec, as used by the firmware's own test
    /// (`core/tests/test_trezor.crypto.elligator2.py`).
    #[test]
    fn elligator2_matches_the_reference_vectors() {
        let vectors = [
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "66665895c5bc6e44ba8d65fd9307092e3244bf2c18877832bd568cb3a2d38a12",
                "04d44290d13100b2c25290c9343d70c12ed4813487a07ac1176daa5925e7975e",
            ),
            (
                "673a505e107189ee54ca93310ac42e4545e9e59050aaac6f8b5f64295c8ec02f",
                "242ae39ef158ed60f20b89396d7d7eef5374aba15dc312a6aea6d1e57cacf85e",
            ),
            (
                "990b30e04e1c3620b4162b91a33429bddb9f1b70f1da6e5f76385ed3f98ab131",
                "998e98021eb4ee653effaa992f3fae4b834de777a953271baaa1fa3fef6b776e",
            ),
            (
                "341a60725b482dd0de2e25a585b208433044bc0a1ba762442df3a0e888ca063c",
                "683a71d7fca4fc6ad3d4690108be808c2e50a5af3174486741d0a83af52aeb01",
            ),
            (
                "922688fa428d42bc1fa8806998fbc5959ae801817e85a42a45e8ec25a0d7541a",
                "696f341266c64bcfa7afa834f8c34b2730be11c932e08474d1a22f26ed82410b",
            ),
            (
                "0d3b0eb88b74ed13d5f6a130e03c4ad607817057dc227152827c0506a538bb3a",
                "0b00df174d9fb0b6ee584d2cf05613130bad18875268c38b377e86dfefef177f",
            ),
            (
                "01a3ea5658f4e00622eeacf724e0bd82068992fae66ed2b04a8599be16662e35",
                "7ae4c58bc647b5646c9f5ae4c2554ccbf7c6e428e7b242a574a5a9c293c21f7e",
            ),
            (
                "1d991dff82a84afe97874c0f03a60a56616a15212fbe10d6c099aa3afcfabe35",
                "f81f235696f81df90ac2fc861ceee517bff611a394b5be5faaee45584642fb0a",
            ),
            (
                "185435d2b005a3b63f3187e64a1ef3582533e1958d30e4e4747b4d1d3376c728",
                "f938b1b320abb0635930bd5d7ced45ae97fa8b5f71cc21d87b4c60905c125d34",
            ),
        ];
        for (input, output) in vectors {
            assert_eq!(
                elligator2(&bytes32(input)),
                bytes32(output),
                "input {input}"
            );
        }
    }

    /// The generator vector from trezorlib's `tests/test_cpace.py` (the CPace draft's own).
    #[test]
    fn generator_matches_the_reference_vector() {
        let prs = b"Password";
        let ci = b"oc\x0bB_responder\x0bA_initiator";
        let sid = hex::decode("7e4b4791d6a8ef019b936c79fb7f2c57").unwrap();
        let expected_string = hex::decode(concat!(
            "0843506163653235350850617373776f72646d000000000000000000",
            "00000000000000000000000000000000000000000000000000000000",
            "00000000000000000000000000000000000000000000000000000000",
            "00000000000000000000000000000000000000000000000000000000",
            "000000000000000000000000000000001a6f630b425f726573706f6e",
            "6465720b415f696e69746961746f72107e4b4791d6a8ef019b936c79",
            "fb7f2c57",
        ))
        .unwrap();
        assert_eq!(generator_string(prs, ci, &sid), expected_string);
        assert_eq!(
            generator(prs, ci, &sid),
            bytes32("64e8099e3ea682cfdc5cb665c057ebb514d06bf23ebc9f743b51b82242327074")
        );
    }
}
