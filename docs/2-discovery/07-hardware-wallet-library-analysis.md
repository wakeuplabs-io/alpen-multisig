# Hardware Wallet Library Analysis — Rust Adapter Implementation

## Overview

This document evaluates the available Rust libraries for implementing hardware wallet communication (Ledger and Trezor) in the Tauri native process. The goal is to select the right stack before implementing the production path described in `06-hardware-wallet-architecture.md`.

**Signing context (SPS-65 — strict):** Admin multisig actions use the **tagged sighash** from SPS-65. The desktop shell computes it in Rust (e.g. `MultisigAction::compute_sighash`); verifiers expect **ECDSA over that 32-byte digest** (no BIP-137 prefix). Canonical form:

```
sighash = SHA256( SHA256(tag) || seqno_be_bytes(8) || sighash_payload )
tag     = "strata/admin/<type_name>"
```

**POC / legacy note:** Early JS adapters used **Bitcoin Signed Message** (BIP-137): the device signs `SHA256d("\x18Bitcoin Signed Message:\n" + varint(len) + message_bytes)`. That produces a **different digest** than the bare SPS-65 sighash. A signature from `sign_message` / BIP-137 **cannot** be verified with `verify_threshold(..., sighash_hex)` against the SPS-65 digest — see `docs/archive/poc-specs/poc5-trezor-hw-wallet-integration.md` (open issue: BIP-137 vs raw ECDSA).

**Library decision:** For **production** Trezor (and Ledger parity), the Rust stack must implement a path where the device ends up signing (or committing to) material that verifies as **ECDSA(msg = SPS-65 sighash, ...)** per protocol — not only “the same API as the old JS POC.”

**Existing workspace pinned versions (must be compatible):**
- `secp256k1 = "0.29.1"`
- `bitcoin = "0.32.6"`

---

## 1. HID Transport Layer

All hardware wallet communication goes over USB HID. This is the foundation both Ledger and Trezor implementations sit on.

### 1.1 `hidapi`

- **Crate:** [`hidapi`](https://crates.io/crates/hidapi)
- **What it is:** Rust bindings to the `libhidapi` C library. Cross-platform (macOS, Windows, Linux). The de facto standard for HID device access in Rust.
- **Used by:** `ledger-transport-hid`, `trezor-client`, and most hardware wallet Rust projects.
- **Tauri note:** Works as a direct dependency in `src-tauri`. No plugin needed — the Rust process owns the HID connection directly, not the WebView.

### 1.2 `tauri-plugin-hid`

- **Crate:** [`tauri-plugin-hid`](https://crates.io/crates/tauri-plugin-hid)
- **What it is:** A Tauri plugin that wraps `hidapi` and exposes HID access to the **WebView via IPC**. Designed for the opposite architecture from what we want (device access in JS, not Rust).
- **Assessment:** Not relevant for this project. We want HID in Rust, not exposed to the WebView.

**Decision:** Use `hidapi` directly as a Rust dependency. No Tauri plugin needed.

---

## 2. Ledger

### 2.1 `ledger_bitcoin_client` (LedgerHQ official)

- **Source:** [`LedgerHQ/app-bitcoin-new`](https://github.com/LedgerHQ/app-bitcoin-new/tree/develop/bitcoin_client) — official LedgerHQ repository
- **Crate:** [`ledger_bitcoin_client`](https://crates.io/crates/ledger_bitcoin_client)
- **What it is:** The official Rust client for Ledger's modern Bitcoin app (app-bitcoin-new). Implements the full Bitcoin app APDU protocol at a high level. Provides `sign_message`, `get_wallet_address`, PSBT signing.
- **Transport:** Uses `ledger-transport-hid` underneath for HID communication.
- **Signing support:** `sign_message(path, message)` — produces a Bitcoin Signed Message signature (BIP-137). This matches the **legacy JS POC** (`btc.signMessage()`), but **not** strict SPS-65 verification: verifiers expect ECDSA over the **tagged admin sighash** (see Overview), not over the BIP-137 preimage. Production parity with Trezor implies a **tx/PSBT** (or equivalent) path, not `sign_message` alone.
- **Maintenance:** Actively maintained by LedgerHQ alongside the Bitcoin app firmware.

```rust
// Approximate API
let transport = TransportNativeHID::new(&hidapi)?;
let client = BitcoinClient::new(transport);
let sig = client.sign_message(&path, message_bytes)?;
// sig: (v, r, s) — BIP-137; strict SPS-65 needs tx/PSBT path + verify_threshold gate
```

**Pros:**
- Official — protocol compatibility guaranteed
- `sign_message` maps 1:1 to the **BIP-137 JS POC** (useful only for non–SPS-65 demos)
- PSBT / tx signing APIs exist for Bitcoin-aligned ECDSA flows
- Active maintenance by LedgerHQ

**Cons:**
- Targets the **new** Bitcoin app only (app-bitcoin-new). Devices running the legacy Bitcoin app need a different path.
- PSBT-based flows (future Taproot signing) require PSBTv2 — adds complexity for that use case.
- Potential version conflicts with workspace `bitcoin = "0.32.6"` — needs verification.

### 2.2 `ledger-transport-hid` + manual APDU (Zondax)

- **Crates:** [`ledger-transport-hid`](https://crates.io/crates/ledger-transport-hid), [`ledger-apdu`](https://crates.io/crates/ledger-apdu)
- **Source:** [`Zondax/ledger-rs`](https://github.com/Zondax/ledger-rs) — widely used in the Rust/blockchain space
- **What it is:** Low-level APDU transport. You construct raw APDU commands and send them to the device. No Bitcoin-specific abstraction — you implement the protocol yourself.
- **Maintenance:** Stable but lower-level. Used by many Cosmos/Polkadot ecosystem projects.

```rust
let transport = LedgerHIDTransport::new(hidapi_device);
let apdu = APDUCommand {
    cla: 0xe0,
    ins: 0x4e,      // sign message
    p1: 0x00,
    p2: 0x01,
    data: payload,
};
let response = transport.exchange(&apdu)?;
```

**Pros:**
- Works with both legacy and new Bitcoin app (you control the APDU)
- No high-level dependency that might conflict with workspace versions
- Full control over the protocol

**Cons:**
- Must implement the Bitcoin Signed Message APDU flow manually
- More code to write and maintain
- Error handling for device state (locked, wrong app open) is your responsibility

### 2.3 `coins-ledger`

- **Crate:** [`coins-ledger`](https://crates.io/crates/coins-ledger)
- **What it is:** Fork of Zondax's `ledger-rs` with minor API refinements. Used by `ethers-ledger`.
- **Assessment:** Essentially the same as option 2.2. No advantage for this use case.

---

## 3. Trezor (SPS-65 strict)

### 3.1 Requirement recap

Under **strict SPS-65**, a hardware signature is acceptable only if it verifies as **ECDSA over the 32-byte tagged admin sighash** — the same `sighash_hex` produced in Rust and checked by `verify_threshold` in [`desktop-app/src-tauri/src/infrastructure/signing.rs`](../../desktop-app/src-tauri/src/infrastructure/signing.rs) (and protocol-side equivalents). There is **no** step that re-hashes the sighash through BIP-137.

**`SignMessage` / `sign_message` is out of scope for this requirement.** Trezor’s message-signing API applies the Bitcoin Signed Message prefix and double-SHA256; the resulting compact signature is valid BIP-137 but **wrong message** for SPS-65 threshold checks.

### 3.2 `trezor-client` — roles: POC vs production

- **Crate:** [`trezor-client`](https://crates.io/crates/trezor-client)
- **What it is:** The most complete **community** Rust client for Trezor. HID + Trezor protobuf (`trezor-common`). Supports Bitcoin, Ethereum, and others.
- **`sign_message`:** Convenient for **POC / debugging** (matches TrezorConnect `signMessage` and the old JS adapter). **Not** a production path for admin SPS-65 signatures — see §3.1.
- **Production direction:** Use the crate’s (or the wire protocol’s) **Bitcoin transaction signing** surface — e.g. PSBT / `SignTx`-style flows — so that the commitment the user approves aligns with how the protocol verifies admin threshold signatures. Exact PSBT construction that binds the SPS-65 digest is a **protocol + product** design step; at library level we only assert: **do not ship `sign_message` as the SPS-65 Trezor adapter.** See `docs/archive/poc-specs/poc5-trezor-hw-wallet-integration.md` (resolution options; Option B PSBT / `sign_tx` called out there as the production-leaning direction).
- **Version:** 0.1.5 (docs build failed; 0.1.4 docs are available and stable)
- **Maintenance:** Not officially maintained by Trezor. Treat as **integration dependency** to validate against emulator + `verify_threshold`, not as a guarantee of long-term API stability.

```rust
// POC-only illustration — BIP-137, NOT SPS-65 strict
let mut trezor = Trezor::find_device(None)?;
trezor.init_device(None)?;
let _sig = trezor.sign_message(path.clone(), "Bitcoin", message_bytes)?;
// Do not feed this signature into verify_threshold(sighash_sps65, ...)
```

**Pros (crate as transport + higher-level Bitcoin ops):** Hides protobuf framing; may expose or be extended toward `SignTx`/PSBT paths needed for SPS-65.

**Cons:** Community-maintained; 0.1.5 docs failure; large proto tree; version pin friction with workspace crates.

### 3.3 `trezor-connect-rs`

- **Crate:** [`trezor-connect-rs`](https://crates.io/crates/trezor-connect-rs)
- **What it is:** Wraps Trezor through a **Deno** bridge to TrezorConnect JS.
- **Assessment:** Same trust/shape problems as JS-in-WebView, plus Deno. **Not viable** for the native Rust signing architecture in `06-hardware-wallet-architecture.md`, regardless of SPS-65.

**Verdict: Not viable.**

### 3.4 Custom implementation via `hidapi` + protobuf

- **What it is:** Implement the Trezor USB protocol with `hidapi` + `prost` (or `protobuf`) and `trezor-common` `.proto` files.
- **Reference:** [`trezor/trezor-common`](https://github.com/trezor/trezor-common)

**SPS-65–relevant flow (conceptual):** Initialize / Features, then the **Bitcoin signing** message sequence that ends in signatures verifiable against the admin sighash (e.g. `SignTx` + transaction/PSBT exchange), **not** `SignMessage` → `MessageSignature` for production admin actions.

```
Device flow (production-shaped, not POC SignMessage):
  hidapi open → Initialize → Features
  → SignTx / PSBT-related messages (per chosen binding to SPS-65 sighash)
  → signatures returned for verify_threshold(msg = SPS-65 sighash)
```

**Pros:** No reliance on `trezor-client` release cadence; narrow proto subset possible; full control over errors and firmware quirks.

**Cons:** High engineering cost (chunked HID, state machine, proto drift); you still must implement the **correct** signing semantics for SPS-65, not only transport.

---

## 4. Comparison Matrix

| | `ledger_bitcoin_client` | `ledger-transport-hid` + manual | `trezor-client` | Custom Trezor protobuf |
|---|---|---|---|---|
| **Vendor** | Ledger | Ledger | Trezor | Trezor |
| **Level** | High (Bitcoin-specific) | Low (raw APDU) | High (protobuf RPC) | Low (raw HID + proto) |
| **`sign_message` (BIP-137)** | Yes (built-in) | Manual | Yes (`sign_message`) | Manual (`SignMessage`) |
| **Strict SPS-65 (ECDSA over tagged admin sighash)** | Not via `sign_message` alone — needs tx/PSBT-style path | Same — manual tx/PSBT if supported by app | Not via `sign_message` — need `SignTx`/PSBT (or equivalent) from crate or fork | Yes — if you implement the correct signing message chain |
| **Official / maintained** | Yes (LedgerHQ) | Yes (Zondax, stable) | No (community) | N/A (you own it) |
| **Legacy app support** | No | Yes | Yes | Yes |
| **Implementation effort** | Low (msg) / higher for PSBT binding | High | Medium–high for SPS-65 path | Very high |
| **Dependency risk** | bitcoin version conflict? | Low | Docs build failure; API drift | Low (you control protos) |
| **Tauri compatibility** | Via `hidapi` | Via `hidapi` | Via `hidapi` | Via `hidapi` |

---

## 5. Open Questions Before Deciding

| # | Question | Impact |
|---|----------|--------|
| 1 | Do target users run the **legacy** Ledger Bitcoin app or the **new** one? Legacy app is common on older Nano S devices. | Determines if `ledger_bitcoin_client` is sufficient or raw APDU is needed. |
| 2 | Does `ledger_bitcoin_client` compile cleanly with `bitcoin = "0.32.6"` and `secp256k1 = "0.29.1"`? | Could force a version bump or manual patching in the workspace. |
| 3 | **Trezor + SPS-65:** Which concrete **SignTx / PSBT** (or other) flow binds the user-approved device commitment to the **same** 32-byte digest `verify_threshold` uses? | Blocks production Trezor; `sign_message` is a dead end for strict SPS-65. |
| 4 | Does `trezor-client` expose a **maintainable** PSBT/`SignTx` API for that binding, or is custom protobuf + minimal message subset lower risk? | Build vs. buy for Trezor **after** Q3 is answered. |
| 5 | Future Taproot / extended Bitcoin flows — in scope? | Affects Ledger and Trezor PSBT surface area and test matrix (Speculos / emulator). |

---

## 6. Recommended Approach

**Target:** **Strict SPS-65** — signatures must verify with **`verify_threshold(..., sighash_hex)`** where `sighash_hex` is the tagged admin digest from Rust (`compute_sighash` / `MultisigAction::compute_sighash`). BIP-137 outputs are **excluded** from production admin signing.

**Ledger:** Keep `ledger_bitcoin_client` as the primary **Ledger transport + Bitcoin app** integration; run `cargo build` to confirm `bitcoin`/`secp256k1` pins. Plan **tx/PSBT** (or protocol-agreed) signing for SPS-65, not `sign_message` alone. Fall back to `ledger-transport-hid` + manual APDU if legacy app or version pins force it.

**Trezor (strict SPS-65):**
1. **Do not** adopt `trezor-client::sign_message` (or wire `SignMessage`) as the production admin signer — it cannot satisfy §3.1.
2. **Do** prototype emulator + `cargo build` with `trezor-client` **only** if using it toward **SignTx / PSBT** (or whatever message chain implements the binding from **§5 row 3**). If the crate lacks a stable path, plan **custom protobuf** for the minimal **transaction signing** exchange, not for `SignMessage` alone.
3. Use **`verify_threshold`** on emulator CI as the **acceptance gate** (see §7 Layer 3).

**HID transport:** `hidapi` as a direct dependency where the stack does not already pull it in.

**Tauri integration pattern** (intent — actual vendor modules must implement SPS-65–compatible signing, not BIP-137):

```rust
#[tauri::command]
async fn sign_action(
    vendor: String,
    action_hex: String,
    seqno: u64,
) -> Result<SignResult, String> {
    let sighash = compute_sighash(seqno, &action_hex)?; // SPS-65 tagged digest; stays in Rust
    match vendor.as_str() {
        // Production: ECDSA over sighash.sighash_hex — NOT sign_message / BIP-137
        "ledger" => ledger::sign_admin_sps65(&sighash.sighash_hex),
        "trezor" => trezor::sign_admin_sps65(&sighash.sighash_hex),
        _ => Err("Unknown vendor".into()),
    }
}
```

Spike work should close **section 5, items 2–4** (Ledger pins, Trezor binding design, crate vs custom) before locking the full implementation.

---

## 7. Testing Strategy — Physical Device Not Required

A physical device is only needed for the final UX validation step. Every prior layer can be tested without hardware.

### Layer 1 — Build compatibility (no device, immediate)

The most urgent open question (section 5, item 2) is answered by a plain `cargo build`:

```bash
# Add candidate deps to src-tauri/Cargo.toml, then:
cargo build -p desktop-app
```

Version conflicts surface here before writing any integration code.

### Layer 2 — Unit tests (no device)

Test all code that surrounds the device independently: APDU construction, response parsing, compact signature parsing. The key technique is capturing real response bytes once (from the emulator or a physical device) and using them as static fixtures. Keep **BIP-137 fixture tests** separate from **SPS-65** fixtures — the latter must roundtrip to `verify_threshold` with the tagged sighash only.

```rust
#[test]
fn parses_ledger_sign_message_response() {
    // raw bytes captured from device/emulator — fixture, never changes
    let raw = hex!("1f aabb...cc ddee...ff"); // v (1 byte) + r (32) + s (32)
    let sig = parse_ledger_sign_message_response(&raw).unwrap();
    assert_eq!(sig.signature_hex.len(), 128); // 64-byte compact r+s
    assert_eq!(sig.signature_format, SignatureFormat::BitcoinMessage);
}
```

### Layer 3 — The critical integration test (no device)

The gate for **strict SPS-65** is not “does the device return bytes?” but:

> Does the signature **verify with `verify_threshold`** against the **same** `sighash_hex` produced by `compute_sighash` (SPS-65 tagged digest)?

**Acceptance rule:** A Trezor (or Ledger) adapter is **not** production-correct for admin actions unless `verify_threshold(&pubkeys, threshold, &[sig_hex], &sighash.sighash_hex)` succeeds **without** any BIP-137 preimage or alternate message transformation. Signatures from `sign_message` / BIP-137 **must fail** this check when passed the SPS-65 sighash — use that as a negative test once a BIP-137 probe exists.

Baseline today: mock / software `sign_sighash` in [`infrastructure/signing.rs`](../../desktop-app/src-tauri/src/infrastructure/signing.rs) already satisfies this. When hardware adapters exist, swap only the signing step:

```rust
#[tokio::test]
async fn sign_verify_roundtrip_sps65() {
    let sighash = compute_sighash(SEQNO, ACTION_HEX).expect("sighash");

    // Replace with trezor::sign_admin_sps65 / ledger::sign_admin_sps65 when implemented
    let sig = sign_sighash(&SECRET_KEY_HEX, &sighash.sighash_hex).expect("sign");

    let result = verify_threshold(
        &[sig.public_key_hex],
        1,
        &[sig.signature_hex],
        &sighash.sighash_hex,
    )
    .expect("verify");

    assert!(result.valid);
}
```

If this passes with a hardware-backed `sig` and the **device-derived** pubkey set, the adapter matches SPS-65 verification expectations for that fixture — still run emulator/device layers for UX and firmware edge cases.

### Layer 4 — Emulator tests (no physical device, setup required)

Both Ledger and Trezor provide official emulators that run the real device firmware as a local process. This is the closest test to a physical device without having one.

**Ledger — Speculos**

Official LedgerHQ emulator. Runs the Bitcoin app firmware. Accepts TCP connections, so `ledger-transport` has a dedicated TCP transport that replaces `TransportNativeHID` in tests.

```bash
docker run --rm -it -p 5000:5000 \
  ghcr.io/ledgerhq/speculos \
  --model nanos apps/bitcoin.elf --display headless
```

```rust
// Swap transport for tests — all other code is identical
#[cfg(test)]
let transport = TransportTcp::new("127.0.0.1:5000")?;
#[cfg(not(test))]
let transport = TransportNativeHID::new(&hidapi)?;

let client = BitcoinClient::new(transport);
// BIP-137 POC only — strict SPS-65 needs the Ledger tx/PSBT path aligned with verify_threshold
let sig = client.sign_message(&path, &sighash_bytes)?;
```

Speculos supports `--automation` mode which auto-approves on-screen confirmations — no button presses needed in CI.

**Trezor — official emulator**

The `trezor/trezor-firmware` repo ships an emulator binary for each device model. `trezor-client` discovers it the same way it discovers a physical device. **Layer 4b** is only meaningful for SPS-65 when the adapter under test uses the **SignTx / PSBT (or equivalent)** path; a roundtrip that still uses `sign_message` validates BIP-137 only, not admin SPS-65.

```bash
# From trezor-firmware repo
./build_emulators.sh
./build/unix/trezord-go
```

The emulator uses test seeds, requires no PIN by default, and auto-approves operations in headless mode — suitable for CI.

### Layer 5 — Physical device (UX validation only)

The physical device is needed for exactly one thing: confirming what the user sees on the device screen and that the approval flow is correct. By this point, the signing code is already verified to produce correct signatures. The device test is a UX test, not a correctness test.

### Testing progression

```
Layer 1: cargo build with new deps          no device  — answers version compatibility
Layer 2: unit tests (fixture-based)         no device  — answers format/parsing correctness
Layer 3: verify_threshold + SPS-65 sighash  no device  — mandatory gate; mock or software key first
Layer 4a: Speculos + verify_threshold       no device  — Ledger SPS-65 path (not BIP-137-only)
Layer 4b: Trezor emu + verify_threshold     no device  — Trezor SPS-65 path (not sign_message-only)
Layer 5: physical device                    device     — validates UX (screen, buttons)
```

Layers 1–4 can run in CI. Layer 5 is a manual check before shipping.

---

Sources:
- [ledger-transport-hid — crates.io](https://crates.io/crates/ledger-transport-hid)
- [ledger-apdu — crates.io](https://crates.io/crates/ledger-apdu)
- [LedgerHQ/ledger-rust — GitHub](https://github.com/LedgerHQ/ledger-rust)
- [Zondax/ledger-rs — GitHub](https://github.com/Zondax/ledger-rs)
- [ledger_bitcoin_client — crates.io](https://crates.io/crates/ledger_bitcoin_client)
- [LedgerHQ/app-bitcoin-new — GitHub](https://github.com/LedgerHQ/app-bitcoin-new)
- [trezor-client — crates.io](https://crates.io/crates/trezor-client)
- [trezor-client 0.1.5 — docs.rs](https://docs.rs/crate/trezor-client/latest)
- [trezor-connect-rs — crates.io](https://crates.io/crates/trezor-connect-rs)
- [hidapi — crates.io](https://crates.io/crates/hidapi)
- [tauri-plugin-hid — crates.io](https://crates.io/crates/tauri-plugin-hid)
