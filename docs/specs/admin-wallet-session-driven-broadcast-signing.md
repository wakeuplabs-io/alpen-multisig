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

---

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
  `WalletError::HwSigningFailed` / `HwDisconnected` returned **before** any broadcast.
- HW device access in the pre-sign window via `tokio::task::spawn_blocking` (the Trezor client is
  synchronous); the device is re-opened by fingerprint at sign time (no live connection held in the
  session).

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
| `WalletError::HwSigningFailed` / `HwDisconnected` | `application/wallet_session.rs` (or shared) | Device absent / user refusal, returned before broadcast |
| `TrezorAdapter::sign_psbt` / `LedgerAdapter::sign_psbt` | `infrastructure/hw_wallet/{trezor,ledger}.rs` | New: taproot key-path PSBT signing (synchronous client) |
| `WalletSession::init_from_mnemonic` / `init_from_xpub` | `application/wallet_session.rs` | Attach `MnemonicPsbtSigner` / `HwPsbtSigner` per login type |

---

## Test Cases

1. **Happy path — mnemonic-mock (simulated HW), regtest.** Mnemonic login → `MnemonicPsbtSigner`
   attached → `build_signed_commit` produces a valid extractable commit tx; broadcast completes
   commit → reveal → confirmed.
2. **Happy path — HW on-device.** HW login → `HwPsbtSigner` attached → `sign_psbt` runs inside
   `spawn_blocking`, re-opens device by fingerprint, taproot key-path signs; commit finalizes and
   broadcasts.
3. **Signer selection unit tests.** `init_from_mnemonic` attaches `MnemonicPsbtSigner`;
   `init_from_xpub` attaches `HwPsbtSigner`; `can_sign()` reflects `signer.allowed_on(network)`.
4. **MnemonicPsbtSigner rejected on mainnet — fail fast, no side effects.** `allowed_on(Network::Bitcoin)`
   = false; broadcast on mainnet with a mnemonic signer returns `AdminWalletError::SignerNotAllowedOnNetwork`.
   Assert the `allowed_on` check fires **first**: **no `sync()` / RPC call and no PSBT build occurs** on the
   reject path (the enforcement point short-circuits before any sync/RPC/build), and nothing is broadcast.
5. **MnemonicPsbtSigner accepted on regtest and testnet.** `allowed_on` true for both.
6. **HwPsbtSigner allowed on any network** including mainnet (`allowed_on(Network::Bitcoin)` = true).
12. **Capability matrix — direct `allowed_on` assertions.** Assert the per-signer capability directly:
    `MnemonicPsbtSigner.allowed_on(Network::Testnet) == true`, `allowed_on(Network::Regtest) == true`,
    `allowed_on(Network::Bitcoin) == false`; and `HwPsbtSigner.allowed_on(_) == true` for any network
    (Regtest, Testnet, Bitcoin).
13. **HW session present but no signer attached yet (step-(a)→(b) intermediate window).** A HW session
    exists but no `PsbtSigner` is attached (the window after the unified flow lands in (a) but before
    `HwPsbtSigner` is wired in (b)). `build_signed_commit` returns a typed error (the missing-signer guard,
    e.g. `ReadOnly`) — **not a panic, and never a silent software/mnemonic sign** — and **nothing is
    broadcast**.
7. **Device-absent / user-refusal typed error before any broadcast.** `HwPsbtSigner.sign_psbt`
   surfaces `HwSigningFailed` / `HwDisconnected`; `build_signed_commit` returns the error and
   `broadcast_commit_then_reveal` never reaches the network (commit not sent).
8. **Reveal still ephemeral.** Reveal is signed by the per-broadcast envelope key; assert the reveal
   path never calls `PsbtSigner`.
9. **Regtest e2e — mnemonic-mock walking skeleton.** Full broadcast on the real Tauri binary using the
   "Palabras" login with zero device; commit + reveal confirmed.
10. **Regression — downstream unchanged.** Existing `broadcast_commit_then_reveal` and `CommitFunding`
    tests stay green (port signature and orchestration untouched).
11. **CI no longer needs `ALLOW_DEV_MNEMONIC_SIGNING`.** Remove the variable from CI/e2e env; tests on
    regtest pass because `MnemonicPsbtSigner.allowed_on(regtest)` = true.

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
- `application/proposals.rs`, `commands/proposals.rs` (**no / minimal change**) — downstream unchanged.

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
