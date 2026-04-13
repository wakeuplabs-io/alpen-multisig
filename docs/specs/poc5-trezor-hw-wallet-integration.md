# Spec: POC-5 — Trezor Hardware Wallet Integration

## Objective

Prove that the desktop app can communicate with a physical Trezor device (or emulator) via HID to: read a public key and address at a BIP-84 derivation path, and produce a signature over an SPS-65 sighash — all bridged from the React frontend through Tauri IPC to a Rust HID driver.

## Scope

### Included

- `trezor-client = "0.1.5"` dependency in `desktop-app/src-tauri/Cargo.toml`
- `infrastructure/hw_wallet/` module with `trezor` submodule and shared `HwWalletInfo` struct
- Two Tauri commands: `get_trezor_info`, `sign_with_trezor`
- TypeScript `WalletAdapter` interface and `createTrezorPocAdapter()` factory
- Integration test binary `trezor_test` (requires emulator + trezord)
- `wallet/types.ts` with `WalletVendor`, `WalletAdapter`, `SignatureFormat` domain types
- `wallet/create-poc-wallet-adapter.ts` factory dispatching across mock/mnemonic/trezor/ledger

### NOT included

- PIN entry or passphrase entry (returns descriptive error — not supported in this build)
- Ledger implementation (stub only, pending Speculos validation)
- Raw ECDSA signing via Trezor (blocked by BIP-137 prefix — see open issue below)
- Backend integration or session-bound signing
- UI flow beyond the POC `App.tsx` demo

## Technical Design

### Layer diagram

```
React (App.tsx)
  └── createTrezorPocAdapter()          wallet/trezor-poc-adapter.ts
        └── tauriCall('get_trezor_info' | 'sign_with_trezor')
              └── commands/hw_wallet.rs  (Tauri IPC boundary)
                    └── infrastructure/hw_wallet/trezor.rs
                          └── trezor-client crate (HID → device)
```

### Key components

| Component | File | Responsibility |
|-----------|------|----------------|
| `open_trezor()` | `trezor.rs` | Opens unique HID connection, initializes device |
| `resolve()` | `trezor.rs` | Drives `TrezorResponse` state machine, handles ButtonRequest/Ack |
| `connect()` | `trezor.rs` | Reads compressed pubkey + P2WPKH address at derivation path |
| `sign_message()` | `trezor.rs` | Gets pubkey + signs hex string via BIP-137 format |
| `HwWalletInfo` | `hw_wallet/mod.rs` | Shared device info type; `camelCase` for JS serialization |
| `get_trezor_info` | `commands/hw_wallet.rs` | Tauri command → `connect()` |
| `sign_with_trezor` | `commands/hw_wallet.rs` | Tauri command → `sign_message()` |
| `createTrezorPocAdapter()` | `trezor-poc-adapter.ts` | `WalletAdapter` implementation for Trezor |
| `WalletAdapter` | `wallet/types.ts` | Interface for all hardware wallet adapters |

### BIP-84 derivation path

Default: `m/84'/0'/0'/0/0` (first native SegWit receive address). Matches the JS POC adapter constant. Overridable per call via `derivation_path` parameter.

### `resolve()` — ButtonRequest loop

Trezor communicates interactively: when the device shows a confirmation screen it sends a `ButtonRequest` and waits for a `ButtonAck` before continuing. `resolve()` is a generic helper that drives this protocol:

```
TrezorResponse::Ok(data)           → done, return data
TrezorResponse::ButtonRequest(req) → send ack, loop again
TrezorResponse::Failure(f)         → return error
TrezorResponse::PinMatrixRequest   → unsupported, return error
TrezorResponse::PassphraseRequest  → unsupported, return error
```

This is correct for an emulator-only build where no PIN or passphrase protection is used.

### Open issue: BIP-137 vs raw ECDSA

Trezor's `sign_message` API applies a `"Bitcoin Signed Message:\n"` prefix and double-SHA256 before signing. This means the resulting signature covers a **different hash** than the bare SPS-65 sighash. Concretely:

- `verify_threshold()` (from POC-3) expects: `ECDSA.verify(sighash, sig, pubkey)`
- Trezor produces: `ECDSA.verify(SHA256d("Bitcoin Signed Message:\n" + sighash_hex), sig, pubkey)`

These are incompatible. The signature is valid BIP-137 but cannot be fed directly into the on-chain ASM verification.

**Resolution options (for the next phase):**

| Option | Description | Tradeoff |
|--------|-------------|----------|
| A | Verify with BIP-137 prefix server-side | Backend must know the format; breaks protocol uniformity |
| B | Use `sign_tx` / PSBT path | Trezor signs a Bitcoin input directly — raw ECDSA over a tx sighash | Requires constructing a PSBT; more complex but protocol-correct |
| C | Use Trezor's `sign_identity` | Experimental, not guaranteed on all firmware versions | |

Option B (PSBT/`sign_tx`) is the correct production path for SPS-65 compatibility. Option A is acceptable for auth nonce signing (where the server controls verification).

### Error handling

All public functions return `Result<T, String>`. This is intentional for the POC:
- Tauri command return types must implement `serde::Serialize`; `String` satisfies this without a custom error type
- All error strings are human-readable and surfaced directly in the frontend
- Production hardening would introduce a typed `HwWalletError` enum

## Test Binary

`cargo run -p desktop-app --bin trezor_test` validates the full emulator flow:

**Prerequisites:**
1. Trezor emulator running (`trezor-user-env` Docker or `trezor-emu-core`)
2. Trezor Bridge: `trezord -e 21324`
3. Emulator seeded: `trezorctl -p udp:127.0.0.1:21324 debug load-device --mnemonic "all all all..." --pin "" --passphrase-protection false`

**Steps validated:**
1. Connect → read pubkey + address at default path
2. Compute SPS-65 sighash for a demo `MultisigAction`
3. Sign via Trezor (`sign_message`) — confirms BIP-137 format in output

The binary explicitly notes the BIP-137 incompatibility in its output, making it a self-documenting probe.

## TypeScript Adapter

`createTrezorPocAdapter()` implements `WalletAdapter` with closures over `derivationPath` and `publicKeyHex` state:

- `connect()` — calls `get_trezor_info`, stores derivation path returned by device
- `signTestPayload(utf8)` — SHA-256 hashes the payload client-side, signs via `sign_with_trezor`
- `signSighash(hex)` — requires prior `connect()`, signs via `sign_with_trezor`; returns `signatureFormat: 'bitcoin-message'` to signal BIP-137 format to callers

The `xpubOrFingerprint` field received from Rust is a truncated display string (first 8 bytes of the compressed pubkey in hex + ellipsis), not a full xpub. The full key is only used server-side for verification — never stored in JS state.

## Protocol Checklist

- [x] Device never receives private key material — signing happens on-device
- [x] User confirmation required for every signing operation (ButtonRequest)
- [x] Derivation path is explicit and matches between Rust and TypeScript adapters
- [x] Signature format is tagged (`signatureFormat: 'bitcoin-message'`) so callers know how to verify
- [x] BIP-137 incompatibility is documented and not hidden
- [ ] Raw ECDSA signing — not yet achieved via Trezor (blocked, see open issue)
- [ ] PIN / passphrase support — not in scope for this POC
- [ ] Ledger parity — stub only
