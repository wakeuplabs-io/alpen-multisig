# Manual Test Playbook — Admin ID Verification Certificate (G9)

> Required deliverable for [`admin-id-verification-certificate.md`](./admin-id-verification-certificate.md).
> Device behavior is **not** in CI: the certificate's whole point is what the signer reads on the
> hardware signer's screen, and that cannot be asserted from the app. CI covers the encoding, the
> self-verification and the modal states; this playbook covers the screens.
>
> The two automated device specs below do most of this by machine
> (`desktop-app/e2e-webdriver/test/specs/g9-certificate-{ledger,trezor}.qa.js`). Run them first;
> this playbook is what a human follows on **real hardware**, where the emulators cannot speak.

## Scope of this playbook

| # | Flow | Device | Spec ref |
|---|------|--------|----------|
| 1 | Sign the certificate, read the message on the device | Ledger | §3.c.i, §3.b.ii.iv |
| 2 | Sign the certificate, read the message on the device | Trezor | §3.c.i, §3.b.ii.iv |
| 3 | Verify the Admin ID on the device (Step 2) | Ledger / Trezor | §4.2 |
| 4 | Certificate verifies independently, outside the app | — | §3.c.i (public key recovery) |
| 5 | Mnemonic session: Step 2 disabled with a reason | — | D3 |
| 6 | On-device rejection | Ledger / Trezor | error path |

## Prerequisites

- Local regtest stack up and the backend running (`cargo run -p orchestrator-be`).
- Desktop app: `cd desktop-app && npm run tauri dev` — or the built debug binary the WebDriver specs
  use, which is what the automated runs drive.
- A signer connected as **Strata Administrator**. The certificate is reachable **before** sign-in on
  the connect card and **after** login on the wallet panel; both must be exercised.

### Emulators (for a dry run before touching real hardware)

```bash
# Trezor — dockerised emulator, T2B1 / Safe 3
trezor-emu-docker/up.sh          # outside this repo

# Ledger — Speculos
./scripts/ledger-up.sh <path-to-bitcoin_testnet_*.elf>
# desktop-app/.env: LEDGER_SPECULOS_URL=http://localhost:5001
# To watch the real prompts instead of auto-approving: LEDGER_SPECULOS_AUTO_APPROVE=0
```

## Flow 1 — Ledger: sign the certificate (§3.c.i, §3.b.ii.iv)

1. Connect the Ledger and open the certificate modal from the **Verify** control next to the
   Admin ID.
2. Confirm Step 1 shows `Admin ID: <address>` and the certificate box reads the waiting caption.
3. Press **Sign**. On the device, page through the review screens.
   - **PASS:** the device shows `Path 84'/…/0/0`, then `Message (n/m)` pages carrying
     `Admin ID: <address>` in readable text, then `Sign message`. Every character of the address
     must match the app, across the page breaks.
   - **FAIL:** the device shows a **`Message hash`** screen instead of the text. The signer cannot
     read what they are signing, and req. 3.b.ii.iv is not met on that build — record the model and
     Bitcoin app version and stop. See `issues/evidence/G9-B0-LEDGER-MEASUREMENT.md`: on Bitcoin app
     2.4.2 the text is shown, and the hash fallback is triggered by non-printable bytes, so a hash
     here means a different build behaves differently.
4. Approve. The modal shows the **Signed** chip and an 88-character base64 certificate.

## Flow 2 — Trezor: sign the certificate (§3.c.i, §3.b.ii.iv)

Same as Flow 1, with the Trezor's screens:

- **PASS:** the device shows `Signing address` with the Admin ID, then `Confirm message` carrying
  `Admin ID: <address>`. A Trezor renders message text unconditionally, so there is no hash case.
- **FAIL:** the address on `Signing address` differs from the one in the modal.

## Flow 3 — Verify the Admin ID on the device (§4.2)

1. With the certificate signed, press **Verify** in Step 2.
2. The device displays an address for confirmation. Compare it character by character with the
   Admin ID in Step 1.
   - **PASS:** they match, and the app shows "Confirmed the Admin ID on your <device>."
   - **FAIL:** the app shows the **mismatch** state, or the strings differ. A mismatch means the app
     and the device disagree about which key is the Admin ID — stop and escalate; do not treat the
     Admin ID as verified.

> **Read this before running Flow 3 on a Trezor.** The Trezor titles this screen **`Receive
> address`**, because that is the firmware's generic label for showing an address. It is *not* an
> invitation to send funds. **The Admin ID must never receive funds** — it is an identity, which is
> why the app deliberately refuses to render a QR code for it. Ledger titles the same screen
> `Address`.

## Flow 4 — The certificate verifies independently (§3.c.i)

The requirement is that **anyone** holding the certificate can derive the compressed public key of
that Admin ID, without the app.

1. Press the copy control. The clipboard now holds two lines: the message, then the signature.
2. Verify it outside the app. Bitcoin Core cannot check a bech32 address directly
   (`verifymessage` only accepts P2PKH — measured in G8), so verify against the **P2PKH derived from
   the same key**:

   ```bash
   bitcoin-cli verifymessage <p2pkh-from-the-same-key> "<signature>" "Admin ID: <address>"
   ```

   - **PASS:** `true` for the copied message, and `false` when a single character of the message is
     altered.
   - **FAIL:** `true` for a tampered message, or the recovered key does not derive the Admin ID.

## Flow 5 — Mnemonic session: Step 2 disabled with a reason (D3)

1. Connect with seed words (dev builds only) and open the modal.
2. **PASS:** Step 1 signs normally, and Step 2's **Verify** button is **disabled with an
   explanation** beneath it — never hidden. A step that disappears reads as a broken modal.
3. **FAIL:** the step is missing, or the button is enabled and does nothing.

## Flow 6 — On-device rejection

1. Press **Sign**, then reject on the device.
2. **PASS:** the modal shows the error next to Step 1's control, keeps the modal open, and the
   Sign button returns to a pressable state. No certificate is shown.
3. **FAIL:** a certificate appears anyway, the modal closes silently, or the error renders below
   Step 2 where a signer staring at a dead button never sees it.

## Notes

- The Admin ID's HRP follows **the device**, not the local node: a testnet Ledger build returns
  `tb1…` and a Trezor defaults to `bc1…` while the local stack is regtest. That is expected — the
  app does not re-encode the Admin ID. What matters in every flow is that the app and the device
  show the *same* string.
- The app never displays a certificate it has not verified itself: Rust recovers the public key,
  re-derives the P2WPKH address and requires it to equal the Admin ID before the modal shows
  anything. A certificate on screen is already self-consistent; Flow 4 is about someone else being
  able to check it.
