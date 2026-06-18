# Evolution: Admin Wallet — Ephemeral Reveal Key (R1.0)

**Date:** 2026-05-30
**PR:** #195
**Branch:** feature/admin-wallet-ephemeral-reveal-key
**Spec:** docs/specs/admin-wallet-ephemeral-reveal-key.md

## Summary

R1.0 replaced the seed-derived SPS-50 envelope keypair (previously cached at `m/86'/0'/73'/2/0` in
`WalletSession`) with a per-broadcast ephemeral key generated in-memory via `OsRng`. The reveal
change output was redirected from the throwaway ephemeral key's own address to the Admin Wallet's
rotating internal keychain (`reveal_next_address(Internal)`), preventing stranded funds. The
`BroadcastEnv` struct was stripped of keypair fields while its three signing gates
(`MnemonicSigningDisabled`, `WalletSessionRequired`, `ReadOnly`) were preserved. A frontend label
change (`"Commit TX"` → `"Commit TX (preview)"`) aligns the UI with the fact that the shown
commit address is ephemeral and not the final on-chain address.

## Business Context

Replace the seed-derived SPS-50 envelope keypair with a per-broadcast ephemeral key to:

- Make reveal signing login-agnostic (no dependency on the session mnemonic for the envelope key)
- Redirect reveal change to the Admin Wallet (no stranded funds on throwaway key)
- Shrink R1.1 scope to "session signer signs only the commit-funding tx"

## Steps Completed

| Step ID | Name | Status |
|---------|------|--------|
| 01-01 | Ephemeral envelope key module | PASS |
| 01-02 | broadcast_tx rename and change_spk param | PASS |
| 01-03 | wallet_service reveal_change_address | PASS |
| 02-01 | proposals ephemeral key and change_spk wiring | PASS |
| 02-02 | broadcast_env remove keypair fields | PASS |
| 02-03 | wallet_session cleanup and mod update | PASS |
| 03-01 | commands proposals wiring | PASS |
| 04-01 | Broadcast details card preview label | PASS |

All 8 steps executed and committed on 2026-05-30.

## Key Decisions

- Per-broadcast ephemeral key via `OsRng` — never persisted, never seed-derived. The envelope key
  is not custody-significant; governance authority lives in the SPS-65 `SignatureSet` inside the
  reveal payload.
- Reveal change redirected via `WalletService::reveal_change_address()` which calls
  `wallet.reveal_next_address(KeychainKind::Internal)`, advancing the Admin Wallet's internal
  keychain index on each broadcast.
- Three signing gates preserved in `load_broadcast_env` despite keypair removal: `MnemonicSigningDisabled`,
  `WalletSessionRequired`, and `ReadOnly`/watch-only checks continue to gate broadcast capability.
- `commands/proposals.rs` is the sole wiring point for `WalletService` → change address →
  proposals layer. No other layer calls `reveal_change_address()`, keeping the dependency
  boundary clean.

## Files Changed

**Production files:**
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/ephemeral_envelope_key.rs` — new module with `generate_ephemeral_envelope_keypair()`
- `desktop-app/src-tauri/src/infrastructure/broadcast_tx.rs` — renamed `operator_keypair` → `envelope_keypair`; added `change_spk: ScriptBuf` param to `build_reveal_tx`
- `desktop-app/src-tauri/src/application/wallet_service.rs` — added `reveal_change_address()` method
- `desktop-app/src-tauri/src/application/proposals.rs` — ephemeral key generated inline; `reveal_change_spk: ScriptBuf` param replaces keypair arg
- `desktop-app/src-tauri/src/infrastructure/broadcast_env.rs` — removed `commit_reveal_keypair` and `operator_keypair` fields; removed `resolve_commit_reveal_keypair`
- `desktop-app/src-tauri/src/application/wallet_session.rs` — removed `commit_reveal_keypair` from `SessionState` and accessor
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/mod.rs` — replaced `mod commit_reveal_key` with `mod ephemeral_envelope_key`
- `desktop-app/src/domain/broadcast-proposal/components/broadcast-details-card.tsx` — relabeled `"Commit TX"` → `"Commit TX (preview)"`

**Test files:**
- Tests added in: `ephemeral_envelope_key.rs`, `broadcast_tx.rs`, `wallet_service.rs`, `proposals.rs`, `commands/proposals.rs`

**Deleted files:**
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/commit_reveal_key.rs`
- Tests deleted: `init_stores_commit_reveal_keypair_matching_derivation`, `commit_reveal_keypair_none_when_slot_empty`, `build_session_from_xpub_returns_none_keypair`, `load_broadcast_env_uses_session_commit_reveal_key`

## Known Limitations (R1.0)

- **R1.0.1 not included**: commit and reveal are not pre-signed before broadcast. A crash between
  commit confirmation and reveal construction strands the commit dust + fee. The ephemeral key
  lives across the commit→reveal window; loss on crash is bounded to commit dust + fee.
- **R1.1 not included**: commit funding is still software-signed by the mnemonic session
  (`BdkAdminWalletMnemonic` → `WalletService::fund_commit`). Watch-only and hardware-wallet
  sessions remain `ReadOnly` for broadcast.

## Lessons Learned

- RED_ACCEPTANCE was skipped for all structural slices (no `.feature` files existed for the
  affected modules); RED_UNIT served as the specification gate throughout. For future structural
  refactors without prior Cucumber coverage, this pattern is intentional and acceptable.
- Step 02-02 (remove keypair fields) and 02-03 (cleanup/delete) had no RED_UNIT phase because
  the work was deletion of existing passing tests — the compile check after deletion was the
  effective red signal. This pattern is sound for pure-removal slices and can be documented as
  a standard approach in the delivery workflow.
- The four-phase structure (new infrastructure → application layer → command wiring → frontend)
  cleanly sequenced dependencies: each phase compiled and tested before the next was started,
  avoiding mid-stream breakage.
