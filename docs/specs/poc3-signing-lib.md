# Spec: POC-3 — Signing Library (Tauri Commands)

## Objective

Prove that the desktop app can use Alpen crates directly to compute SPS-65 sighashes, produce ECDSA signatures, and verify them against a threshold configuration — all exposed as Tauri commands callable from the frontend via IPC.

## Scope

### Included

- Root `rust-toolchain.toml` pinning `nightly-2026-01-01`
- Alpen crate dependencies added to `desktop-app/src-tauri/Cargo.toml` via `{ workspace = true }`
- A `signing` module with production Tauri commands
- Exhaustive tests on production functions
- ADR-001 crate inventory update

### NOT included

- UI changes
- Bitcoin transaction construction (SPS-50/51)
- Transaction broadcast
- Hardware wallet (HWI) integration
- Backend API calls

## Technical Design

### Production code vs. test helpers

| Type | Function | Exposed as Tauri command? |
|------|----------|--------------------------|
| **Production** | `compute_sighash(seqno, action)` | Yes |
| **Production** | `sign_sighash(secret_key_hex, sighash_hex)` | Yes |
| **Production** | `verify_threshold(public_keys_hex, threshold, signatures_hex, sighash_hex)` | Yes |
| **Test helper** | `generate_demo_keys(num_signers, threshold)` | No — `#[cfg(test)]` only |
| **Test helper** | `build_demo_action()` | No — `#[cfg(test)]` only |

### Production functions

1. **`compute_sighash(seqno: u64, action: MultisigAction) -> Result<SighashResult>`**
   - Receives an action and seqno, computes the SPS-65 tagged sighash via Alpen crate
   - Returns hex-encoded 32-byte sighash

2. **`sign_sighash(secret_key_hex: String, sighash_hex: String) -> Result<SignatureResult>`**
   - Parse secret key and sighash from hex
   - ECDSA sign with `secp256k1`
   - Return hex-encoded signature + public key

3. **`verify_threshold(public_keys_hex: Vec<String>, threshold: u32, signatures_hex: Vec<String>, sighash_hex: String) -> Result<VerifyResult>`**
   - Reconstruct ThresholdConfig, parse signatures
   - Verify each signature against the signer set
   - Return pass/fail + counts

### Module structure

```
desktop-app/src-tauri/src/
├── main.rs              # Tauri setup, registers production commands only
└── signing.rs           # Production types + commands + #[cfg(test)] test module
```

Future extraction: the signing logic (sighash, sign, verify) should eventually move to a shared crate that both `desktop-app` and `e2e-tests` can consume. For now it lives in the Tauri crate since this is a POC.

## Test Cases

All tests target production functions. Test helpers (`generate_demo_keys`, `build_demo_action`) are used for setup only.

### compute_sighash
1. Returns a valid 32-byte (64 hex chars) sighash
2. Deterministic: same (seqno, action) → same hash
3. Different seqno → different hash

### sign_sighash
4. Signs successfully with a valid key and sighash
5. Returns a signature that is non-empty
6. Invalid secret key hex → descriptive error
7. Invalid sighash (wrong length) → descriptive error

### verify_threshold
8. Full flow: 2-of-3 threshold with 2 valid signatures → `valid: true`
9. Below threshold: 1 signature against threshold 2 → `valid: false`
10. Empty signatures → `valid: false`
11. Invalid signature against valid keys → `valid: false`

## Protocol Checklist

- [x] Complies with SPS-65 (uses `compute_sighash` from Alpen crate)
- [x] Preserves manual fallback (self-contained, no backend dependency)
- [x] Authority isolation enforced (demo uses `StrataAdministrator` role)
- [x] Signer safety guaranteed (private keys only in test helpers, never in production paths)
