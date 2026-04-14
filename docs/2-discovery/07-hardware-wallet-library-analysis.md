# Hardware Wallet Library Analysis — Rust Adapter Implementation

## Overview

This document evaluates the available Rust libraries for implementing hardware wallet communication (Ledger and Trezor) in the Tauri native process. The goal is to select the right stack before implementing the production path described in `06-hardware-wallet-architecture.md`.

**Signing context:** The Alpen Multisig app signs admin action sighashes using the Bitcoin Signed Message format (BIP-137 style ECDSA). Devices hash the payload as:

```
SHA256d( "\x18Bitcoin Signed Message:\n" + varint(len) + sighash_hex )
```

This is already working from the POC JS adapters. The library decision is about which Rust stack replicates this in the native process.

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
- **Signing support:** `sign_message(path, message)` — produces a Bitcoin Signed Message signature (BIP-137). This is exactly what the current POC JS adapter does via `btc.signMessage()`.
- **Maintenance:** Actively maintained by LedgerHQ alongside the Bitcoin app firmware.

```rust
// Approximate API
let transport = TransportNativeHID::new(&hidapi)?;
let client = BitcoinClient::new(transport);
let sig = client.sign_message(&path, message_bytes)?;
// sig: (v, r, s) — maps directly to current SignSighashResult format
```

**Pros:**
- Official — protocol compatibility guaranteed
- `sign_message` maps 1:1 to the current JS adapter behavior
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

## 3. Trezor

### 3.1 `trezor-client`

- **Crate:** [`trezor-client`](https://crates.io/crates/trezor-client)
- **What it is:** The most complete Rust client for Trezor. Communicates with the device via USB HID using the Trezor protobuf protocol (`trezor-common` protos). Supports Bitcoin, Ethereum, and others.
- **Signing support:** `sign_message(path, coin, message)` — produces a Bitcoin Signed Message signature, same format as TrezorConnect's `signMessage` used in the POC.
- **Version:** 0.1.5 (docs build failed; 0.1.4 docs are available and stable)
- **Maintenance concern:** Not officially maintained by Trezor. Community-maintained. The docs build failure on 0.1.5 is a yellow flag.

```rust
let mut trezor = Trezor::find_device(None)?;
trezor.init_device(None)?;
let sig = trezor.sign_message(
    path.clone(),
    "Bitcoin",
    message_bytes,
)?;
// sig: MessageSignature { address, signature }
```

**Pros:**
- Full Bitcoin sign_message support out of the box
- Does not require implementing the protobuf protocol manually

**Cons:**
- **Not officially maintained by Trezor** — community crate
- 0.1.5 docs build failure suggests potential instability
- Large protobuf dependency tree
- Potential version conflicts with workspace pins

### 3.2 `trezor-connect-rs`

- **Crate:** [`trezor-connect-rs`](https://crates.io/crates/trezor-connect-rs)
- **What it is:** Wraps Trezor communication through a **Deno JavaScript bridge script** (`functions-with-trezor.js`). Rust shells out to Deno, which calls TrezorConnect JS, which communicates with the device.
- **Assessment:** Adds a Deno runtime dependency to a Tauri app. Architecturally worse than the current JS POC approach — still has JS in the signing path, just inverted.

**Verdict: Not viable.**

### 3.3 Custom implementation via `hidapi` + protobuf

- **What it is:** Implement the Trezor USB protocol directly using `hidapi` for transport and `prost` (or `protobuf`) for message serialization using `trezor-common` `.proto` files.
- **Reference:** The Trezor protocol is open and well-documented in [`trezor/trezor-common`](https://github.com/trezor/trezor-common).

```
Device flow:
  hidapi open device → write Initialize message → read Features response
  → write SignMessage (path, message, coin) → read MessageSignature
```

**Pros:**
- No dependency on unmaintained community crates
- Full control over protocol version and error handling
- Can target exactly the messages needed (no unused protobuf bloat)

**Cons:**
- Significant implementation work (proto compilation, message framing, chunked HID packets)
- Must maintain the proto files if Trezor firmware changes

---

## 4. Comparison Matrix

| | `ledger_bitcoin_client` | `ledger-transport-hid` + manual | `trezor-client` | Custom protobuf |
|---|---|---|---|---|
| **Vendor** | Ledger | Ledger | Trezor | Trezor |
| **Level** | High (Bitcoin-specific) | Low (raw APDU) | High (protobuf RPC) | Low (raw HID + proto) |
| **`sign_message` support** | Yes (built-in) | Manual implementation | Yes (built-in) | Manual (proto codegen) |
| **Official / maintained** | Yes (LedgerHQ) | Yes (Zondax, stable) | No (community) | N/A (you own it) |
| **Legacy app support** | No | Yes | Yes | Yes |
| **Implementation effort** | Low | High | Low | Very high |
| **Dependency risk** | bitcoin version conflict? | Low | Docs build failure | Low (you control protos) |
| **Tauri compatibility** | Via `hidapi` | Via `hidapi` | Via `hidapi` | Via `hidapi` |

---

## 5. Open Questions Before Deciding

| # | Question | Impact |
|---|----------|--------|
| 1 | Do target users run the **legacy** Ledger Bitcoin app or the **new** one? Legacy app is common on older Nano S devices. | Determines if `ledger_bitcoin_client` is sufficient or raw APDU is needed. |
| 2 | Does `ledger_bitcoin_client` compile cleanly with `bitcoin = "0.32.6"` and `secp256k1 = "0.29.1"`? | Could force a version bump or manual patching in the workspace. |
| 3 | Is `trezor-client 0.1.4` stable enough for production, or is the custom protobuf path safer long-term? | Determines build vs. buy for Trezor. |
| 4 | Future Taproot signing (PSBT flow) — is this in scope? | Changes the Ledger API surface significantly (PSBT vs message signing are separate APIs). |

---

## 6. Recommended Approach

Given the current state — BIP-137 message signing, Tauri native process, existing workspace pins — the lowest-risk path is:

**Ledger:** `ledger_bitcoin_client` as the primary option. Run a quick build test to confirm `bitcoin`/`secp256k1` version compatibility with the workspace. Fall back to manual APDU via `ledger-transport-hid` only if the new Bitcoin app requirement is a blocker (legacy device users).

**Trezor:** Start with `trezor-client 0.1.4`. If stability or maintenance proves problematic during implementation, the custom protobuf path is the fallback — the protocol is simple enough (two messages: `SignMessage` → `MessageSignature`) that a targeted custom implementation is feasible without implementing the full client.

**HID transport:** `hidapi` as a direct dependency. Both `ledger_bitcoin_client` and `trezor-client` use it internally — no additional setup.

**Tauri integration pattern** (same for both):

```rust
#[tauri::command]
async fn sign_action(
    vendor: String,
    action_hex: String,
    seqno: u64,
) -> Result<SignResult, String> {
    let sighash = compute_action_sighash(&action_hex, seqno)?;   // already exists
    match vendor.as_str() {
        "ledger" => ledger::sign_message(&sighash),
        "trezor" => trezor::sign_message(&sighash),
        _ => Err("Unknown vendor".into()),
    }
}
```

This POC should answer question 2 (version compatibility) and question 3 (trezor-client stability) before committing to the full implementation.

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

Test all code that surrounds the device independently: APDU construction, response parsing, signature format conversion. The key technique is capturing real response bytes once (from the emulator or a physical device) and using them as static fixtures.

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

The most important question is not "does the device sign?" — the POC already confirmed that. The question is:

> Does the signature produced by the Rust adapter pass the Rust verifier (`verify_bitcoin_message_threshold`)?

This roundtrip test can be written today using the mock adapter, and reused verbatim for Ledger and Trezor once the adapters are implemented:

```rust
#[tokio::test]
async fn sign_verify_roundtrip() {
    let sighash = compute_action_sighash(ACTION_HEX, SEQNO).unwrap();

    // Step 2 is the only thing that changes between mock, Ledger, and Trezor
    let sig = sign_with_mock(&sighash).await.unwrap();

    let result = verify_bitcoin_message_threshold(
        &[sig.public_key_hex],
        1,
        &[sig.signature_hex],
        &sighash,
    ).unwrap();

    assert!(result.valid);
}
```

When a Ledger or Trezor adapter is implemented, replace `sign_with_mock` with the new adapter. If the test passes, the adapter is correct — no device needed for that confidence.

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
let sig = client.sign_message(&path, &sighash_bytes)?;
```

Speculos supports `--automation` mode which auto-approves on-screen confirmations — no button presses needed in CI.

**Trezor — official emulator**

The `trezor/trezor-firmware` repo ships an emulator binary for each device model. `trezor-client` discovers it the same way it discovers a physical device.

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
Layer 3: roundtrip with mock adapter        no device  — establishes the test baseline
Layer 4a: roundtrip with Speculos           no device  — validates Ledger adapter end-to-end
Layer 4b: roundtrip with Trezor emulator    no device  — validates Trezor adapter end-to-end
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
