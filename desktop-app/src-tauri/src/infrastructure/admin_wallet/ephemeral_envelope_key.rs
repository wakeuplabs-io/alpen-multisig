use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::SECP256K1;
use rand::rngs::OsRng;

/// Generate a fresh ephemeral envelope keypair using a CSPRNG.
///
/// The keypair is in-memory only — never persisted, never derived from a mnemonic or seed.
/// A new random keypair is generated on every call. OsRng cannot fail in practice; a panic
/// on entropy failure is acceptable.
pub(crate) fn generate_ephemeral_envelope_keypair() -> UntweakedKeypair {
    let (sk, _) = SECP256K1.generate_keypair(&mut OsRng);
    UntweakedKeypair::from_secret_key(SECP256K1, &sk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_distinct_keypairs_on_consecutive_calls() {
        let kp1 = generate_ephemeral_envelope_keypair();
        let kp2 = generate_ephemeral_envelope_keypair();
        assert_ne!(
            kp1.secret_key().secret_bytes(),
            kp2.secret_key().secret_bytes()
        );
    }
}
