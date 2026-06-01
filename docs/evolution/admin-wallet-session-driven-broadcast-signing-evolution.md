# Evolution: Admin Wallet Session-Driven Broadcast Signing (R1.1)

**Feature ID:** admin-wallet-session-driven-broadcast-signing
**Status:** Complete
**Date:** 2026-06-01

## Summary

Implemented R1.1 of the Admin Wallet — session-driven broadcast signing with a unified `PsbtSigner` port supporting both mnemonic (simulated HW) and real hardware wallet (Trezor/Ledger) signing paths.

## What Changed

### Backend (Rust)
- **New:** `PsbtSigner` trait (driven port) in `application/psbt_signer.rs`
- **New:** `MnemonicPsbtSigner` (software signer, regtest/testnet only)
- **New:** `HwPsbtSigner` (hardware signer, any network) with `FakeHwDevice` test double
- **Modified:** `WalletService` — split `build_psbt`/`sign`, holds `Option<Arc<dyn PsbtSigner>>`
- **Modified:** `WalletSession` — attaches correct signer per login type
- **Modified:** `commands/proposals.rs` — structured `{code, message}` error contract (DDD-8)
- **Removed:** `ALLOW_DEV_MNEMONIC_SIGNING` env flag (replaced by per-signer network capability)
- **Renamed:** `BdkAdminWalletMnemonic` → `AdminWalletCommitFunding`

### Frontend (TypeScript/React)
- **New:** `BroadcastDevicePrompt` component ("Confirm on your device")
- **New:** `deriveBroadcastError` — parses structured JSON errors, maps to recovery actions
- **New:** `BroadcastError` view-model with `code`, `message`, `recovery`
- **Modified:** `useBroadcastProposal` — recovery-gated `canResubmit`, `awaiting-device` phase
- **Modified:** `useAdminWalletCapability` — surfaces `signerKind` + `canSignReason`
- **Modified:** `BroadcastDetailsCard` — device prompt mount, kind-specific error copy
- **Modified:** `BroadcastPhaseProgress` — ranks `awaiting-device` as commit-active

## Architecture Decisions (DDD-1 through DDD-9)

All 9 domain-driven decisions from the DESIGN wave were implemented and locked:
- DDD-1: `PsbtSigner` driven port on `WalletService`
- DDD-2: Two implementors behind same port
- DDD-3: Remove `ALLOW_DEV_MNEMONIC_SIGNING`, replace with `allowed_on(network)`
- DDD-4: HW device access via `spawn_blocking`, re-open by fingerprint
- DDD-5: Reveal unchanged — signed by ephemeral envelope key
- DDD-6: Slice R1.1 into (a) port + mnemonic + flag removal, (b) HW signer
- DDD-7: PSBT carries taproot derivation metadata
- DDD-8: Structured broadcast error contract
- DDD-9: Frontend device-UX + error surfacing

## Test Coverage

- **31 roadmap steps** executed with TDD (PREPARE → RED → GREEN → COMMIT)
- **72 Rust tests** passing (clippy clean, fmt clean)
- **12+ TypeScript tests** passing (lint clean, build clean)
- **E2E:** Mnemonic walking skeleton (regtest, no device)
- **Release checklist:** 6 manual real-device test paths documented

## Quality Gates

| Gate | Status |
|------|--------|
| Roadmap created + approved (3 iterations) | ✅ |
| All steps COMMIT/PASS (5-phase TDD) | ✅ |
| L1-L4 refactoring complete | ✅ |
| Adversarial review passed (1 iteration + fixes) | ✅ |
| Mutation testing | ⚠️ Skipped (timeout) |
| All tests passing | ✅ |
| Clippy + fmt + lint + build | ✅ |

## Commits

35 commits total (31 roadmap steps + 4 refactoring/review commits).
See `git log --oneline --grep="Step-ID"` for full list.
