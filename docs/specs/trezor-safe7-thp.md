# Trezor Safe 7 (T3W1) — THP support

**Ticket:** [#566](https://github.com/wakeuplabs-io/alpen-multisig/issues/566) — the Trezor
connection method fails on a Safe 7.

**Status:** Planned. Phase 1 and Phase 2 close the ticket; Phase 3 is deferred.

This document is both the contract (§2) and the plan (§4). The contract section wins if they ever
disagree.

## 1. Why it fails

Connecting a Safe 7 fails at device initialization, on both "Connect wallet" and "Enter passphrase
on Trezor". The dockerised T3W1 emulator reproduces it exactly:

```
Trezor init failed on both transport modes. ... Details:
debug=false: init failed (Incorrect tag) |
debug=true: init failed (failure received: code=Failure_UnexpectedMessage ...)
```

- **Safe 7 firmware speaks only THP** (Trezor Host Protocol). The protocol is fixed at build time
  (`utils.USE_THP`); no device setting turns it off. Only the bootloader still speaks codec V1.
- **The app speaks Protocol V1**, through `trezor-client` 0.1.5
  (`infrastructure/hw_wallet/trezor.rs`, `open_trezor()`).
- **`Incorrect tag`** is how a V1 client decodes the device's answer to a V1 frame. The firmware replies
  with a fixed `Failure_InvalidProtocol`. Firmware 2.9.3 declares a 20-byte payload and pads it
  with zeros, so protobuf decoding fails on the padding before the failure code can surface.
- **`Failure_UnexpectedMessage`** comes from the `debug=true` retry. That opens the debug interface,
  and `Initialize` does not exist under THP at all (`apps/base.py` registers it only when THP is off).

The report says the same device worked on v0.2.4. Nothing in the analysis supports a V1 mode on
Safe 7 firmware, so that regression is recorded as not reproduced rather than investigated further.

## 2. Contract

### Acceptance criteria

1. **Connect.** A Safe 7 connects through "Connect wallet" and shows the same Admin ID as the
   mnemonic it holds (`m/84'/0'/73'/0/0`, P2WPKH).
2. **Hidden wallet.** "Enter passphrase on Trezor" opens the passphrase wallet, with the passphrase
   typed on the device keyboard and never on this computer. If passphrase protection is switched off
   on the device, the app refuses the hidden wallet with the existing message
   (`PASSPHRASE_DISABLED_ON_DEVICE`) rather than silently opening the standard one.
3. **Every device operation.** On a Safe 7 the following work as they do on a Safe 3:
   - verify an address on the device,
   - account xpub and master fingerprint,
   - sign a message (SPS-65 signing, the login challenge, the Admin ID certificate),
   - taproot PSBT signing (Admin Wallet send, commit funding).
4. **Pairing.** The first connection in an app run pairs the host with the device using
   **CodeEntry**: the device shows a 6-digit code, and the signer types it into the app. A wrong
   code shows an error, and connecting again starts a fresh pairing. Later operations in the same
   run do not pair again.
5. **No regression.** Safe 3 (T2B1) and every other V1 model behave exactly as today, including the
   passphrase and session-resume behavior documented in `trezor.rs`.

### Constraints

1. **Release firmware offers only CodeEntry.** SkipPairing, QR and NFC exist only in debug builds
   (`_DEFAULT_ENABLED_PAIRING_METHODS = [CodeEntry]`). So a physical Safe 7 needs CodeEntry, and the
   emulator has to be driven through the same path to prove it.
2. **The passphrase is chosen when the session is created, not by answering a request.** Under THP,
   `ThpCreateNewSession` carries it, and `PassphraseRequest` never arrives:
   - Standard wallet → an empty `ThpCreateNewSession`, which means passphrase `""` with no prompt.
   - Hidden wallet → `on_device = true`, which brings up the device keyboard during that call.

   Asking for a new session on a session id that already holds a seed is an error, so each connect
   takes a fresh id.
3. **The device ignores "passphrase disabled" under THP.** The refusal in AC 2 therefore reads
   `passphrase_protection` from `GetFeatures`, as the V1 path already does.
4. **The THP channel outlives a single operation.** Its keys and nonces live on the host. Today every
   operation opens its own transport. Under THP that would mean a new handshake each time and,
   without a stored credential, a new pairing.
5. **No backend-to-frontend events exist** in this app; every device call is one blocking `invoke`.
   Pairing has to fit that shape.

### Out of scope

- **Remembering the pairing across app restarts** (Phase 3, deferred). Until then the signer types a
  code once per app run.
- Bluetooth and the Trezor Bridge. The app keeps talking to USB and to the emulator's UDP port
  directly.

## 3. Architecture

### What already exists and is reused

| Piece | Location | Why it matters |
|---|---|---|
| `trezor_with_transport(model, Box<dyn Transport>)` | `trezor-client` `src/client/mod.rs:39` | Public. A THP transport plugs in underneath `Trezor`, so `call`, `resolve`, `get_xpub`, `sign_message_recoverable` and `sign_taproot_psbt` are reused unchanged. |
| `Transport` trait | `trezor-client` `src/transport/mod.rs` | `write_message` / `read_message` over `ProtoMessage(type, bytes)`: exactly what an encrypted THP channel carries. |
| `find_devices(false)` | `trezor-client` `src/lib.rs` | Already says whether the device is on USB or on the emulator's UDP port. |
| `TREZOR_DEVICE_LOCK`, `TrezorDevice` | `trezor.rs:66-90` | One operation at a time. The THP channel lives beside it. |
| `WalletKind`, `start_session`, `current_wallet_kind` | `trezor.rs:15-156` | The wallet choice per connection is the same fact under THP; only how it reaches the device changes. |
| Debuglink QA helpers | `desktop-app/e2e-webdriver/test/specs/g9-certificate-trezor.qa.js:37-97` | `read_layout` / `press_yes` through the emulator container. The pairing-code reader builds on them. |

### New pieces

- **`trezor-thp = "=0.1.1"`** — Trezor's own host-side implementation (`trezor-firmware/rust/trezor-thp`,
  MIT/Apache, sans-IO). It covers framing, CRC, ACK and retransmission, channel allocation and the
  Noise XX handshake. The version is pinned exactly because the API still changes between patch
  releases. Its crypto backend is `trezor-noise-rust-crypto` (x25519, AES-256-GCM, SHA-256), and it
  needs `getrandom`.
- **`trezor-client` 0.1.5 → 0.1.6.** This brings the generated `messages_thp` protos and
  `ThpCreateNewSession` as a `TrezorMessage`. `sign_tx`, `call` and `Transport` are unchanged. The one
  API change is to `get_public_key`, which the app does not call.
- **A packet link** with two variants: `UdpSocket` for the emulator, and `rusb` for USB (interface 0,
  endpoints `0x01`/`0x81`, 64-byte interrupt transfers). `trezor-client`'s own links are private,
  and `rusb` is already in the lockfile through it.
- **CPace for CodeEntry.** No stable crate exposes Elligator2 on Curve25519. The ~40-line RFC 9380
  map is ported from `trezorlib/thp/curve25519.py` onto `crypto-bigint` (already in the lockfile).
  It is checked against the official vectors (`core/tests/test_trezor.crypto.elligator2.py`,
  `python/tests/test_cpace.py`); those vectors are the only tests this code needs.

### How it fits

- **Detection.** `open_trezor()` tries V1 first, exactly as today. If `init_device` fails with
  `Failure_InvalidProtocol` or a protobuf decode error, it switches to THP and remembers that for
  the process. The `debug=true` retry is skipped for a THP device.
- **Channel.** One process-wide `Mutex<Option<ThpConn>>` holds the link, the encrypted channel, and
  the current `(session id, WalletKind)`. `open_trezor()` still returns a `TrezorDevice`, whose
  `Trezor` is built with `trezor_with_transport` over the channel. Any transport or channel error
  drops the static, and the next operation reopens it.
- **Sessions.** `GetFeatures` on session 0 replaces `Initialize`. `connect` sends
  `ThpCreateNewSession` on a fresh session id (Constraint 2), and the other operations reuse it.
- **Pairing across two calls** (Constraint 5):
  1. When the handshake reports the host unpaired, `connect` runs `ThpPairingRequest` →
     `ThpSelectMethod(CodeEntry)` → `ThpCodeEntryChallenge`. The device now shows the code. The
     pairing state stays in the static, and `connect` returns the fixed error
     `TREZOR_PAIRING_CODE_REQUIRED`.
  2. The connect hook recognizes that error, asks for the 6-digit code, and calls a new command,
     `trezor_submit_pairing_code(code)`. That command sends the CPace host tag, checks
     `ThpCodeEntrySecret` against the commitment, sends `ThpEndRequest`, and closes pairing.
  3. The hook then calls `connect` again.

  A wrong code makes the device answer a failure. The state is dropped, and the next connect starts
  over.

## 4. Phased plan

| Phase | Name | Closes | Touches |
|---|---|---|---|
| 1 | THP transport and sessions | AC 1, 2, 3, 5 on the emulator | `src-tauri` |
| 2 | CodeEntry pairing | AC 4; AC 1–3 on release firmware | `src-tauri`, `desktop-app` |
| 3 (deferred) | Remember the pairing | — (out of scope) | `src-tauri` |

Each phase is its own pull request against `develop`, branched from a freshly pulled `develop`.
Phases are sequential.

### Phase 1 — THP transport and sessions

Everything but pairing. While Phase 2 is pending, the app pairs with **SkipPairing** only when the
device offers it, which is only in emulator and debug builds. No release ships between the two
phases.

- `desktop-app/src-tauri/Cargo.toml`: add `trezor-client` 0.1.6, `trezor-thp =0.1.1` (`use_std`),
  `trezor-noise-rust-crypto`, `getrandom` and `rusb`.
- New `infrastructure/hw_wallet/trezor_thp/`: the packet link, the channel open and handshake, and a
  `Transport` implementation over the channel.
- `trezor.rs`: protocol detection, sessions and the passphrase through `ThpCreateNewSession`, and
  `GetFeatures`. The V1 path is left intact.

**Done when**, against `up.sh --model T3W1 --wipe`:
- `qa:login-trezor` (g10) passes;
- `qa:certificate-trezor` (g9) passes;
- with `--passphrase`, `qa:trezor-wallet-choice` (g5) passes;
- with passphrase protection off, `qa:trezor-hidden-refused` passes;
- after `up.sh --wipe` (T2B1), g10 still passes.

### Phase 2 — CodeEntry pairing

- New `trezor_thp/cpace.rs` (with the vector tests) and `trezor_thp/pairing.rs`. The SkipPairing branch
  is removed, so the emulator runs the same path as a real device.
- `commands/hw_wallet.rs`: `trezor_submit_pairing_code`, registered in **both** handler lists in
  `commands/invoke.rs`.
- Frontend: `wallet/hw-adapter.ts`, `domain/connect-wallet/hooks/use-hw-wallet-connect.ts`, and a
  6-digit input in the connect phase (`data-testid="e2e-trezor-pairing-code"`).
- QA: a helper reads the code through the emulator's debuglink with `read_layout()` (shown as
  `"123 456"`; confirm the format on the first run) and types it into the app. The confirm loop has
  to **stop pressing "yes" once the code screen is up**, because pressing it there cancels pairing.

**Done when**, on T3W1:
- g10 logs in through CodeEntry;
- a wrong code shows the error, and connecting again succeeds;
- g9 and g5 still pass;
- T2B1 g10 still passes.

A physical Safe 7 run is left to manual review.

### Phase 3 — Remember the pairing (deferred)

A `CredentialStore` backed by a host static key the app owns. The `ThpCredentialResponse` is kept
in the app data directory with owner-only permissions, so a restart asks only for the device's
connection confirmation, not for a code. It adds secret storage and its own review, which is why it
is not part of #566.

## 5. Verification

- Stack: `./scripts/local-stack.sh`. Emulator: `trezor-emu-docker/up.sh --model T3W1 --wipe`, with
  `--passphrase` for g5.
- Device QA, in `desktop-app/e2e-webdriver`: `SKIP_E2E_BUILD=1 npm run qa:<spec>` for the specs
  listed in each phase.
- The pre-commit CI checklist in `AGENTS.md`, every commit.

## 6. Risks

- **`trezor-thp` is young** (0.1.x, released 2026-08). It is pinned exactly, and its `host-cli`
  example is the reference whenever its API is unclear.
- **The code-screen text format** is taken from the firmware source (`pairing_context.py`) and must be
  confirmed on the emulator before the QA helper relies on it.
