# POC-5 Findings — Trezor & Hardware Wallet Limits (Desktop / Tauri)

## Overview

This document consolidates **POC-5 implementation results** (Trezor over HID, Tauri IPC, synthetic PSBT path) with **ecosystem research** on consumer hardware wallets (Trezor, Ledger, Coldcard, BitBox02, Jade, HWI). The goal is a single reference for **protocol alignment (SPS-65 admin ECDSA)**, **what was actually proven in code**, and **open questions for Alpen** plus **implementation options**.

### Sources

- **Trezor driver** — [`desktop-app/src-tauri/src/infrastructure/hw_wallet/trezor.rs`](../../desktop-app/src-tauri/src/infrastructure/hw_wallet/trezor.rs)
- **Signing / verify helpers** — [`desktop-app/src-tauri/src/signing.rs`](../../desktop-app/src-tauri/src/signing.rs) (e.g. `verify_threshold`, `p2wpkh_segwit_sighash_hex`)
- **Shared types** — [`desktop-app/src-tauri/src/infrastructure/hw_wallet/mod.rs`](../../desktop-app/src/infrastructure/hw_wallet/mod.rs)
- **Tauri commands** — [`desktop-app/src-tauri/src/commands/hw_wallet.rs`](../../desktop-app/src-tauri/src/commands/hw_wallet.rs)
- **TypeScript adapter** — [`desktop-app/src/wallet/trezor-poc-adapter.ts`](../../desktop-app/src/wallet/trezor-poc-adapter.ts)
- **Wallet types** — [`desktop-app/src/wallet/types.ts`](../../desktop-app/src/wallet/types.ts)
- **Integration test binary** — [`desktop-app/src-tauri/src/bin/trezor_test.rs`](../../desktop-app/src-tauri/src/bin/trezor_test.rs)
- **Spec** — [`docs/specs/poc5-trezor-hw-wallet-integration.md`](../specs/poc5-trezor-hw-wallet-integration.md)
- **Protocol** — SPS-65 (admin digest), BIP-137 (Bitcoin Signed Message), BIP-143 (SegWit ECDSA sighash)

---

## 1. Protocol reminder (why HW is constrained)

**Strata / Alpen admin multisig (SPS-65)** expects verification roughly as: a **32-byte digest** built from protocol-defined tagging (e.g. tagged SHA256 over `seqno ‖ payload`), then **ECDSA secp256k1** over that digest using keys tied to product derivation (e.g. **BIP86 Taproot** `m/86'/0'/73'/0/n` in product discussions).

**Important:** the verifier uses `Message::from_digest_slice` / equivalent on **that exact 32 bytes**, not on:

- the preimage of **Bitcoin Signed Message** (magic string `\x18Bitcoin Signed Message:\n` + compact sizes + payload, then SHA256 twice in typical stacks), nor
- the **BIP143 / BIP341 transaction sighash** of some PSBT, unless the **chain** is changed to verify that instead.

Consumer Bitcoin apps on Trezor/Ledger **do not** expose “sign this arbitrary 32-byte digest with my Bitcoin key at path P”.

---

## 2. What POC-5 validated in code

| Capability                                                 | Status                   | Notes                                                                                                                                                   |
| ---------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| HID session + `init_device`                                | Validated                | `trezor_client::unique(false)`                                                                                                                          |
| Read **compressed pubkey** + address sample                | Validated                | `get_trezor_info` now follows the product default path **`m/86'/0'/73'/0/0`**                                                                           |
| **`sign_tx` on a synthetic PSBT** (“binding” tx)           | Validated                | Admin SPS-65 hex is embedded as **payload in `OP_RETURN`**; Trezor still signs **BIP143 SegWit v0 sighash** of the constructed tx                       |
| Internal check: signature verifies with `verify_threshold` | Validated                | Only when `verify_threshold` is called with the **derived SegWit sighash** (`p2wpkh_segwit_sighash_hex`), **not** with the SPS-65 hex shown to the user |
| **`sign_message` (BIP-137)**                               | POC helper only          | `sign_message_poc_bip137` — **not** wired to production `sign_with_trezor`; incompatible with SPS-65 verification                                       |
| **Taproot `m/86'…` + same ECDSA path as chain**            | Not validated for SPS-65 | `sign_message` firmware rejects Taproot script type for message signing; PSBT binding remains a different signed message than SPS-65 admin digest       |

### Tauri entry points (current)

- `get_trezor_info` → `trezor::connect` (pubkey + address).
- `sign_with_trezor` → `trezor::sign_admin_sps65_binding` (PSBT / `sign_tx`, **not** `SignMessage`).

### Design choices (POC)

- **No private key in app memory** for Trezor paths — signing occurs on device.
- **`Result<T, String>`** on Tauri boundary for POC simplicity (`Serialize` on errors).
- **TypeScript `WalletAdapter`** — vendor switch (mock / mnemonic / trezor / ledger stub).

---

## 3. Architecture (current stack)

```
┌─────────────────────────────────────────────────────────────┐
│  React — trezor-poc-adapter.ts                               │
│    .connect()      → invoke('get_trezor_info')               │
│    .signSighash()  → invoke('sign_with_trezor')              │
└────────────────────────────┬────────────────────────────────┘
                             │ Tauri IPC
┌────────────────────────────▼────────────────────────────────┐
│  commands/hw_wallet.rs                                      │
│    get_trezor_info   → trezor::connect                        │
│    sign_with_trezor → trezor::sign_admin_sps65_binding       │
└────────────────────────────┬────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────┐
│  trezor.rs                                                   │
│    open_trezor / resolve (ButtonRequest loop)                │
│    connect → get_public_key at `m/86'/0'/73'/0/0`            │
│    sign_admin_sps65_binding → build PSBT → sign_tx → compact │
└────────────────────────────┬────────────────────────────────┘
                             │ trezor-client (HID / bridge)
┌────────────────────────────▼────────────────────────────────┐
│  Trezor device / emulator                                    │
└─────────────────────────────────────────────────────────────┘
```

**`resolve()`** still implements the **ButtonRequest → ButtonAck** loop required when the device waits for on-screen confirmation.

---

## 4. Three signing semantics (do not conflate)

### 4.1 BIP-137 / `SignMessage` (Trezor, Ledger, most “sign message” APIs)

Firmware builds a **Bitcoin Signed Message** digest from the **message bytes** (host-supplied payload), **not** from a raw protocol digest chosen as the sole hash input.

Illustrative pipeline (Bitcoin coin on Trezor; details in firmware [`signverify.py`](https://github.com/trezor/trezor-firmware/blob/main/core/src/apps/common/signverify.py)):

1. Serialize: `compact_size(len(header)) ‖ header ‖ compact_size(len(msg)) ‖ msg` with header like `"Bitcoin Signed Message:\n"`.
2. Hash (Bitcoin: typically **double SHA256** of that serialization).
3. ECDSA-sign that hash.

Ledger’s Bitcoin app does the same pattern explicitly (`BSM_SIGN_MAGIC` + varint + message → digest → second SHA256) in [`sign_message.c`](https://github.com/LedgerHQ/app-bitcoin-new/blob/develop/src/handler/sign_message.c).

**Putting the SPS-65 digest in as the “message” still applies BSM wrapping** — the signed hash **≠** the SPS-65 digest the verifier checks.

### 4.2 PSBT / `sign_tx` (BIP143 / BIP341)

The device computes the **transaction sighash** from the **unsigned transaction** and witness/redeem data. The host can place **arbitrary bytes in `OP_RETURN`**, but that **does not** become the ECDSA message hash. The signature is over the **sighash**, always.

**POC binding:** `sign_admin_sps65_binding` commits `admin_digest` in `OP_RETURN` for **human / audit binding**, while the ECDSA output verifies against **`p2wpkh_segwit_sighash_hex(...)`** for that synthetic tx.

### 4.3 Target for on-chain `verify_threshold` (SPS-65)

Verifier expectation: ECDSA over **`Message = admin_digest_32`** (the tagged construction in SPS-65).

**No consumer Trezor/Ledger Bitcoin command documented today matches §4.3** for host-supplied `admin_digest_32` at an arbitrary standard path.

---

## 5. Ecosystem matrix (consumer HW / HWI)

| Vendor / surface            | Raw ECDSA over **host-chosen 32 B** with **Bitcoin app key** at product path? | Typical API                                                                                                                      | Notes                                                                                                                                                                                        |
| --------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Trezor** (One, T, Safe 3) | **No**                                                                        | `SignMessage` → BSM digest; `SignTx` → sighash                                                                                   | Taproot **message** signing historically unsupported / rejected (`SPENDTAPROOT` in `sign_message` path — see [trezor-firmware#1943](https://github.com/trezor/trezor-firmware/issues/1943)). |
| **Ledger** Bitcoin app      | **No**                                                                        | `SIGN_MESSAGE` (`INS 0x10`)                                                                                                      | Same BSM pipeline in firmware (see link above).                                                                                                                                              |
| **Coldcard**                | **No** (documented)                                                           | `ckcc msg` / on-device message sign                                                                                              | BIP-137 style flows ([docs](https://coldcard.com/docs/message-signing/)).                                                                                                                    |
| **BitBox02**                | **No**                                                                        | `btcSignMsg`                                                                                                                     | BSM-style construction in firmware; Taproot message signing historically limited.                                                                                                            |
| **Blockstream Jade**        | **No** documented raw-Bitcoin-digest sign                                     | Wallet / PSBT flows                                                                                                              | PSBT-first in practice.                                                                                                                                                                      |
| **HWI**                     | **No** aggregate API                                                          | `sign_message` = “legacy Bitcoin Core signed message format” ([docs](https://hwi.readthedocs.io/en/latest/usage/api-usage.html)) | `sign_tx(psbt)` only for tx sighashes.                                                                                                                                                       |
| **Ledger Ethereum**         | Partially (dangerous / blind-hash flows)                                      | App-specific                                                                                                                     | **Wrong domain**: SLIP44 / app semantics ≠ Bitcoin `m/86'/0'/73'/…` admin key.                                                                                                               |

**Signature shape:** message APIs often return **65-byte recoverable** style; `verify_threshold` in this repo can consume **compact 64-byte** ECDSA where applicable. The **hard problem** is the **32-byte message**, not R‖S encoding.

---

## 6. Misleading option removed: “PSBT fixes SPS-65”

**PSBT does not**, by itself, make Trezor produce a signature valid for **on-chain SPS-65** over `admin_digest`, unless **Alpen changes verification** to something derived from a **specific Bitcoin sighash** (or a new protocol ties them cryptographically one-to-one).

The POC **does** prove: device + user confirmation + **ECDSA over a deterministic tx sighash** + optional **commitment** of `admin_digest` in the tx for UX/audit.

---

## 7. Known issues and engineering gaps

| ID  | Topic                                          | Severity    | Notes                                                                                                                                              |
| --- | ---------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **Path / cryptography mismatch**               | Product     | Product/default derivation is now **`m/86'/0'/73'/…`**, while PSBT binding still signs a Bitcoin tx sighash, not the SPS-65 admin digest directly. |
| G2  | **Blocking HID inside `async` Tauri handlers** | Engineering | Prefer `spawn_blocking` for Trezor calls (see original POC note in this file’s history).                                                           |
| G3  | **`xpub_or_fingerprint` stub**                 | UX          | Truncated hex display, not a real xpub.                                                                                                            |
| G4  | **PIN / passphrase**                           | UX          | `resolve()` returns errors for PIN/passphrase flows — needs UI product work.                                                                       |

---

## 8. Emulator quick reference

```bash
trezord -e 21324
trezorctl -p udp:127.0.0.1:21324 debug load-device \
  --mnemonic "all all all all all all all all all all all all" \
  --pin "" \
  --passphrase-protection false
cargo run -p desktop-app --bin trezor_test
```

Expect **`bc1p…`** sample addresses when using the product path `m/86'/0'/73'/0/n` in list-address flows.

---

## 9. Relationship to other POCs

- **POC-3** — `sign_sighash` / `verify_threshold` in software over the **real admin digest**.
- **POC-5** — Proves Trezor **transport + PSBT signing + internal consistency**; **does not** prove end-to-end “Trezor signature accepted by Strata verifier for SPS-65 digest” unless the verifier is changed or a separate binding story is accepted.
- **Future HWI / Ledger** — Same **digest** limitation applies unless using **custom app / HSM**.

---

## 10. Questions for Alpen (product + protocol)

Use these to decide whether HW is **identity / ceremony only**, **binding / audit**, or **first-class on-chain admin signer**.

### Protocol / verification

1. **Must** each admin signature be **ECDSA verifiable on-chain** with `Message = SPS-65_digest_32` exactly as today, or is a **two-layer** model acceptable (e.g. on-chain still SPS-65 from software / custody, HW signs a **different** object for audit)?
2. If **binding PSBT** is used: should the **chain / indexer** ever interpret `OP_RETURN` commitment, or is that **off-chain evidence only**?
3. Are admin keys **required** to live at **`m/86'/0'/73'/…`** (BIP86 x-only / Taproot semantics) for display/consistency, while ECDSA uses another representation of the same scalar — and is that **explicitly** specified vs implementation detail?

### Security / UX

4. Is **“blind signing”** of a **32-byte hex** digest on a small screen acceptable for admins if a future HSM/custom path existed — or is **transaction-shaped** confirmation (fees, outputs) mandatory?
5. For **threshold**: must **every** cosigner use the **same** signing semantics (all HW, all software, or mixed)?

### Roadmap / compliance

6. Is **software signing** (secure enclave, OS keychain, air-gapped machine) an acceptable **MVP** for admin SPS-65 while Trezor is used for **address proof / optional second factor**?
7. Is budget assumed for **enterprise HSM** or **Ledger custom Bitcoin app** if HW must match SPS-65 **byte-for-byte**?

---

## 11. Implementation options (for WakeUp / desktop)

| Option                                                               | Delivers SPS-65 `verify_threshold` compatible sig?             | HW role                                                | Effort / risk                                                                 |
| -------------------------------------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------- |
| **A — Software admin signing (current POC-3 style)**                 | **Yes**                                                        | HW optional (not on critical path)                     | Lowest protocol risk; key handling is the main security workstream.           |
| **B — Trezor PSBT binding only**                                     | **No** for SPS-65 message; **yes** for sighash of synthetic tx | Proof of user interaction + commitment of digest in tx | Medium engineering; **requires Alpen acceptance** of binding semantics (§10). |
| **C — BIP-137 message sign**                                         | **No**                                                         | Auth / “prove control of key” off-chain only           | Low; good for challenges **if verifier uses BSM**.                            |
| **D — Custom Ledger app / HSM “sign raw SHA256”**                    | **Yes** (if designed that way)                                 | Full HW admin                                          | High: distribution, audits, per-org firmware policies.                        |
| **E — Protocol change** to verify Bitcoin sighash or BIP-322/BIP-137 | Depends on new spec                                            | Could use retail HW                                    | **Protocol / fork** level; not a desktop-only change.                         |

**Practical recommendation for a Tauri client today:** **A** for on-chain admin signatures that must match SPS-65; **B** or **C** only with **explicit Alpen** agreement on what is being proven where.

---

## 12. Changelog of this document

| Revision          | Summary                                                                                                                                                |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Earlier draft     | BIP-137 vs raw digest, `SPENDADDRESS` note, PSBT suggested as “fix” for ASM without nuance.                                                            |
| **This revision** | Aligns with **`sign_admin_sps65_binding`** + **ecosystem matrix** + **correct PSBT limitation** + **Alpen question set** + **implementation options**. |

---

## 13. References (external)

- [BIP-137](https://github.com/bitcoin/bips/blob/master/bip-0137.mediawiki) — Bitcoin signed message format.
- [Trezor `message_digest` / `sign_message`](https://github.com/trezor/trezor-firmware/blob/main/core/src/apps/common/signverify.py), [`sign_message.py`](https://github.com/trezor/trezor-firmware/blob/main/core/src/apps/bitcoin/sign_message.py).
- [Ledger Bitcoin app `SIGN_MESSAGE`](https://github.com/LedgerHQ/app-bitcoin-new/blob/develop/doc/bitcoin.md), [handler](https://github.com/LedgerHQ/app-bitcoin-new/blob/develop/src/handler/sign_message.c).
- [HWI `sign_message` description](https://hwi.readthedocs.io/en/latest/usage/api-usage.html).
- [Trezor issue — Taproot / message signing consensus](https://github.com/trezor/trezor-firmware/issues/1943).
- [Coldcard message signing](https://coldcard.com/docs/message-signing/).
