# Admin Wallet — Session-Driven Broadcast Signing (R1.1)

**Status:** Designed
**Release:** R1.1 (adds the hardware-wallet signing path)
**Implementation plan:** [admin-wallet-implementation-plan.md](./admin-wallet-implementation-plan.md) — R1.1 row (§2) and §4 Release 1
**Related proposal:** [proposal-broadcast-commit-reveal.md](./proposal-broadcast-commit-reveal.md)
**Predecessor spec:** [admin-wallet-presign-commit-reveal.md](./admin-wallet-presign-commit-reveal.md) (R1.0.1)

---

## Objective

Unify broadcast signing behind a single driven port, `PsbtSigner`, so the commit transaction is signed
by either a **software mnemonic key** (a simulated hardware wallet, used for the "Palabras"/mnemonic
login) or a **real hardware wallet** (Trezor/Ledger on-device, used for the HW login) **through the same
flow**. The commit PSBT is always built by BDK from the wallet descriptor and handed to the selected
signer; only the signer implementation differs. This removes the watch-only dead-end (HW sessions today
have `can_sign = false` and hit `ReadOnly` on broadcast) and replaces the `ALLOW_DEV_MNEMONIC_SIGNING`
environment flag with a typed, per-signer network capability.

The downstream broadcast orchestration (`broadcast_commit_then_reveal`), the `CommitFunding` port
signature, and the reveal-signing semantics are **unchanged** — that is the central design constraint
and the main selling point of this slice.

This spec covers the **full end-to-end path** required for a governance broadcast to be signed by a
hardware wallet — not only the backend signing seam (D1–D7) but the desktop surface that makes it usable:
the user flow from clicking Broadcast through the on-device confirmation window to on-chain confirmation
(§"End-to-End Flow"), a structured broadcast error contract so the UI can branch on failure kind
(§DDD-8), and the device-interaction UX + error surfacing (§DDD-9). D1–D7 remain locked and unchanged;
DDD-8 and DDD-9 are **added**.

## Scope

### Included

- A `PsbtSigner` driven port on the signing boundary (`sign_psbt` + `allowed_on(network)`).
- `MnemonicPsbtSigner` — software signer wrapping the existing BDK `wallet.sign`; behaves as a
  *simulated hardware wallet* so the unified flow is exercised end-to-end with no device.
- `HwPsbtSigner` — real on-device taproot **key-path** PSBT signing via the Trezor/Ledger adapters.
- Split `WalletService` commit signing into `build_psbt` (BDK build, both paths) + `sign` (delegates to
  the attached `PsbtSigner`) + BDK finalize + `extract_tx`.
- Replace `WalletService.can_sign: bool` with an optional signer capability
  (`Option<Arc<dyn PsbtSigner>>`); `can_sign()` = signer present **and** `signer.allowed_on(network)`.
- Per-signer network capability replacing `ALLOW_DEV_MNEMONIC_SIGNING`:
  `MnemonicPsbtSigner` is allowed on **regtest | testnet only**; `HwPsbtSigner` is allowed on **any**
  network.
- Attach the correct signer per login type at session init (`wallet_session.rs`).
- Typed errors: `AdminWalletError::SignerNotAllowedOnNetwork` and HW-side
  `WalletError::HwSigningFailed` / `HwDisconnected` / `HwUserRefused` returned **before** any broadcast.
- HW device access in the pre-sign window via `tokio::task::spawn_blocking` (the Trezor client is
  synchronous); the device is re-opened by fingerprint at sign time (no live connection held in the
  session).
- **(DDD-8) Structured broadcast error contract** — `proposals_broadcast` returns `{ code, message }`
  (backward-compatible JSON string) so the UI can branch on kind and offer recovery correctly. See
  §"Structured broadcast error contract (DDD-8)".
- **(DDD-9) Frontend device-UX + error surfacing** — the desktop broadcast surface needed for the HW path
  to actually work end-to-end: a coarse "Confirm on your device" affordance during the HW pre-sign window
  (skipped for the mnemonic/simulated-HW path), kind-specific error copy via a new `deriveBroadcastError`,
  recovery-gated resubmit, and per-reason disabled-tooltip copy. The confirm control already auto-enables
  from `canSign` (no new gating). See §"Frontend / UI impact (DDD-9)" and §"End-to-End Flow".

### Not included

- **Send-on-HW** (sending a value transaction on hardware) — Phase 7.
- **Verify-on-device** (address/amount confirmation UX beyond the device's native prompt) — Phase 7.
- **Mainnet/testnet remote-RPC hardening** (Esplora/electrum endpoints, auth, retries) — Phase 9.
- Any change to **SPS-50/51/65 envelope or reveal semantics**.
- **Receive flow** (R1.2) and **receive rotation** (R1.3).
- Persistent storage of pre-signed reveals (still in-memory, inherited from R1.0.1).

---

## Technical Design

### Current state (R1.0.1)

```
session login ── mnemonic ─► WalletService::new(...)            can_sign = true
              └─ HW xpub  ─► WalletService::new_watch_only(...)  can_sign = false ──► broadcast = ReadOnly (dead end)

broadcast(commit_funding: &dyn CommitFunding):
  BdkAdminWalletMnemonic.build_signed_commit
    └─ WalletService.build_signed_commit
         ├─ if !can_sign            -> AdminWalletError::ReadOnly
         ├─ check_enabled()         -> regtest && ALLOW_DEV_MNEMONIC_SIGNING  (env flag)
         ├─ sync()
         └─ build_and_sign_tx()     -> build_tx -> finish (PSBT) -> wallet.sign -> extract_tx
```

### Target state (R1.1)

```
session login ── mnemonic ─► WalletService::with_signer(MnemonicPsbtSigner)   (simulated HW)
              └─ HW (xpub) ─► WalletService::with_signer(HwPsbtSigner{fingerprint})  (real device)

broadcast(commit_funding: &dyn CommitFunding):   <- UNCHANGED port + UNCHANGED orchestration
  AdminWalletCommitFunding.build_signed_commit
    └─ WalletService.build_signed_commit
         ├─ signer = self.signer.ok_or(ReadOnly)?
         ├─ if !signer.allowed_on(network) -> AdminWalletError::SignerNotAllowedOnNetwork
         ├─ sync()
         ├─ psbt = build_psbt()                       <- BDK builds fully-annotated PSBT (both paths)
         ├─ signer.sign_psbt(&mut psbt)               <- port call: software OR on-device
         │     ├─ MnemonicPsbtSigner: wallet.sign(psbt, SignOptions::default())
         │     └─ HwPsbtSigner: spawn_blocking -> re-open device by fingerprint -> taproot key-path sign
         │           (device absent / refusal -> HwSigningFailed / HwDisconnected, BEFORE broadcast)
         └─ finalize_psbt + extract_tx -> Transaction

reveal: signed in-app by the per-broadcast ephemeral envelope key  <- NEVER routed to PsbtSigner
        (taproot script-path spend over a custom envelope leaf; a HW cannot sign it)
```

**Invariant (D7):** BDK builds the fully-annotated **unsigned** PSBT for both paths. The watch-only
wallet built from the HW xpub knows the BIP-86 descriptor, so `build_tx` populates the taproot
internal-key / BIP32 derivation fields on each input — which is exactly what the device needs to sign.

**Reveal change stays on the Admin Wallet (HW path).** `reveal_change_address()` on a watch-only HW
session resolves to an Admin-Wallet **internal** SPK: the descriptor wallet's `peek` / `reveal_next_address`
work without private keys (address derivation needs only the descriptor, not the seed). The R1.0 invariant
"reveal change → Admin Wallet" therefore holds unchanged on the HW path — change never strands on the
per-broadcast ephemeral envelope key.

**Single source of truth for `network` (D3).** The `network` argument passed to `allowed_on` derives from
the wallet's own `network()` — the same value already used inside `build_signed_commit` — which must agree
with the descriptor's network. Do **not** introduce a second source from `broadcast_env`; the wallet's
`network()` is the single source of truth for the capability check.

**Master-fingerprint capture (D7 correctness dependency).** The HW **master** fingerprint must be
captured at connect time — alongside the account xpub in `auth_complete` / `init_from_xpub` — and stored
on the session and on `HwPsbtSigner`. The account xpub's `parent_fingerprint` is **not** the master
fingerprint; it is the fingerprint of the xpub's immediate parent, not the seed's. The watch-only
descriptor must embed the correct **origin** fingerprint (`[<master_fp>/86'/...]`) so the device
recognizes its own inputs when it receives the PSBT; an incorrect origin fingerprint causes the device to
refuse to sign. Capturing the master fingerprint at connect (not deriving it from the xpub) is therefore
a correctness dependency for D7, not just for device re-open.

**spawn_blocking timeout.** The `HwPsbtSigner.sign_psbt` call runs inside `tokio::task::spawn_blocking`
(synchronous HID client). Wrap it with `tokio::time::timeout` at **60 seconds** so that if the device
is unresponsive, the user is not stuck indefinitely. On timeout, return `HwSigningFailed` with message
"Device did not respond within 60 seconds. Check the connection and try again." This also handles the
case where the user closes the app mid-sign — the blocking task is cancelled and no broadcast occurs.

### Production functions / types

| Function / Type | Module | Responsibility |
|---|---|---|
| `trait PsbtSigner` | `application/psbt_signer.rs` (new) | Driven port: `sign_psbt(&self, &mut Psbt)` + `allowed_on(&self, Network) -> bool` |
| `MnemonicPsbtSigner` | `application/psbt_signer.rs` (new) | Software signer = simulated HW; wraps BDK `wallet.sign`; `allowed_on` = regtest \| testnet |
| `HwPsbtSigner` | `infrastructure/hw_wallet/hw_psbt_signer.rs` (new) | On-device taproot key-path PSBT signing; re-opens by fingerprint; `allowed_on` = any |
| `WalletService::with_signer` | `application/wallet_service.rs` | Construct holding `Option<Arc<dyn PsbtSigner>>` |
| `WalletService::build_psbt` | `application/wallet_service.rs` | BDK build_tx → finish → unsigned annotated PSBT (both paths) |
| `WalletService::build_signed_commit` | `application/wallet_service.rs` | **Single authoritative enforcement point.** Calls `signer.allowed_on(network)` **FIRST** and returns `SignerNotAllowedOnNetwork` **BEFORE** any sync / RPC / PSBT build. Order: guard (signer present) → `allowed_on` → sync → build_psbt → `signer.sign_psbt` → finalize → extract_tx |
| `WalletService::can_sign` | `application/wallet_service.rs` | signer present AND `signer.allowed_on(network)` |
| `AdminWalletError::SignerNotAllowedOnNetwork` | `infrastructure/admin_wallet/wallet.rs` | Typed error replacing the `Disabled`/env-flag gate |
| `WalletError::HwSigningFailed` / `HwDisconnected` / `HwUserRefused` | `application/wallet_session.rs` (or shared) | Device absent / user refusal / device-reported rejection, returned before broadcast |
| `TrezorAdapter::sign_psbt` / `LedgerAdapter::sign_psbt` | `infrastructure/hw_wallet/{trezor,ledger}.rs` | New: taproot key-path PSBT signing (synchronous client) |
| `WalletSession::init_from_mnemonic` / `init_from_xpub` | `application/wallet_session.rs` | Attach `MnemonicPsbtSigner` / `HwPsbtSigner` per login type. **`init_from_xpub` signature changes**: must accept `master_fingerprint` alongside `account_xpub` + `network` (captured at connect time, not derived from xpub). |

---

## Test Cases

### Backend signing (slice (a) unless noted)

**BE-01. Happy path — mnemonic (simulated HW), regtest.** Mnemonic login → `MnemonicPsbtSigner`
attached → `build_signed_commit` produces a valid extractable commit tx; broadcast completes
commit → reveal → confirmed.

**BE-02. Happy path — HW on-device (slice (b)).** HW login → `HwPsbtSigner` attached → `sign_psbt` runs inside
`spawn_blocking` (wrapped in 60s timeout), re-opens device by fingerprint, taproot key-path signs; commit finalizes and
broadcasts.

**BE-03a. Signer selection — mnemonic.** `init_from_mnemonic` attaches `MnemonicPsbtSigner`.

**BE-03b. Signer selection — HW.** `init_from_xpub` (with `master_fingerprint`) attaches `HwPsbtSigner`.

**BE-03c. can_sign reflects network capability.** `can_sign()` returns `signer.allowed_on(network)` for the attached signer.

**BE-04. Capability matrix — parametrized `allowed_on` assertions.** All signer/network combinations in one test:
`(MnemonicPsbtSigner, Testnet) → true`, `(MnemonicPsbtSigner, Regtest) → true`,
`(MnemonicPsbtSigner, Bitcoin) → false`; `(HwPsbtSigner, Regtest) → true`,
`(HwPsbtSigner, Testnet) → true`, `(HwPsbtSigner, Bitcoin) → true`.

**BE-05. Mnemonic rejected on mainnet — fail fast, no side effects.** `allowed_on(Network::Bitcoin)`
= false; broadcast on mainnet with a mnemonic signer returns `AdminWalletError::SignerNotAllowedOnNetwork`.
Assert the `allowed_on` check fires **first**: **no `sync()` / RPC call and no PSBT build occurs** on the
reject path, and nothing is broadcast.

**BE-06. HW session present but no signer attached (step-(a)→(b) intermediate window).** A HW session
exists but no `PsbtSigner` is attached. `build_signed_commit` returns `ReadOnly` — **not a panic, and never a silent software/mnemonic sign** — and **nothing is broadcast**.

**BE-07. Device-absent typed error before any broadcast.** `HwPsbtSigner.sign_psbt`
surfaces `HwDisconnected`; `build_signed_commit` returns the error and
`broadcast_commit_then_reveal` never reaches the network (commit not sent).

**BE-08. Device disconnects mid-sign — timeout or HID error.** `HwPsbtSigner.sign_psbt` is in progress
inside `spawn_blocking`; device is unplugged. Either the 60s timeout fires or the HID call returns an error.
Result: `HwSigningFailed` or `HwDisconnected` returned, **no broadcast occurs**.

**BE-09. Fingerprint mismatch — different device plugged in at sign time.** `HwPsbtSigner` was initialized
with fingerprint A; device with fingerprint B is plugged in. `sign_psbt` detects mismatch and returns
`HwSigningFailed` with message indicating wrong device. **No broadcast occurs.**

**BE-10. Reveal still ephemeral.** Reveal is signed by the per-broadcast envelope key; assert the reveal
path never calls `PsbtSigner`.

**BE-11. Session expiry during broadcast (OrchestratorUnauthorized).** Orchestrator returns 401 during
`proposals_broadcast`. Error maps to `code = OrchestratorUnauthorized`, boundary = BEFORE, recovery = `re-auth → retry`.
**No broadcast occurs.**

**BE-12. Confirmation timeout (Timeout).** Broadcast succeeds but confirmation poll exceeds
`confirm_timeout_ms`. Error maps to `code = Timeout`, boundary = AFTER, recovery = `resubmit-reveal`.
`canResubmit === true`.

**BE-13. BitcoinRpc failure — BEFORE boundary (build/sign failure).** RPC error occurs before
`submit_package` is reached (e.g. during sync). Error maps to `code = BitcoinRpc`, boundary = BEFORE,
recovery = `retry-from-scratch`. `canResubmit === false` even if `PendingReveal` exists in store (NIT-3).

**BE-14. BitcoinRpc failure — AFTER boundary (post-broadcast).** RPC error occurs after
`submit_package` / sequential-send was attempted. `PendingReveal` exists. Error maps to
`code = BitcoinRpc`, boundary = AFTER, recovery = `resubmit-reveal`. `canResubmit === true`.

**BE-15. Regression — downstream unchanged.** Existing `broadcast_commit_then_reveal` and `CommitFunding`
tests stay green (port signature and orchestration untouched).

**BE-16. Mnemonic on regtest without `ALLOW_DEV_MNEMONIC_SIGNING` env var.** Full broadcast succeeds
because `MnemonicPsbtSigner.allowed_on(regtest)` = true. This test runs with the env var **unset**,
validating that the flag removal does not break regtest.

### Frontend (slice (a) unless noted) — see §"Frontend / UI impact (DDD-9)"

**FE-01. Button auto-enables when `can_sign` flips true — regression, no new gating code.** The broadcast
confirm control already renders `disabled={isBroadcasting || !canSign}` and `canSign` already flows from
`useAdminWalletCapability()` → `admin_wallet_can_sign`. Assert: when `admin_wallet_can_sign` returns
`true` for an HW session, the confirm control is enabled with **no change** to gating logic. This is a
*characterization / regression* test guarding the design win — not a new feature. Extend the existing
`broadcast-details-card-can-sign.test.ts` (which already encodes `broadcastButtonDisabled(isBroadcasting,
canSign)`). **Note:** the existing test file uses Node `assert` as a script that reimplements component logic.
Replace with a proper Vitest test using `@testing-library/react` that renders the actual component and
asserts DOM state.

**FE-02. Structured-error mapping — all 10 error codes.** Unit test `broadcast_error_code` (Rust pure helper):
each `BroadcastError` / `AdminWalletError` / `WalletError` variant maps to the expected `{ code }` per the
§DDD-8 table (`SignerNotAllowedOnNetwork`, `HwDisconnected`, `HwSigningFailed`, `HwUserRefused`, `ReadOnly`,
`BitcoinRpc`, `Timeout`, `OrchestratorUnauthorized`, `NoPendingReveal`, `Unknown`), and carries a non-empty `message`.
Parametrized — one assertion per code.

**FE-03a. `deriveBroadcastError` parses structured JSON.** Given `{ code: 'HwDisconnected', message: '...' }`
JSON, `deriveBroadcastError` returns `{ code, message, recovery: 'reconnect-device' }` with kind-specific copy.

**FE-03b. `deriveBroadcastError` falls back for legacy string.** Given a bare legacy error string,
`deriveBroadcastError` returns `{ code: 'Unknown', message: <raw>, recovery: 'retry' }` (backward compatible).

**FE-04. Resubmit-reveal gating — structural prevention of the latent bug (CRITICAL).** `canResubmit` is
`true` **only** when `error.recovery === 'resubmit-reveal'` (a post-broadcast-boundary error with a live
`PendingReveal`), and **false** for `HwDisconnected` / `HwUserRefused` / `HwSigningFailed` /
`SignerNotAllowedOnNetwork` / `ReadOnly` / `OrchestratorUnauthorized` (all pre-broadcast-boundary). The
gating contract MUST be `recovery`-driven, never `Boolean(error)`. Assert that a device-absent
commit-signing failure yields `canResubmit === false` (so no resubmit affordance can ever be offered for
it). This guards the bug structurally before any resubmit control is wired into the card.

**FE-05a. Device-prompt state appears during HW pre-sign window (slice (b)).** For an HW
session the controller exposes an `awaiting-device` phase while `proposals_broadcast` is in flight, and
`BroadcastDevicePrompt` renders "Confirm on your device".

**FE-05b. Mnemonic path skips device prompt (slice (a) regression).** For the mnemonic / simulated-HW session the
signer returns instantly, so the prompt is transient/never shown and the card behaves byte-identically
to today (no device affordance). Assert both branches off the `canSign`-source signer kind.

**FE-06. Mnemonic / simulated-HW path keeps card behavior identical (slice (a) regression).** With a mnemonic
session, the broadcast card renders no device prompt, advances commit → reveal → confirmed exactly as
R1.0.1, and the only observable change is structured-error copy + corrected resubmit gating.

### E2E

**E2E-01. Regtest e2e — mnemonic walking skeleton.** Full broadcast on the real Tauri binary using the
"Palabras" login with zero device; commit + reveal confirmed.

**E2E-02. Webdriver — mnemonic walking skeleton (slice (a) GATE).** This is the slice (a)
completion gate: extend the existing `desktop-app/e2e-webdriver` broadcast spec
(`specs/broadcast-flow.e2e.ts`) per the repo README pattern — with the "Palabras" (mnemonic) login and
zero device, an approved proposal broadcasts through the unified flow; the phase progress advances
commit → reveal → confirmed; no device prompt appears. Run via the package's `test:e2e:*` scripts per
`desktop-app/e2e-webdriver/README.md`. Slice (a) is not done until this gate is green.

### Release checklist (manual, NOT CI-automatable)

**REG-01. Manual real-device path (slice (b)).** Connect a real Trezor/Ledger, log in via HW, broadcast an approved proposal, observe
the "Confirm on your device" prompt, physically confirm on-device, and verify commit + reveal confirm.
A second manual case: unplug / refuse on the device and assert the UI shows the `HwDisconnected` /
`HwUserRefused` copy with **no** "Resubmit reveal" control and nothing broadcast. This is a release
checklist item, not a CI gate (no device in CI).

### Test doubles policy

- **`MockBitcoinRpc`**: validates RPC method names, simulates `submit_package` success / unknown-method / RPC error.
  Must reject invalid inputs (empty tx hex, malformed txid) like the real adapter.
- **`MockOrchestratorClient`**: validates auth token present, simulates 200 / 401 / 409 responses.
- **`InMemoryCommitFunding`**: validates network, address format, amount > 0. Returns a deterministic signed `Transaction`.
- **`FakeHwDevice`**: simulates fingerprint match/mismatch, signing success/failure, device absent, user refusal.
  Used to test `HwPsbtSigner` without physical USB device. Must validate that fingerprint is non-empty before signing.
- **Real I/O**: `E2E-01` and `E2E-02` use real Tauri binary, real BDK wallet, real regtest RPC.
  `REG-01` uses real hardware device (manual).

---

## End-to-End Flow (UI → device → chain)

This section traces a single governance broadcast from the operator's click to on-chain confirmation,
making explicit **where the user must physically confirm on the hardware device** and **what the UI shows
during the blocking pre-sign window**. It binds the locked backend design (D1–D7) to the desktop surface.

> Naming note (ground truth — verified against the repo). The repository frontend uses a `domain/`-oriented
> layout, not an FSD `features/` layout. The "controller" is the hook
> `domain/broadcast-proposal/hooks/use-broadcast-proposal.ts`; the **confirm button + disabled tooltip +
> error rendering live in `domain/broadcast-proposal/components/broadcast-details-card.tsx`** and the phase
> rail + error banner in `broadcast-phase-progress.tsx`. There is **no** separate confirmation-modal /
> button / error-alert file, and — important — **there is no "Resubmit reveal" control wired into this
> broadcast card today**. The resubmit IPC (`proposals_resubmit_reveal`, R1.0.1) exists on the backend but
> is not surfaced on this screen. `canSign` is sourced by **`useAdminWalletCapability()`** in
> `domain/admin-wallet/hooks/use-admin-wallet-capability.ts` (via the `admin_wallet_can_sign` IPC) and
> passed into `BroadcastDetailsCard` from `screens/broadcast-proposal-screen.tsx`. The controller's `error`
> is a flat `string | null` today. The design below targets these real files; any reference to a "modal" or
> "panel" means an in-card affordance unless a new component is explicitly proposed.

### Sequence

```mermaid
sequenceDiagram
  actor User
  participant Card as BroadcastDetailsCard (UI)
  participant Ctl as useBroadcastProposal (controller hook)
  participant Cap as useAdminWalletCapability (canSign source)
  participant Api as api/proposals.ts (invoke)
  participant Cmd as proposals_broadcast (Tauri cmd)
  participant App as proposals::broadcast_commit_then_reveal
  participant WS as WalletService.build_signed_commit
  participant Sig as PsbtSigner.sign_psbt
  participant Dev as Hardware device (spawn_blocking)
  participant RPC as Bitcoin Core / Esplora
  participant Orch as Orchestrator

  User->>Card: Open broadcast screen
  Cap->>Api: getAdminWalletCanSign() → admin_wallet_can_sign
  Api-->>Cap: canSign (+ signerKind, reason in slice b)
  Ctl->>Api: prepareBroadcast(actionId)
  Api->>Cmd: proposals_prepare_broadcast
  Cmd-->>Card: { commitAddress, commitAmountSats, estimatedFeeSats }
  Note over Card: Confirm control disabled={isBroadcasting || !canSign} (already wired)
  User->>Card: Click "Confirm & Broadcast"
  Card->>Ctl: broadcast()
  Ctl->>Ctl: phase = 'broadcasting'  (HW: isAwaitingDevice = true)
  Ctl->>Api: broadcastProposal(actionId)
  Api->>Cmd: proposals_broadcast  (single awaited IPC — blocks through signing)
  Cmd->>App: broadcast_commit_then_reveal(...)
  App->>WS: build_signed_commit (guard → allowed_on → sync → build_psbt)
  WS->>Sig: sign_psbt(&mut psbt)
  alt Mnemonic (simulated HW)
    Sig-->>WS: signed instantly (no prompt window)
  else Real hardware
    Sig->>Dev: spawn_blocking → re-open by fingerprint → key-path sign
    Note over User,Dev: User physically confirms on the device screen
    Dev-->>Sig: taproot key-spend signature (or HwUserRefused / HwDisconnected)
  end
  WS-->>App: finalize + extract_tx → signed commit Transaction
  App->>App: build+sign reveal (ephemeral) → drop key → pending_reveals.insert
  App->>RPC: submit_package([commit,reveal]) or sequential
  App->>Orch: commit_broadcasted ; reveal_broadcasted
  App->>RPC: (regtest) mine ; wait confirm
  App->>Orch: reveal_confirmed ; pending_reveals.remove
  App-->>Cmd: (commit_txid, reveal_txid)
  Cmd-->>Api: BroadcastResultDto  OR  Err({ code, message })
  Api-->>Ctl: ApiResult (ok | raw error string)
  Ctl->>Ctl: error string → deriveBroadcastError → { code, message, recovery }
  Ctl->>Card: phase = 'done'  OR  phase = 'error' (kind-specific copy)
```

### The blocking pre-sign window (what the user sees)

`proposals_broadcast` is **one awaited IPC call** that does not return until commit signing, reveal
signing, broadcast and confirmation have all completed (or failed). For the HW path, the device prompt
happens *inside* this single call (within `spawn_blocking`). The frontend therefore cannot receive
fine-grained progress for free — it only knows "the call is in flight". The design (see §DDD-9 and Open
notes (a)) uses a **coarse `awaiting-device` state** derived from `inFlight` + signer kind: enough to render
"Confirm on your device" without a Rust→JS event channel. For the mnemonic path the signer returns in
microseconds, so the coarse state is never observed and the card is visually unchanged from R1.0.1.

### The broadcast boundary (the recovery invariant)

Every failure is classified as **before** or **after** the *broadcast boundary* — the moment the commit
first hits the network. This single fact decides the recovery action offered to the user:

- **Before** the boundary (signer-not-allowed, device absent/refused, read-only/no-signer, prepare/auth
  failures): nothing is on-chain. Note (NIT-3): a `PendingReveal` **may already exist** for these cases,
  because the signed reveal is inserted into the store *before* the commit is broadcast — so presence of a
  `PendingReveal` does NOT prove the commit was sent. The boundary is decided by whether the broadcast was
  reached/attempted, not by `PendingReveal` presence. Recovery = fix the cause and **retry from scratch**;
  "Resubmit reveal" MUST NOT be offered even if a `PendingReveal` is in the store.
- **After** the boundary (transient RPC blip on the sequential path, confirm-timeout) with a live
  `PendingReveal`: the commit may be in the mempool and the signed reveal is stored. Recovery =
  **resubmit reveal** (eligibility = AFTER-boundary AND a live `PendingReveal` exists).

---

## Structured broadcast error contract (DDD-8)

### Problem

Today `proposals_broadcast` collapses every failure to a flat `String` via `map_broadcast_error`; the
controller stores it as `error: string | null` and the UI renders it verbatim in
`broadcast-phase-progress.tsx`. The UI **cannot branch on error kind**. There is no `deriveBroadcastError`
and no resubmit-reveal control on the broadcast card **yet** — but the resubmit IPC
(`proposals_resubmit_reveal`) already exists on the backend, and the natural next step (surfacing a
"Resubmit reveal" affordance) would, if naively gated on "is there an error", be a **latent bug** for the
HW path: a commit-signing failure (device absent/refused) happens **before** the broadcast boundary, so no
`PendingReveal` exists — yet a presence-gated control would still offer "Resubmit reveal", which is wrong
and misleading. DDD-8 + DDD-9 make the contract kind-aware **before** that control is ever wired, so the
bug is structurally impossible: resubmit is gated on `recovery === 'resubmit-reveal'`, never on error
presence.

### The DTO

`proposals_broadcast` (and `proposals_resubmit_reveal`) return a structured, backward-compatible error
shape over IPC:

```jsonc
{ "code": "HwUserRefused", "message": "You declined the transaction on your device." }
```

- `code`: a stable machine-readable enum string (below). `message`: a human-readable fallback.
- **Backward compatible**: Tauri still rejects the IPC promise with a JSON **string**; today's consumers
  that read it as a bare message keep working because `deriveBroadcastError` parses-or-passes-through. The
  change is additive — `map_broadcast_error` now returns `serde_json::json!({ "code", "message" })`
  (stringified), mirroring the existing `serialize_wallet_error` `{ type, message }` precedent in
  `commands/admin_wallet.rs`.

### Code set, boundary, copy and recovery

| `code` | When it fires | Boundary | User-facing message (English) | Recovery |
|---|---|---|---|---|
| `SignerNotAllowedOnNetwork` | `AdminWalletError::SignerNotAllowedOnNetwork` — e.g. mnemonic signer on mainnet; fired by `allowed_on` **before** sync/build | BEFORE | "This signer is not allowed on the current network. Use a hardware wallet for mainnet." | retry-from-scratch (after switching signer/network) |
| `HwDisconnected` | `WalletError::HwDisconnected` — device not present / lost at re-open or during signing | BEFORE | "Hardware wallet not detected. Reconnect your device and try again." | reconnect-device → retry |
| `HwUserRefused` | device reported user rejection (if distinguishable from a generic failure) | BEFORE | "You declined the transaction on your device." | retry-from-scratch |
| `HwSigningFailed` | `WalletError::HwSigningFailed` — device present but signing failed (wrong app, locked, firmware, fingerprint mismatch) | BEFORE | "The device could not sign this transaction. Check it is unlocked and on the Bitcoin app, then try again." | reconnect-device → retry |
| `ReadOnly` | `AdminWalletError::ReadOnly` — no signer attached (watch-only, or HW session in the (a)→(b) window) | BEFORE | "This wallet cannot sign. Connect a hardware wallet to broadcast." | retry-from-scratch (after attaching signer) |
| `BitcoinRpc` | `BroadcastError::BitcoinRpc` — node/Esplora RPC error | BEFORE or AFTER | "The Bitcoin node rejected or could not process the broadcast." | resubmit-reveal **iff** the broadcast was reached/attempted (AFTER boundary) **and** a live `PendingReveal` exists (NIT-3); else retry-from-scratch |
| `Timeout` | confirmation poll exceeded `confirm_timeout_ms` after broadcast | AFTER | "Broadcast sent but confirmation timed out. You can resubmit the reveal." | resubmit-reveal |
| `OrchestratorUnauthorized` | orchestrator returns 401 (`BroadcastError::ProposalFetch(Backend{401})`) | BEFORE (pre-broadcast fetch) | "Your orchestrator session expired (401). Re-authenticate and retry." | re-auth → retry |
| `NoPendingReveal` | `proposals_resubmit_reveal` called but the store has no entry for `action_id` | n/a (resubmit only) | "No pending reveal to resubmit — re-run the broadcast." | retry-from-scratch |
| `Unknown` | any unmapped error / legacy flat string | unknown | (the raw message, or "Broadcast failed for an unknown reason.") | retry-from-scratch |

Recovery values collapse on the UI to three actions: `retry` (re-run broadcast / fix cause),
`resubmit-reveal` (offer the resubmit control), `reconnect-device` (prompt to reconnect, then retry). The
**critical invariant**: `resubmit-reveal` is the recovery for **`Timeout`** and a post-broadcast
**`BitcoinRpc`** only; never for any `Hw*`, `SignerNotAllowedOnNetwork`, `ReadOnly`, or
`OrchestratorUnauthorized` code.

### Backend mapping site

> **Invariant (NIT-3) — PendingReveal presence does NOT prove the commit was broadcast.** Verify against
> the real code in `application/proposals.rs::broadcast_commit_then_reveal`: today the signed reveal is
> inserted into `PendingReveals` **before** the commit is broadcast (the insert step precedes the
> broadcast step). Therefore the existence of a `PendingReveal` for an `action_id` does **not** by itself
> prove the commit hit the network — a build / sign / sync failure can leave a `PendingReveal` in the
> store while nothing was ever submitted. The BEFORE/AFTER-broadcast-boundary classification (for
> `BitcoinRpc` and **any** post-sign error) MUST be derived from **whether the broadcast call was actually
> reached / attempted** — e.g. the orchestrator `commit_broadcasted` (or `reveal_broadcasted`) report was
> sent, or an explicit in-flow boundary flag — **not** merely from `pending.get(action_id).is_some()`.
>
> Concretely:
> - A commit/reveal **BUILD**, **SIGN**, or **sync** failure (pre-broadcast) → **BEFORE** boundary →
>   recovery = retry-from-scratch, and **resubmit-reveal MUST NOT be offered** even though a
>   `PendingReveal` may already be in the store.
> - A failure **at or after** the `submit_package` / sequential-send step → **AFTER** boundary →
>   resubmit-reveal eligible.
> - **Recommended signal:** thread a boundary flag from the broadcast flow (or gate on the
>   `commit_broadcasted` / `reveal_broadcasted` report having been sent) into `map_broadcast_error`, rather
>   than inferring the boundary from `PendingReveal` presence.
> - **Resubmit eligibility = AFTER-boundary AND a live `PendingReveal` exists** (both conditions, not
>   presence alone).

`commands/proposals.rs::map_broadcast_error` is extended (not replaced) to:
1. Classify the `BroadcastError` / underlying `AdminWalletError` / `WalletError` into a `code`.
2. Determine the boundary for `BitcoinRpc` from **whether the broadcast call was reached/attempted** (the
   boundary flag or the `commit_broadcasted` report), **then** require a live `PendingReveal` for the
   `action_id` before treating it as resubmittable (the store is already in scope for `proposals_broadcast`).
   Presence of a `PendingReveal` alone is **not** sufficient (NIT-3).
3. Return `serde_json::json!({ "code": code, "message": msg }).to_string()`.

The existing 401 special-case becomes `code = OrchestratorUnauthorized`. `NoPendingReveal` is mapped in the
`proposals_resubmit_reveal` arm (already special-cased today). A new small pure helper
`broadcast_error_code(&BroadcastError, broadcast_reached: bool, has_pending: bool) -> &'static str` is
unit-testable in isolation (FE-02). Per NIT-3 the helper takes **both** the boundary signal
(`broadcast_reached` — was `submit_package` / sequential-send reached, e.g. via the boundary flag or the
`commit_broadcasted` report) **and** `has_pending`; resubmit eligibility requires `broadcast_reached &&
has_pending`, never `has_pending` alone.

---

## Frontend / UI impact (DDD-9)

The desktop surface needs three changes — none of which alter the locked backend ports.

**Controller input (NIT-2, slice (b) only — consistent with NIT-1):** `useBroadcastProposal` receives
`signerKind` (and `canSignReason`) as an **optional parameter passed from the screen** (sourced from
`useAdminWalletCapability()`), **not** sourced internally; it computes
`awaiting-device = inFlight && signerKind === 'hardware'`. In slice (a) the parameter is absent, so the
expression is never true and the controller stays on today's path (per the NIT-1 fail-safe rule).

Per-file:

| File | Verdict | Change |
|---|---|---|
| `domain/broadcast-proposal/model/broadcast-proposal.ts` | EXTEND | Add a `BroadcastError` view-model `{ code, message, recovery }`; add `BroadcastErrorCode` and `BroadcastRecovery` narrow-union types; add a new pure `deriveBroadcastError(raw: string): BroadcastError` that parses the `{ code, message }` JSON (or falls back for a legacy string) and maps `code → recovery` + kind-specific copy — mirroring the existing `parseAdminWalletError` precedent in `domain/admin-wallet/hooks/parse-admin-wallet-error.ts`. Add `'awaiting-device'` to `BroadcastPhase`. |
| `domain/broadcast-proposal/hooks/use-broadcast-proposal.ts` | EXTEND | Change `error` from `string | null` to `BroadcastError | null` (run the raw IPC reject through `deriveBroadcastError`). Expose `canResubmit = error?.recovery === 'resubmit-reveal'` (the forward-looking gating contract — never `Boolean(error)`). Add an `awaiting-device` transient phase for the HW path: set when `broadcast()` starts **and** `signerKind === 'hardware'`. Expose `isAwaitingDevice`. Signer kind is passed in from the screen (sourced by `useAdminWalletCapability`), keeping the controller a thin consumer. |
| `domain/broadcast-proposal/components/broadcast-details-card.tsx` | EXTEND | Render `<BroadcastDevicePrompt>` when `isAwaitingDevice`. Use the kind-specific disabled tooltip (replacing the single `Hardware wallet required to sign` string) driven by `canSignReason`. If/when a resubmit control is added here, gate it on `canResubmit` (recovery-driven), never on error presence. No change to `disabled={isBroadcasting || !canSign}`. |
| `domain/broadcast-proposal/components/broadcast-phase-progress.tsx` | EXTEND | Render `error.message` (kind-specific copy) in the error banner instead of a flat string. Treat `'awaiting-device'` like `'broadcasting'` for step ranking (commit/reveal group active) so the rail does not regress. |
| `domain/broadcast-proposal/components/broadcast-device-prompt.tsx` | CREATE NEW | Small presentational component: "Confirm on your device" with a device glyph; mounted only during the HW pre-sign window. Single responsibility; no IPC. For the mnemonic path it is never mounted. |
| `domain/admin-wallet/hooks/use-admin-wallet-capability.ts` | EXTEND | This is the **real `canSign` source** (`useAdminWalletCapability`). Surface a `signerKind` (`'hardware' | 'mnemonic' | 'none'`) and an optional `canSignReason` alongside `canSign`, sourced from the wallet-status DTO (see §"Wallet-status / canSign contract"). Drives the device-prompt branch and the disabled-tooltip wording. |
| `screens/broadcast-proposal-screen.tsx` | EXTEND (wiring) | Pass `signerKind` / `canSignReason` from `useAdminWalletCapability()` into `useBroadcastProposal` and `BroadcastDetailsCard`. Route composition only (no business logic), per the React rules. |
| `api/proposals.ts` + `api/ipc-schemas.ts` | EXTEND | `broadcastProposal` already returns `ApiResult` whose `error` is the raw reject string — feed that string to `deriveBroadcastError` in the controller. Extend the `admin_wallet_can_sign` wrapper / add a Zod schema for the new `{ canSign, signerKind, reason? }` DTO if the backend adds them. Happy-path result schemas unchanged. |
| `domain/cancel-proposal/hooks/use-cancel-broadcast.ts` | VERIFY (likely small change) | Spreads `useBroadcastProposal`'s return; once `error` becomes `BroadcastError | null`, the cancel-broadcast consumers that read `error` as a string must read `error?.message`. Audit and adjust the 1–2 call sites. |

### Device-interaction UX

> **Slice boundary (NIT-1, ties to §"Wallet-status / canSign contract").** The device affordance and the
> `signerKind` / `canSignReason` capability fields land **together in slice (b)** — they are introduced as
> one unit so the prompt always has a signer kind to branch on. In **slice (a)** the device prompt is
> **not rendered at all**: `signerKind` / `canSignReason` are absent from the capability surface (the
> `admin_wallet_can_sign` command still returns a bare `bool`), so slice (a) has nothing to branch on and
> **MUST NOT attempt to branch on `signerKind`**. Slice (a) therefore reproduces **identical-to-today modal
> behavior on regtest via the simulated-HW (mnemonic) path**; the "Confirm on your device" affordance and
> the `awaiting-device` state arrive in slice (b) alongside the `signerKind` capability field.
>
> **Fail-safe rule (applies in both slices):** if `signerKind` is unavailable the controller treats the
> flow as the mnemonic / instant path — **no device affordance**. In slice (a) `signerKind` is always
> unavailable, so this rule deterministically yields today's behavior.

- **HW path (slice (b))**: when `confirm()` runs and `signerKind === 'hardware'`, the controller enters
  `awaiting-device` immediately (before the IPC resolves) and `BroadcastDevicePrompt` shows
  "Confirm on your device". Because the single IPC blocks through signing, this coarse state remains until
  the call resolves to `confirmed` or an error. No Rust→JS event channel is needed for R1.1 (Open note (a)).
- **Mnemonic / simulated-HW path**: when `signerKind === 'mnemonic'` (slice (b)) **or `signerKind` is
  absent (slice (a))**, the controller **skips** the `awaiting-device` state (the signer returns
  instantly, or there is no kind to branch on). The card behaves exactly as R1.0.1 — this is a hard
  requirement so slice (a) ships with zero UI regression.
- **Graceful degradation**: if `signerKind` is unavailable (slice (a), or an older backend in slice (b)),
  default to **not** showing the prompt (fail safe toward the unchanged mnemonic behavior). This is the
  same fail-safe rule above, made explicit for forward/backward compatibility.

### Button / title messaging

`disabled={isBroadcasting || !canSign}` is **unchanged** (the design win — the control already auto-enables
when `can_sign` flips true; verified in `broadcast-details-card.tsx` line ~158). What improves is the
*reason* shown when `canSign === false`:

- Today the card renders a single hard-coded string `Hardware wallet required to sign` (and the same on the
  retry control in `broadcast-proposal-screen.tsx`).
- With `canSignReason` from the status DTO, the disabled tooltip becomes specific:
  - `not-allowed-on-network` → "This signer is not allowed on the current network. Use a hardware wallet for mainnet."
  - `watch-only-no-signer` → "Connect a hardware wallet to sign and broadcast."
  - `no-session` → "Connect a wallet to broadcast."
- **Recommended approach**: carry a machine-readable `canSignReason` in the wallet-status DTO
  (see next section) rather than inferring in the controller. Reason: the backend already knows *why*
  (`signer.allowed_on(network)` vs no signer vs no session); re-deriving it in TS would duplicate the
  capability rule (DDD-3) and risk drift. The controller stays a thin consumer.

### Copy strings (English, authoritative)

- Disabled tooltip (network): "This signer is not allowed on the current network. Use a hardware wallet for mainnet."
- Disabled tooltip (watch-only): "Connect a hardware wallet to sign and broadcast."
- Disabled tooltip (no session): "Connect a wallet to broadcast."
- Device prompt title: "Confirm on your device"
- Device prompt body: "Review the transaction on your hardware wallet and approve it to continue."
- Error messages: per the §DDD-8 table (one per `code`).
- Success state: "Broadcast confirmed." (commit + reveal txids shown as today).

All copy obeys the repo TS/React conventions (tabs, single quotes, ~120 cols, strict equality, kebab-case
filenames, PascalCase components, camelCase hooks, Zod-parsed IPC at the boundary).

---

## Wallet-status / canSign contract

`admin_wallet_can_sign` today returns a bare `bool` (`wallet_session.can_sign()`), and `getAdminWalletInfo`
merges it into `AdminWalletInfo.canSign`. Under R1.1:

- It returns `true` for an HW session whose `HwPsbtSigner` is attached and `allowed_on(active_network)`;
  `true` for a mnemonic session on regtest/testnet; **`false`** for watch-only/no-signer, no session, or a
  signer not allowed on the active network (e.g. mnemonic on mainnet). This falls directly out of
  `WalletService::can_sign` (D-locked) — no new rule. (Verified: the backend command
  `admin_wallet_can_sign` returns `wallet_session.can_sign()`; the existing test
  `admin_wallet_can_sign_returns_false_after_watch_only_init` continues to hold — a watch-only session with
  no attached signer is still `false`, and only attaching `HwPsbtSigner` flips it true.)
- **Recommendation (resolves Open question) — DTO fields land in slice (b) (NIT-1):** the `signerKind` /
  `reason` capability fields are added in **slice (b)** together with the device prompt; in **slice (a)**
  the command keeps returning the bare `bool` and the UI has no `signerKind` to branch on (slice (a) MUST
  NOT branch on `signerKind`). Evolve the command (`admin_wallet_can_sign` returns a bare
  `bool` today, consumed by `getAdminWalletCanSign()` in `api/admin-wallet.ts`) to return a small DTO
  instead of a bare bool, e.g. `{ canSign: bool, signerKind: 'hardware'|'mnemonic'|'none', reason?: 'not-allowed-on-network'|'watch-only-no-signer'|'no-session' }`.
  The `reason` is `Some` only when `canSign === false`. This is the single source of truth for both the
  device-prompt branch (`signerKind`) and the disabled tooltip (`reason`), keeping the capability rule on
  the backend. The change is additive and Zod-parsed; the legacy bare-bool is still accepted (graceful
  degradation → `signerKind: 'none'`, no specific reason). Naming the command `admin_wallet_sign_status`
  (new) vs overloading `admin_wallet_can_sign` is an implementer choice; either keeps the bool meaning.

---

## Module structure

- `application/psbt_signer.rs` (**new**) — `PsbtSigner` port + `MnemonicPsbtSigner` (software, simulated
  HW). Application layer; depends inward only.
- `infrastructure/hw_wallet/hw_psbt_signer.rs` (**new**) — `HwPsbtSigner` adapter; wraps the synchronous
  Trezor/Ledger client in `spawn_blocking`; depends on `hw_wallet/{trezor,ledger}.rs`.
- `application/wallet_service.rs` (**extend**) — split `build_psbt` / `sign`; hold
  `Option<Arc<dyn PsbtSigner>>`; `with_signer`; new `can_sign`.
- `application/wallet_session.rs` (**extend**) — attach the right signer per login type at init; new
  HW error variants.
- `application/commit_funding.rs` (**no signature change**) — `CommitFunding` /
  `BdkAdminWalletMnemonic` keep their surface; routes through the session signer transparently.
- `infrastructure/hw_wallet/{trezor,ledger}.rs` (**extend**) — add `sign_psbt` (taproot key-path).
- `infrastructure/broadcast_env.rs` (**extend**) — drop `allow_dev_mnemonic_signing`.
- `application/proposals.rs` (**no change**) — downstream orchestration unchanged.
- `commands/proposals.rs` (**extend**) — `map_broadcast_error` now classifies into a `{ code, message }`
  structured error (DDD-8); new pure helper `broadcast_error_code`. IPC happy-path contracts unchanged.

### Frontend (desktop-app/src/domain/broadcast-proposal) — single responsibility per file

- `model/broadcast-proposal.ts` (**extend**) — *Error/phase domain model*: `BroadcastErrorCode` /
  `BroadcastRecovery` types, `deriveBroadcastError` (parses `{ code, message }`, maps to copy + recovery),
  `'awaiting-device'` phase.
- `hooks/use-broadcast-proposal.ts` (**extend**) — *Broadcast controller*: recovery-driven `canResubmit`
  (latent-bug fix), `awaiting-device` transient phase, `isAwaitingDevice`.
- `components/broadcast-details-card.tsx` (**extend**) — *Broadcast card UI*: recovery-gated resubmit,
  device prompt mount, kind-specific error copy; `disabled={!canSign || inFlight}` unchanged.
- `components/broadcast-device-prompt.tsx` (**new**) — *"Confirm on your device" affordance*; presentational
  only, HW path only.
- `components/broadcast-phase-progress.tsx` (**extend, minor**) — rank `'awaiting-device'` as commit-active.
- `domain/admin-wallet/hooks/use-admin-wallet-capability.ts` (**extend**) — *real canSign source*
  (`useAdminWalletCapability`): surfaces `signerKind` + `canSignReason`.
- `screens/broadcast-proposal-screen.tsx` (**extend, wiring only**) — passes `signerKind`/`canSignReason`
  into the controller + card (route composition only).
- `api/proposals.ts` + `api/ipc-schemas.ts` (**extend**) — parse structured error + optional wallet-status
  DTO fields; happy-path schemas unchanged.

Frontend dependency direction: **UI (component) → controller (hook) → api (`invoke` wrapper) → Tauri command
→ application**. Components stay declarative; IPC lives only in `api/*`; the capability rule (DDD-3) stays on
the backend and the frontend consumes `canSign` / `signerKind` / `reason` as data.

Dependency direction: ports live in `application/`; concrete signers live with their substrate
(`MnemonicPsbtSigner` in application because it is pure BDK; `HwPsbtSigner` in infrastructure because it
touches a device). `WalletService` depends on the `PsbtSigner` abstraction, never on a concrete signer.

---

## Open notes for the implementer

- **Taproot key-path PSBT feasibility on Trezor/Ledger.** Confirm the pinned `trezor_client` / Ledger
  app versions support BIP-86 key-path PSBT signing and return the taproot key-spend signature in the
  PSBT. The commit funds BIP-86 key-path inputs — script-path is not needed here.
- **Master-fingerprint capture at connect time.** Capture the HW **master** fingerprint at connect time,
  alongside the account xpub in `auth_complete` / `init_from_xpub`, and store it on the session and on
  `HwPsbtSigner`. Do **not** use the xpub's `parent_fingerprint` — it is the parent's fingerprint, not the
  master's. The watch-only descriptor must embed the correct origin fingerprint (`[<master_fp>/86'/...]`)
  so the device recognizes its PSBT inputs; a wrong origin fingerprint makes the device refuse to sign
  (correctness dependency for D7).
- **Fingerprint-based device re-open.** `HwPsbtSigner` holds the master fingerprint (captured at connect),
  not a live connection; it re-opens the device at sign time and verifies the fingerprint matches before
  signing. Surface a clear error if a different device is plugged in.
- **PSBT derivation-metadata requirement (D7).** Verify `build_tx` on the watch-only descriptor wallet
  populates taproot internal-key + BIP32 derivation fields. If any field is missing the device will
  refuse; add a guard/assert in `build_psbt` if needed.
- **`BdkAdminWalletMnemonic` rename — do it in slice (a).** Once it routes through a session signer (which
  may be HW), the name actively misleads. **Verdict: rename to `AdminWalletCommitFunding` during step (a)**
  — the file is already being touched in (a) and the name is actively wrong the moment `HwPsbtSigner` can
  be attached. The `CommitFunding` port signature does not change.
- **`ALLOW_DEV_MNEMONIC_SIGNING` removal — explicit file checklist.** Remove the variable and every
  reference to it from each of the following; replace doc references with the per-signer network capability:
  - [ ] `.env.example`
  - [ ] `desktop-app/e2e-webdriver/README.md`
  - [ ] CI workflow YAML (GitHub Actions env)
  - [ ] `render.yaml`
  - [ ] `staging/docker-compose.yml`
  - [ ] `dev_secrets.rs` (if it references the flag)
  - [ ] `wallet_service.rs::check_enabled` (code site — delete the gate)
  - [ ] `broadcast_env.rs::load_broadcast_env` Gate 1 / `BroadcastEnvError::MnemonicSigningDisabled` (code
        site — delete the gate and the error variant)
- **Slicing (D6).** Land slice (a) first — `PsbtSigner` + `MnemonicPsbtSigner` + flow unification + flag
  removal (walking skeleton, regtest, no device) — then slice (b) `HwPsbtSigner` real device signing.
  Both ship under R1.1; (a) de-risks (b).
- **DELIVER — spike slice (b) first.** Start step (b) with a short spike to confirm the installed
  `trezor-client` / Ledger crate versions actually expose **taproot key-path** PSBT signing (and return the
  key-spend signature in the PSBT) before committing to the full on-device flow. If the pinned versions do
  not support it, that is a go/no-go gate for (b) — surface it before building out the adapter path.

### Frontend / end-to-end open notes

- **(a) Device progress: coarse pending state vs event channel.** `proposals_broadcast` is a single awaited
  IPC call that blocks through device signing; the frontend cannot get intra-call progress without a
  Rust→JS event channel (e.g. `app_handle.emit`). **Recommendation: a coarse `awaiting-device` state is
  enough for R1.1.** It is derived from `inFlight` + `signerKind === 'hardware'` and needs no new channel,
  no new IPC, and no backend change. A fine-grained event stream (e.g. "device opened", "awaiting button")
  is a Phase-7 nicety, not an R1.1 requirement. Document the trade-off; do not build the channel now.
- **(b) `map_broadcast_error` → `{ code, message }` migration touches three files.** The change is
  localized: `commands/proposals.rs` (classify + emit JSON; new `broadcast_error_code` helper), and the two
  TS consumers `model/broadcast-proposal.ts` (`deriveBroadcastError` parses it) and — transitively —
  `hooks/use-broadcast-proposal.ts` (recovery-gated `canResubmit`). Keep it backward compatible: a bare
  legacy string must still parse to `{ code: 'Unknown', message }`. This belongs in **slice (a)** — it is
  device-independent and fixes the latent resubmit bug regardless of HW.
- **(c) Mnemonic / simulated-HW path must be visually unchanged in slice (a).** The simulated-HW (mnemonic)
  signer returns instantly, so the `awaiting-device` state is never observed and `BroadcastDevicePrompt` is
  never mounted. Slice (a) ships the structured error + resubmit-gating fix + button-auto-enable
  verification on regtest with **zero device**, and the broadcast card must render and advance exactly as
  R1.0.1. Test FE-06 guards this; treat any visual delta on the mnemonic path as a slice-(a) regression.
- **Resubmit-reveal latent bug is a slice (a) fix.** `canResubmit: Boolean(error)` →
  `error?.recovery === 'resubmit-reveal'`. This is device-independent (the bug is reachable today on the
  mnemonic path for any non-resubmittable error) and must not wait for slice (b).
- **Button auto-enable is a regression obligation, not new code.** `disabled={!canSign || inFlight}` already
  enables the control when `can_sign` flips true for an HW session. No new gating is written; Test FE-01 is a
  characterization test guarding the win.

---

## Changed Assumptions

The original implementation plan (§6 Configuration) stated:

> `ALLOW_DEV_MNEMONIC_SIGNING` | `false` | Dev-only | Gate dev signing on regtest. **Keep.**

R1.1 **removes** this variable. Rationale: the env flag conflated "is signing enabled" with "is this an
acceptable network for this key material". A software mnemonic is a hot key held in memory; permitting it
on mainnet would violate PRD §3.2 (all signing hardware-mediated). The replacement is a typed,
per-signer network capability — `MnemonicPsbtSigner.allowed_on` = regtest | testnet only;
`HwPsbtSigner.allowed_on` = any network. On mainnet only the real hardware signer is permitted, while the
unified flow (same port, same steps) is preserved end-to-end.

---

## Slicing — UI folded into D6 (no change to D6 itself)

The frontend work distributes across the two existing R1.1 slices; D6 is **extended, not contradicted**:

- **Slice (a) — device-independent, fully shippable on regtest with the simulated-HW (mnemonic) path and
  ZERO device:** `PsbtSigner` port + `MnemonicPsbtSigner` + flow unification + `ALLOW_DEV_MNEMONIC_SIGNING`
  removal (all D-locked) **plus** the structured broadcast error contract (DDD-8), the
  `deriveBroadcastError` rewrite + recovery-gated resubmit fix (DDD-9), and the button-auto-enable
  regression test. The broadcast card behaves byte-identically to R1.0.1 (no device prompt). The
  structured-error contract and the resubmit-gating fix **belong in slice (a)** because they are
  device-independent and fix a latent bug that is reachable today.

  **What slice (a) does and does not ship for resubmit (NIT-4):** slice (a) introduces the
  recovery-driven **gating data contract** (`canResubmit = error?.recovery === 'resubmit-reveal'`) **and
  its unit tests** (FE-04). It does **not** wire a visible "Resubmit reveal" control into
  `broadcast-details-card.tsx` — no such control exists there today. The visible resubmit affordance, if
  and when designed, is a **later slice**; slice (a) prevents the latent bug **structurally** via the data
  contract so any future control can only ever be gated on `recovery`, never on error presence. The
  `proposals_resubmit_reveal` IPC stays **as-is** (it already exists from R1.0.1) — slice (a) adds no new
  IPC for resubmit.
- **Slice (b) — adds the on-device pieces:** real `HwPsbtSigner` device signing **plus** `signerKind`/
  `canSignReason` in the wallet-status DTO, `BroadcastDevicePrompt`, the `awaiting-device` controller state,
  and the network/watch-only-specific disabled tooltips. The manual real-device test procedure (REG-01)
  is part of slice (b)'s release checklist.

Both ship under R1.1; (a) de-risks (b) and independently improves error UX and fixes the resubmit bug.
