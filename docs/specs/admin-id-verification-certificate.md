# Spec: Admin ID Verification Certificate (G8)

**Program:** Admin ID rework — loop G8 of 3 (G7 = Admin ID is an address, done; G9 = device QA + compliance close-out)
**PRD source:** [`docs/0-prd/06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) §3.c.i, §4.a
**Depends on:** [`admin-id-as-bitcoin-address.md`](./admin-id-as-bitcoin-address.md) (G7, merged at `e9ca99e`) — the certificate signs the address
**UI contract:** `docs/0-prd/assets/admin-id-verification-certificate-{unsigned,signed,copied}.jpg` (normative)
**Issues:** #409 (closed by this loop), #410 (confirmed again pre-sign-in)
**Compliance matrix:** [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md) §4.2

## Objective

PRD 06 §3.c.i asks for a `Verify` affordance next to the Admin ID — available **before** sign-in and
**after** login — that signs the Admin ID with its own private key, produces an **Admin ID
Verification Certificate**, offers it for copying, and lets the signer confirm the same Admin ID on
the hardware signer screen. The requirement is explicit that **anyone holding the certificate must
be able to independently derive the compressed public key** of that Admin ID.

This is what finally closes #409. The July objection was that a device screen cannot be the source
of truth, because no supported signer renders a raw compressed public key. G7 made the Admin ID the
address again, so the device now shows the Admin ID *itself*; the certificate pays off the other
half of the debt — it hands the subprotocol the compressed public key, **recovered from a
signature**, without asking firmware for something it cannot do.

The signature is not a new signing path. Trezor, Ledger and the mnemonic dev signer already sign
`signed_msg_hash(message)` and normalise to 65 bytes `[r||s||recid]` hex (`trezor.rs:545-548`,
`ledger.rs:455-467`, `signing.rs:166-186`), and the wallet adapters already expose exactly that
through one port (`signSighash(message)` with no signing context, `signatureFormat:
'bitcoin-message'` — `wallet/hw-adapter.ts:65-79`, `wallet/mnemonic-adapter.ts:60-75`). **The
certificate is a re-encoding, plus a verification, of a signature the app already produces.**

## Scope

**In scope:**

- A Rust module that renders the certificate message, encodes the certificate, and **self-verifies
  it** before it can be shown: recover the public key, re-derive the P2WPKH address, and require it
  to equal the Admin ID. The app never surfaces a certificate it did not verify.
- Two Tauri commands (`admin_id_certificate_message`, `build_admin_id_certificate`) so both the
  displayed message and the encoded certificate come from one Rust source.
- The two-step modal from the wireframes, mounted in **both** placements: next to `AdminIdRow`
  (post-login) and next to `ConnectAdminIdCard` (pre-sign-in, #410).
- Copying **message + signature** as a two-line block (D1).
- Step 2 = verify-on-device for the Admin ID (P2WPKH), **disabled with an explanation** in mnemonic
  sessions (D3).
- Tests: Rust unit + recovery round-trip; `architecture.test.ts` Rule 10 pinning the wireframe
  literals and forbidding certificate assembly outside Rust; a WebDriver spec walking the three
  wireframe states.
- Docs: matrix §4.2 **PARTIAL → PASS**; rename `sign_admin_sps65_binding`, which has been signing
  non-SPS-65 messages since session auth landed.

**NOT in scope (explicitly deferred):**

- **Device QA on real Trezor/Ledger hardware and Speculos**, including the Ledger message-vs-hash
  risk recorded in `lib/device-copy.ts:53-55` → **G9**. G8 ships the flow and the app-side proof;
  what a given device screen actually renders is measured in G9 before anything is promised in
  client docs.
- **Everything Payout Administrator**, including the P2TR Admin ID at `m/86'/0'/73'/0/0`
  (PRD 06 §3.b.ii.1). Recorded in the matrix as **DEFER**, destination: the `block_payouts` program.
- **A QR code for the certificate or for the Admin ID.** Rule 9's QR prohibition stays: the Admin ID
  must never receive funds.
- **Persisting certificates.** The certificate lives in the modal's state and on the clipboard. It
  carries no secret, but it is also not something the app stores or re-displays across sessions.

## Technical Design

### Certificate format (settled in B0, measured — not reasoned about)

```
message      Admin ID: bc1q5lvgztw04yl7addhh63yry2tsuw5vxj9fxadlp
digest       signed_msg_hash(message)          // "\x18Bitcoin Signed Message:\n" || varint || msg
signature    base64( [27 + 4 + recid] || r[32] || s[32] )
copied block <message> "\n" <signature>
```

The header is **31 + recid** — Bitcoin Core's "compressed P2PKH" byte. Three reasons, all checked:

1. It is the byte the normative wireframe shows: the sample `IP3DFVS6rx…` decodes to a first byte
   `0x20` = 31 + recid(1).
2. `bitcoin::sign_message::MessageSignature` produces and parses it with no encoding code of ours.
3. It verifies in Bitcoin Core (see the gate below). The `39 + recid` native-segwit header of
   BIP-137 — what the devices themselves return before our adapters throw the type bits away — was
   measured to verify in Core too, but it does not match the wireframe.

**Recorded imprecision:** a strict BIP-137 reader will conclude from a `31 + recid` header that the
signature belongs to a P2PKH address, while the message names a bech32 one. It changes nothing about
the recovered key — which is what §3.c.i actually requires — and it is what the wireframe pinned.
Noted here so a future reader does not rediscover it as a bug.

### The falsifiable gate for this loop

The master plan's gate (`bitcoin-cli verifymessage <admin-id bc1q…> …` → `true`) is **unreachable**:
Bitcoin Core v28.1 rejects every segwit destination in `signmessage`/`verifymessage` with
`Address does not refer to key`. It was never a property of our encoding. Measured replacement,
which exercises the same digest, header and base64 through an external tool:

```
verifymessage <P2PKH address derived from the same pubkey> <certificate> "Admin ID: bc1q…"  → true
verifymessage <same>                                       <certificate> "Admin ID: tampered" → false
```

If the first call does not return `true`, the encoding is wrong and the loop stops at B1.

### Production code

| File | Responsibility |
|---|---|
| `src-tauri/src/infrastructure/admin_id_certificate.rs` **(new)** | Render the signed message, encode a 65-byte `[r\|\|s\|\|recid]` signature into the certificate, and verify a certificate against an Admin ID by recovering the key and re-deriving the P2WPKH address. |
| `src-tauri/src/commands/admin_id_certificate.rs` **(new)** | Two Tauri commands: `admin_id_certificate_message(admin_id)` and `build_admin_id_certificate(admin_id, signature_hex, network)`. No device access — the signing already happened through the wallet adapter. |
| `src-tauri/src/commands/invoke.rs` | Register both commands in the two handler lists. |
| `desktop-app/src/api/admin-wallet.ts` | Typed bridge for both commands, in the existing `ApiResult` shape. |
| `desktop-app/src/domain/admin-wallet/components/admin-id-certificate-modal.tsx` **(new)** | The two-step modal exactly as the wireframes fix it: title, Step 1 (help text, read-only message box, result box, `Sign` → `✅ Signed` chip, copy icon inside the signature box, `Copied to clipboard`), Step 2 (help text, `Verify`). Presentation only. |
| `desktop-app/src/domain/admin-wallet/hooks/use-admin-id-certificate.ts` **(new)** | The `idle → waiting → signed → error` state machine: ask Rust for the message, sign it through the session's wallet adapter, hand signature + Admin ID back to Rust, hold the verified certificate. |
| `desktop-app/src/domain/admin-wallet/model/admin-id-certificate.ts` **(new)** | The wireframe copy literals and the two-line copied-block builder — the single audited source both surfaces and the guard read. |
| `desktop-app/src/domain/admin-wallet/components/admin-id-row.tsx` | Opens the modal from a `Verify` button; the inline `VerifyOnDeviceButton` **moves into** the modal's Step 2 (one affordance, not two). |
| `desktop-app/src/domain/connect-wallet/components/connect-admin-id-card.tsx` | Same modal, pre-sign-in, with no authenticated session. |

### Where the signature comes from (D5 holds, without a second signing path)

The plan sketched one command that both signs and encodes. It would have to re-implement the
HW-vs-mnemonic dispatch that `wallet/create-wallet-adapter.ts` already owns, and it would push the
mnemonic through a second IPC surface. Instead:

```
modal → admin_id_certificate_message(adminId)          [Rust owns the literal]
      → adapter.signSighash(message)                   [existing port: Trezor | Ledger | mnemonic]
      → build_admin_id_certificate(adminId, sigHex)    [Rust: encode + self-verify, or reject]
```

**All cryptography stays in Rust** — the frontend never hashes, encodes or recovers anything, and no
crypto dependency is added to the frontend. The frontend's only job is to move three strings.

### Dependencies

One cargo **feature**, no new crate: root `Cargo.toml:33` gains `base64` on the `bitcoin` pin
(`features = ["serde", "base64"]`), which is what `MessageSignature::to_base64` / `from_base64` sit
behind. Everything else — `signed_msg_hash`, recovery, P2WPKH derivation — is already in use.

### Copy literals (transcribed from the wireframes, not invented)

| Literal | Where |
|---|---|
| `Generate Admin ID Verification Certificate` | modal title |
| `Step 1. Sign Admin ID` | step heading |
| `Click the "Sign" button and confirm the signature on your hardware signer to digitally sign your Admin ID and generate your Admin ID Verification Certificate.` | Step 1 help |
| `Waiting for signature to generate Admin ID Verification Certificate...` | result box, idle/waiting |
| `Sign` / `Signed` | button, then chip (the button is **replaced**, not disabled) |
| `Copied to clipboard` | right of the `Signed` chip |
| `Step 2. Verify Admin ID` | step heading |
| `Click the "Verify" button to compare and verify that the Admin ID (in bitcoin address format) that appears on your hardware signer screen matches the signed Admin ID shown above.` | Step 2 help |
| `Verify` | Step 2 button |

Three properties the wireframes fix and the implementation must not "improve":

- The copy is **device-agnostic** ("your hardware signer", never a brand) — #24/#18.
- **Step 2 does not depend on Step 1**: `Verify` is enabled in all three frames.
- **No visible close button**: the modal closes through `AccessibleDialog`'s Escape / overlay click,
  like every other modal in the repo.

The ellipsis in the waiting literal is written as three ASCII dots (`...`), matching the wireframe
and the repo's ASCII-only copy convention.

### Mnemonic sessions (D3)

`AdminIdRow` already receives `verify?: AdminIdVerifyContext` only for hardware sessions. Absent
context means a mnemonic session: **Step 1 works** (the mnemonic adapter signs the same message and
the certificate is just as verifiable), and **Step 2 renders disabled with an explanation** —
there is no device screen to compare against. The modal keeps the same shape in every mode.

### Architecture compliance (`architecture.test.ts` — new Rule 10)

- `admin-id-certificate-modal.tsx` must render every literal in the table above.
- The modal and the connect card must both mount `AdminIdCertificateModal`.
- No file under `desktop-app/src` may contain `Bitcoin Signed Message`, a base64 header constant, or
  any signature assembly — the certificate is built in Rust only.
- `admin-id-row.tsx` must not render `VerifyOnDeviceButton` directly any more (it moved into Step 2).

### Test helpers

Signature fixtures for the Rust tests are generated inside `#[cfg(test)]` from a fixed secret key,
the way `challenge_verifier.rs:104-116` already does it. No fixture builder is exported, and no test
helper is registered as a Tauri command.

## Test Cases

**Rust — `admin_id_certificate.rs`**

1. *Happy path*: message renders as `Admin ID: <address>` exactly.
2. *Happy path*: a valid 65-byte signature encodes to a base64 certificate whose first byte is
   `31 + recid`.
3. *Round-trip (the requirement)*: the compressed public key recovered from the certificate alone
   re-derives the Admin ID byte for byte.
4. *Self-verification*: a signature made over a **different** message is rejected, not encoded.
5. *Self-verification*: a signature made by a **different key** is rejected.
6. *Edge*: a signature that is not 65 bytes, or not hex, is a typed error, never a panic.
7. *Edge*: an Admin ID that is not a P2WPKH address is rejected before any signing round trip.
8. *Stability*: the message format is pinned, like `render_challenge_message_format_is_stable` —
   a stray space or a changed prefix breaks every certificate ever issued.

**Frontend**

9. `architecture.test.ts` Rule 10, as above.
10. The copied block is exactly `<message>\n<signature>` — one guard on the builder.

**End-to-end (WebDriver, `admin-id-certificate.e2e.js` + `test:e2e:admin-id-certificate`)**

11. Post-login: `Verify` opens the modal in the unsigned state, showing the Admin ID in the message
    box and the waiting literal in the result box.
12. `Sign` (mnemonic session) replaces the button with the `Signed` chip and puts a base64
    certificate in the result box.
13. The copy icon yields `Copied to clipboard`, and the clipboard holds both lines.
14. Step 2 is present and disabled-with-reason in the mnemonic session.
15. Pre-sign-in: the same modal opens from the connect card, before any membership check.

**External verification (the loop gate, run by hand and recorded in the state file)**

16. `bitcoin-cli verifymessage <P2PKH from the same key> <certificate> "Admin ID: <admin-id>"` →
    `true`, and `false` for a tampered message.

## Module structure

- `infrastructure/admin_id_certificate.rs` — *one sentence*: turns an Admin ID and a raw recoverable
  signature into a verified, base64-encoded certificate, and refuses everything else.
- `commands/admin_id_certificate.rs` — *one sentence*: exposes that module over IPC, with no device
  or wallet state of its own.
- `model/admin-id-certificate.ts` — *one sentence*: holds the wireframe copy and the copied-block
  shape for the two surfaces that render them.
- `hooks/use-admin-id-certificate.ts` — *one sentence*: sequences message → signature → certificate
  and exposes the four states the wireframes show.

Dependency direction: the modal depends on the hook, the hook on the API bridge and the wallet
adapter port, and none of them on each other's internals. The Rust module depends on `bitcoin` only
— not on the hardware adapters, which is what lets its tests run without a device.

## Out-of-scope follow-ups (tracked for the matrix)

| Item | Destination |
|---|---|
| What a Ledger actually renders when signing this message (text vs SHA-256 hash) | G9 · B0 |
| Trezor/Speculos QA evidence for the certificate flow | G9 · B1 |
| Manual playbook for issuing and verifying a certificate | G9 · B2 |
| Admin ID P2TR `m/86'/0'/73'/0/0` for the Payout Administrator | DEFER → `block_payouts` |
| `docs/external/verifying-what-you-sign.md` absorbing the certificate | G9 · B4 |
