# Spec: Hardware Wallet Send + Verify-on-Device (Phase 8)

> SDD spec for Phase 8 of [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md).
> Authority scope: **Strata Administrator** and **Alpen Administrator** only (per the program scope). PRD refs: §3.2 (HW signing), §4.2 (Admin ID verify-on-device), §4.3.4.2 (receive verify-on-device), §4.3.5 (Send).

## Objective

Let a **Trezor** or **Ledger** signer complete the Admin Wallet **Send** flow (PRD §4.3.5) end-to-end on their device — no mnemonic — and verify a displayed address **on the device screen** before trusting it, for both the **receive address** (§4.3.4.2) and the **Admin ID** (§4.2).

Phase 6 already delivers Send for the dev-mnemonic (software) signer, and Release 1 (R1.1) already delivers **Ledger** on-device PSBT signing for governance broadcast — which the Send path reuses through `WalletService::sign_and_finalize_psbt`. So the actual remaining work is:

1. **Trezor** Admin Wallet PSBT signing (today a stub — blocks both Trezor Send and Trezor governance broadcast).
2. **Verify-on-device** for the receive address and Admin ID, dispatched to whichever device is connected, with the correct script type and network.
3. The UI that drives both, consistent with the Phase 6 mnemonic Send experience.

The implementing agent has latitude over the internal approach as long as the behavior below holds and the existing signing seam (`PsbtSigner` port / `sign_and_finalize_psbt`) is reused rather than duplicated.

## Scope

**In scope**

- Trezor taproot key-path PSBT signing for the Admin Wallet, wired into the existing `sign_and_finalize_psbt` HW branch so the Phase 6 Send use-case works unchanged for a Trezor session.
- Verify-on-device for the **receive address** (BIP-86 / P2TR) and the **Admin ID** (BIP-84 / P2WPKH), for both Trezor and Ledger, on the active network (regtest/testnet/mainnet path conventions already in the adapters).
- The `verify_address_on_device` IPC command dispatches to the **connected device type** and the **correct script type + network** (today it is Trezor-only, mainnet/P2WPKH-only).
- UI: Send enabled for HW sessions with a "confirm on your device" / timeout / rejection experience consistent with Phase 6; "Verify on device" affordance on the receive-address and Admin ID rows.

**Out of scope**

- HWI CLI and any POC-miniwallet integration path.
- Governance broadcast signing redesign (delivered in R1.0/R1.1) — Trezor broadcast benefits automatically once Trezor PSBT signing exists, but no protocol change.
- Mainnet hardening / removing the dev-mnemonic guard (Phase 10).
- Payout Administrator flows.
- Any new custody key, derivation path, or protocol change.

## Technical Design

Work concentrates on three seams. Concrete signatures are illustrative — the agent may adjust names and shapes to fit the code, provided the contracts and the reuse of the existing signing port are preserved.

### 1. Trezor Admin Wallet PSBT signing

- `infrastructure/hw_wallet/trezor.rs::sign_admin_wallet_psbt(...)` — replace the stub with a real taproot (BIP-86, P2TR key-path) PSBT signing implementation against the `trezor_client` library, mirroring the responsibilities of the Ledger equivalent (`ledger::sign_admin_wallet_psbt`): verify the device fingerprint matches the session, sign every wallet-owned input, apply the signatures back onto the PSBT.
- `application/wallet_service.rs::sign_and_finalize_psbt` — the HW branch currently hard-errors for any non-Ledger device. Generalize it so a Trezor signer is routed to the Trezor PSBT path on a blocking thread, with the same 180s timeout / device-rejection mapping already used for Ledger. The mnemonic and Ledger paths must remain behaviorally unchanged.
- No change to the Phase 6 `wallet_send.rs` use-case — it already signs through this port. (Confirm by test, not by edit.)

### 2. Verify-on-device

- Adapter functions to confirm a derivation path's address on the device screen for both script types:
  - Receive: BIP-86 / P2TR at the wallet's external keychain index.
  - Admin ID: BIP-84 / P2WPKH at `m/84'/.../73'/0/0`.
  - Ledger needs a new verify-address function (it has none today); Trezor has `verify_address_on_device` but it is hardcoded to mainnet + P2WPKH and must become network- and script-type-aware.
- The `verify_address_on_device` Tauri command must dispatch to the **connected device** (Trezor or Ledger) — today it unconditionally calls Trezor.

### 3. UI

- Send: for an HW session the Confirm step prompts the user to approve on the device, with a pending state, a timeout surface, and an on-device-rejection surface, reusing the Phase 6 result/reject/retry surfaces and the existing typed `SendError` codes (`SignFailed` is the on-device rejection branch — see `wallet_send.rs`).
- Verify-on-device affordance on the receive-address row (§4.3.4.2) and the Admin ID row (§4.2), shown only for HW sessions, surfacing match/mismatch/rejection.

### Production code vs. test helpers

- **Production**: the Trezor PSBT signing fn, the device-dispatching verify-address fn(s), the generalized `sign_and_finalize_psbt` HW branch, the dispatching `verify_address_on_device` command, and the React Send/verify wiring.
- **Test helpers**: PSBT/wallet fixtures, mock signers (e.g. the existing `MnemonicPsbtSigner` in tests), and any emulator harness scaffolding live in `#[cfg(test)]` or test-only modules — never registered as Tauri commands or exposed in production APIs.

## Test Cases

### Testing strategy (evidence-based)

**Hardware-device behavior is validated manually — it is not, and will not be, covered by automated CI.** Evidence from the current repo:

- No `#[ignore]` tests and no `SPECULOS_*` / `LEDGER_*` / `TREZOR_*` env-gated tests exist in the Rust suite.
- The only HW-touching unit tests assert **pure logic**: the `allowed_on` network-capability matrix (`hw_psbt_signer.rs`, `psbt_signer.rs`) and the Ledger coin-type path mapping (`commands/hw_wallet.rs`). None drives a device or emulator.
- CI (`.github/workflows/ci.yml`, `release.yml`) contains **zero** references to Ledger/Trezor/Speculos/emulator. The desktop WebDriver E2E suite logs in with the **mnemonic** ("Palabras") signer; `proposal-broadcast-quorum.e2e.js` even asserts that a device prompt does **not** appear on the mnemonic path.
- Device emulators are brought up by **local developer scripts** (`scripts/ledger-up.sh` for Speculos; `scripts/trezor-up.sh` / `trezor-down.sh` for the Trezor emulator), i.e. a manual/local activity. Ledger's existing on-device PSBT path (`ledger.rs`) already assumes this manual Speculos harness.

Therefore Phase 8 splits its checks into two tiers:

1. **Automated (CI, no device):** everything that does not require a physical/emulated device — signer dispatch in `sign_and_finalize_psbt`, verify-address command dispatch by device type, error/code mapping, the "nothing broadcast when signing fails" guard (using a mock/software signer), and the Phase 6 Send-contract regressions on the mnemonic and (mock-signer) paths.
2. **Manual (documented, against emulator or real device):** the actual on-device Trezor PSBT signing and the verify-on-device match/mismatch/rejection flows. These ship with a written, reproducible manual test playbook (which `*-up.sh` to run, login steps, expected on-screen prompts, pass/fail criteria) — the same way Ledger broadcast signing is exercised today. They are **not** added to CI.

### Automated (CI)

- **Signer dispatch** — `sign_and_finalize_psbt` routes a Trezor signer to the Trezor PSBT path and a Ledger signer to the Ledger path; the previous Trezor "not implemented" hard-error is gone. Asserted at the seam, no device.
- **Reuse / no regression** — a mnemonic (mock) session still sends successfully through the unchanged port; the Trezor change must not alter the mnemonic/Ledger branches. Phase 6 Send-contract assertions (recipient amount, change to first unused internal index, RBF-signaling inputs) stay green.
- **Verify dispatch** — `verify_address_on_device` calls the Trezor adapter for a Trezor session and the Ledger adapter for a Ledger session; the receive path selects P2TR and the Admin ID path selects P2WPKH; the active network is honored. Asserted at the dispatch boundary.
- **Expected errors → no broadcast** — a failing signer (device absent, locked, fingerprint mismatch, user rejection — simulated by a stub signer returning the corresponding error) maps to a typed, high-signal error and **nothing is broadcast** (the broadcaster is never contacted, mirroring the existing `wallet_send` guard tests).
- **Authority isolation** — dispatch only ever targets the Admin Wallet (`m/86'/.../73'`) and Admin ID (`m/84'/.../73'/0/0`) paths; no other authority key is consulted.
- **Watch-only fallback** — a session with no signing capability still cannot Send (`ReadOnly`), unchanged from Phase 6.

### Manual (device / emulator, documented playbook)

- **Trezor happy path** — Trezor session sends BTC on regtest: PSBT built by BDK, reviewed and signed **on device**, finalized, broadcast; resulting tx matches the Phase 6 contract.
- **Ledger happy path (regression)** — Ledger session still sends end-to-end via Speculos (confirms the generalized dispatch did not break the existing Ledger path).
- **Verify-on-device** — receive address (P2TR) and Admin ID (P2WPKH) each display on the device screen; matching address → confirm, tampered/mismatched → surfaced, user-rejection → surfaced.
- **On-device rejection / timeout** — rejecting or letting the prompt time out surfaces the Phase 6 reject/retry experience and broadcasts nothing.

## Module structure

Single responsibilities (one sentence each):

- `infrastructure/hw_wallet/trezor.rs` — Trezor device protocol, **including** taproot Admin Wallet PSBT signing and network/script-aware address verification.
- `infrastructure/hw_wallet/ledger.rs` — Ledger device protocol, **including** a new address-verification function alongside the existing PSBT signing.
- `application/wallet_service.rs::sign_and_finalize_psbt` — selects and drives the session signer (mnemonic / Trezor / Ledger) for any wallet-built PSBT; it is the one place that knows about device dispatch for signing.
- `commands/hw_wallet.rs` — thin Tauri command layer; `verify_address_on_device` dispatches to the connected device.
- React `domain/admin-wallet/` — Send confirm + verify-on-device UI; presentation only, no secrets.

**Dependency direction:** `wallet_service` (application) depends on the `PsbtSigner` abstraction (application port), and the device adapters (infrastructure) implement / are driven by it — business logic depends on the port, not on a concrete device. The Trezor PSBT signer must not introduce an inward dependency from the port to infrastructure beyond the existing `HwPsbtSigner` metadata pattern.

## Decisions & open questions

**Settled by evidence (not open):** Trezor/Ledger device behavior is **not** added to CI. Device validation is manual against emulators (`scripts/*-up.sh`) or real hardware, consistent with how Ledger broadcast signing is exercised today (see Testing strategy). CI covers only the no-device seams. The deliverable therefore **must include a written manual test playbook** for the on-device paths.

**Agent's discretion:**

- The internal shape of the Trezor PSBT signing implementation (how it drives `trezor_client` for a taproot key-path spend), provided it mirrors the Ledger adapter's responsibilities (fingerprint check → sign wallet-owned inputs → apply signatures) and runs on a blocking thread.
- Whether to refactor the signer dispatch in `sign_and_finalize_psbt` into a per-device strategy now or keep the branch and revisit in Phase 9's shared-Send work.
- Whether a Trezor-emulator auto-approve convenience (analogous to the optional Speculos `/automation` helper in `ledger.rs`) is worth adding for local manual runs — optional, never a CI gate.
