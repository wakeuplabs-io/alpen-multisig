# Hardware Wallet Architecture — Options & Decisions

## Overview

This document captures the architectural analysis and decisions around hardware wallet integration in the Alpen Multisig desktop app (Tauri + React + Rust). The central question is: **where should hardware wallet communication and signing logic live** — in the JavaScript WebView layer, or in the native Rust process?

This is a security-critical decision because what the hardware wallet device signs is only as trustworthy as the layer that constructs the payload.

---

## 1. Context

The desktop app runs two distinct processes:

```
┌─────────────────────────────────────────────────────┐
│               Tauri Desktop App                      │
│                                                      │
│  ┌─────────────────────┐   ┌──────────────────────┐  │
│  │  React (WebView)    │   │  Rust (native shell) │  │
│  │                     │   │                      │  │
│  │  - UI rendering     │   │  - IPC commands      │  │
│  │  - user interaction │   │  - sighash compute   │  │
│  │  - action intent    │   │  - session tokens    │  │
│  │                     │   │  - device access     │  │
│  └──────────┬──────────┘   └──────────────────────┘  │
│             │  Tauri IPC (invoke / tauri::command)    │
└─────────────┼───────────────────────────────────────┘
              │
```

Key property: the Rust shell is a **trusted native process**. The WebView is a sandboxed browser context — less trusted, reachable by any JS executing inside it.

The signing model (SPS-65) requires:

```
sighash = SHA256( SHA256(tag) || seqno_be_bytes(8) || sighash_payload )
tag     = "strata/admin/<type_name>"
```

The hardware wallet signs this sighash. **Whoever constructs the sighash controls what the device signs.**

---

## 2. Option A — JS Adapters in the WebView (current POC approach)

```
React (WebView)
  ├── Ledger adapter  ──── @ledgerhq/hw-transport-webhid ──▶ Ledger device
  ├── Trezor adapter  ──── @trezor/connect-web ────────────▶ Trezor device
  ├── Mnemonic adapter ─── Tauri IPC ──▶ Rust ──────────────▶ (sign in Rust)
  └── Mock adapter ─────── Tauri IPC ──▶ Rust ──────────────▶ (sign in Rust)
```

**Current signing flow for hardware wallets:**

```
JS: tauriCall('compute_action_sighash', { seqno, actionHex })
        ↓ returns sighash_hex back to WebView
JS: adapter.signSighash(sighash_hex)   ← sighash now in WebView memory
        ↓ JS SDK sends sighash to device
Device: signs
        ↓
JS: returns { publicKeyHex, signatureHex }
```

### Pros

| | |
|---|---|
| Fast to prototype | JS SDKs (Ledger, Trezor) are mature and well-documented |
| No Rust HID code | WebHID / TrezorConnect handle protocol details |
| Usable in browser | Not bound to the native process |

### Cons

| | |
|---|---|
| **Sighash transits WebView** | Rust computes it correctly, but the result is returned to JS before going to the device. JS code (or any injected script) has a window to intercept or alter it. |
| **Inconsistent trust boundary** | Mnemonic and mock signing happen in Rust (safe). Hardware wallet signing happens in JS (unsafe). Same `WalletAdapter` interface, different security guarantees. |
| **Supply chain exposure** | Ledger and Trezor JS SDKs pull in large dependency trees. A compromised npm package executing in the WebView could intercept the sighash or the resulting signature before it reaches the proposal. |
| **No payload validation gate** | The Rust layer has no opportunity to validate the payload before it is sent to the device. |

---

## 3. Option B — Signing in Rust, WebView as Intent Layer (recommended production path)

```
React (WebView)
  └── sends intent only: { vendor, action_payload, seqno }
        ↓ Tauri IPC
Rust (native shell)
  ├── computes sighash from canonical SPS-65 rules
  ├── validates payload structure before device contact
  └── communicates with device via native HID
        ↓ USB/HID
  Hardware wallet device: signs Rust-constructed sighash
        ↓
Rust: returns { pubkey_hex, sig_hex } to WebView
```

**Signing flow:**

```
JS: tauriCall('sign_action', { vendor, actionHex, seqno })
        ↓ full payload goes to Rust, sighash never returns to WebView
Rust: compute_sighash(actionHex, seqno)       ← SPS-65 canonical
Rust: validate_payload_structure(actionHex)   ← hygiene gate
Rust: device.sign(sighash)                    ← native HID
        ↓
JS: receives { pubkey_hex, sig_hex }
```

### Pros

| | |
|---|---|
| **Sighash stays in Rust** | The WebView never sees the intermediate sighash. It sends intent and receives a completed signature. |
| **Uniform trust model** | All four vendors (mock, mnemonic, Ledger, Trezor) sign under the same security boundary. |
| **Payload validation gate** | Rust can reject malformed or unexpected payloads before the device is ever contacted. |
| **No JS SDK supply chain** | Ledger/Trezor npm packages are removed from the WebView. HID communication uses Rust crates (`hidapi`). |
| **Aligns with POC 2 decision** | Option B mirrors the backend communication decision (React → IPC → Rust → HTTP), which was already chosen for session tokens. Same reasoning applies here. |

### Cons

| | |
|---|---|
| Rust HID implementation required | Must implement Ledger APDU protocol and Trezor protobuf protocol in Rust, or use Rust crates. |
| Device discovery in Rust | USB device enumeration, reconnection, and error handling move to Rust. |
| Address display flow changes | `connect()` / `getAddress()` also move to IPC; JS SDKs are fully replaced. |

---

## 4. Signing Security — The Core Argument

The risk in Option A is not that Ledger or Trezor JS SDKs are malicious. The risk is that the **sighash crosses a trust boundary unnecessarily**.

```
Option A (today):
  Rust computes sighash ──▶ sighash_hex returned to WebView ──▶ JS passes to device

Option B (production):
  Rust computes sighash ──▶ Rust passes directly to device    (WebView never sees it)
```

In Option A, there is a window between Rust returning `sighash_hex` and the device receiving it. Any code executing in the WebView during that window — including third-party JS — could observe or tamper with it. In a governance signing application where a single compromised signature can enact a protocol change, this window should not exist.

The hardware wallet's security guarantee is: *you see what you sign on the device screen*. That guarantee holds only if the payload reaching the device was constructed by a trusted layer. Routing through the WebView weakens the trust chain.

---

## 5. Decision

**Option B is the target production architecture.**

The WebView is an intent layer only. Hardware wallet communication and sighash construction belong in Rust.

**Rationale:**
1. Consistent with the POC 2 decision — session tokens stay in Rust, sighashes stay in Rust.
2. The `WalletAdapter` interface (`connect`, `signSighash`) stays unchanged in JS. Only the Ledger and Trezor implementations change: from JS SDK calls to Tauri IPC wrappers.
3. The current POC approach (Option A) is acceptable for development and testing. Migration requires implementing Rust HID commands, not changing the JS interface.

---

## 6. Migration Path — POC to Production

The current code is structured to make this migration clean. The `WalletAdapter` interface is the stable boundary.

| Layer | POC (today) | Production |
|-------|-------------|------------|
| `mock-poc-adapter.ts` | Tauri IPC → Rust | No change |
| `mnemonic-poc-adapter.ts` | Tauri IPC → Rust | No change |
| `ledger-poc-adapter.ts` | JS WebHID → device | Tauri IPC → Rust HID → device |
| `trezor-poc-adapter.ts` | JS TrezorConnect → device | Tauri IPC → Rust HID → device |
| Tauri commands | `compute_action_sighash`, `sign_action_sighash` | Add `sign_action` (compute + sign in one call) |
| Rust HID | — | `hidapi` crate, Ledger APDU, Trezor protobuf |

**Step 1 (no interface change):** Add a `sign_action` Tauri command that takes `(vendor, action_hex, seqno)` and returns `(pubkey_hex, sig_hex)`. Internally it calls `compute_action_sighash` and routes to the appropriate device handler.

**Step 2:** Replace the body of `createLedgerPocAdapter` and `createTrezorPocAdapter` to call `tauriCall('sign_action', ...)` instead of using the JS SDKs.

**Step 3:** Remove `@ledgerhq/hw-transport-webhid`, `@ledgerhq/hw-app-btc`, and `@trezor/connect-web` from the frontend.

The `useWallet` hook and all React components are unaffected.

---

## 7. Open Questions

| # | Question | Status |
|---|----------|--------|
| 1 | Which Rust crate handles Ledger APDU? (`ledger-transport-hid`, `ledger-apdu`, or custom) | Open |
| 2 | Which Rust crate handles Trezor protobuf? (`trezor-client` or custom) | Open |
| 3 | Does the device need to show a human-readable summary on screen, or just a hex sighash? Determines whether to use `signMessage` vs `signPsbt`. | Open |
| 4 | For the address display flow (`connect()`) — do JS SDKs stay for this step only, or does device discovery also move to Rust? | Open |
