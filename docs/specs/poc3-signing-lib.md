# Spec: POC-3 — Signing Library (Tauri Commands)

## Objective

Prove that the desktop app can use Alpen crates directly to compute SPS-65 sighashes, produce ECDSA signatures, and verify them against a threshold configuration — all exposed as Tauri commands callable from the frontend via IPC.

This is a foundational POC: it validates that the signing primitives work inside the Tauri Rust process before building UI flows or hardware wallet integration on top.

## Scope

### Included

- Root `rust-toolchain.toml` pinning `nightly-2026-01-01` (required by Alpen transitive deps)
- Alpen crate dependencies added to `desktop-app/src-tauri/Cargo.toml` via `{ workspace = true }`
- A `signing` module in `desktop-app/src-tauri/src/` with Tauri commands
- Unit/integration tests for the full signing flow
- ADR-001 crate inventory update

### NOT included

- UI changes (React frontend stays as-is)
- Bitcoin transaction construction (SPS-50/51 envelope)
- Transaction broadcast
- Hardware wallet (HWI) integration
- Backend API calls

## Technical Design

### Dependencies added to `desktop-app/src-tauri/Cargo.toml`

```toml
strata-asm-txs-admin = { workspace = true }
strata-crypto = { workspace = true }
strata-asm-params = { workspace = true }
secp256k1 = { workspace = true }
rand = "0.8"
hex = "0.4"
```

### New file: `desktop-app/src-tauri/src/signing.rs`

Module containing all signing logic and Tauri commands.

#### Serializable types (for Tauri IPC)

```rust
/// Keypair representation for IPC (private key included for demo only)
struct DemoKeypair {
    secret_key_hex: String,
    public_key_hex: String,
}

/// Result of key generation
struct GenerateKeysResult {
    keypairs: Vec<DemoKeypair>,
    threshold: u32,
    num_signers: u32,
}

/// Result of sighash computation
struct SighashResult {
    sighash_hex: String,
    seqno: u64,
    action_description: String,
}

/// A collected signature
struct SignatureResult {
    signer_index: u32,
    public_key_hex: String,
    signature_hex: String,
}

/// Verification result
struct VerifyResult {
    valid: bool,
    signatures_verified: u32,
    threshold_required: u32,
}
```

#### Tauri commands

1. **`generate_demo_keys(num_signers: u32, threshold: u32) -> Result<GenerateKeysResult>`**
   - Generate `num_signers` random `SecretKey`/`PublicKey` pairs using `secp256k1` + `OsRng`
   - Convert to `CompressedPublicKey` (Alpen type)
   - Build a `ThresholdConfig` with the given threshold
   - Return hex-encoded keys and config summary
   - Note: Private keys are returned for demo purposes only. In production, signing happens on hardware wallets.

2. **`compute_sighash(seqno: u64) -> Result<SighashResult>`**
   - Build a hardcoded `MultisigAction` (Strata Admin signer set update — same as e2e test)
   - Call `action.compute_sighash(seqno)` (Alpen crate method)
   - Return hex-encoded 32-byte sighash

3. **`sign_sighash(secret_key_hex: String, sighash_hex: String) -> Result<SignatureResult>`**
   - Parse secret key and sighash from hex
   - Sign the sighash using ECDSA (`secp256k1::Message` + `SECP256K1.sign_ecdsa`)
   - Return hex-encoded signature + corresponding public key

4. **`verify_threshold(public_keys_hex: Vec<String>, threshold: u32, signatures_hex: Vec<String>, sighash_hex: String) -> Result<VerifyResult>`**
   - Reconstruct `ThresholdConfig` from public keys + threshold
   - Parse signatures from hex
   - Call `verify_threshold_signatures` (Alpen crate)
   - Return pass/fail + counts

#### Updated `main.rs`

Register the new commands in the Tauri invoke handler:

```rust
mod signing;

tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        greet,
        signing::generate_demo_keys,
        signing::compute_sighash,
        signing::sign_sighash,
        signing::verify_threshold,
    ])
```

### Flow diagram

```
generate_demo_keys(3, 2)
    → { keypairs: [k0, k1, k2], threshold: 2 }

compute_sighash(seqno=1)
    → { sighash_hex: "ab12...", action_description: "Strata Admin signer set update" }

sign_sighash(k0.secret_key, sighash)
    → { signature_hex: "30440...", public_key_hex: "02ab..." }
sign_sighash(k2.secret_key, sighash)
    → { signature_hex: "3045...", public_key_hex: "03cd..." }

verify_threshold([k0.pub, k1.pub, k2.pub], 2, [sig0, sig2], sighash)
    → { valid: true, signatures_verified: 2, threshold_required: 2 }
```

## Test Cases

### Happy path

1. **Full signing flow (2-of-3 threshold):** Generate 3 keys → compute sighash for seqno=1 → sign with signers 0 and 2 → verify threshold → expect `valid: true`

### Edge cases

2. **Below threshold (1-of-3):** Generate 3 keys → compute sighash → sign with only 1 signer → verify → expect `valid: false`
3. **Sighash determinism:** Compute sighash twice with the same seqno → both must produce identical output
4. **Different seqno produces different sighash:** Compute sighash for seqno=1 and seqno=2 → must differ

### Expected errors

5. **Invalid secret key hex:** Call `sign_sighash` with malformed hex → expect descriptive error
6. **Invalid sighash hex:** Call `sign_sighash` with wrong-length sighash → expect descriptive error
7. **Empty signatures list:** Call `verify_threshold` with no signatures → expect `valid: false`

## Protocol Checklist

- [x] Complies with SPS-65 (uses `compute_sighash` from Alpen crate, which implements tagged hash)
- [x] Preserves manual fallback (this is a self-contained demo, no backend dependency)
- [x] Authority isolation enforced (hardcoded to `StrataAdministrator` role for demo)
- [x] Signer safety guaranteed (private keys only exist in memory for demo; in production, signing is on hardware wallet)
