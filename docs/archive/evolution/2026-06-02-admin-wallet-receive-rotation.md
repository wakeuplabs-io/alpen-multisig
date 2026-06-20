# Evolution: Admin Wallet — Receive rotation (R1.3)

**Date:** 2026-06-02
**Branch:** `feature/admin-wallet-receive-rotation`
**Commit:** `788d4eb`
**Spec:** [`docs/specs/admin-wallet-receive-rotation.md`](../specs/admin-wallet-receive-rotation.md)
**Predecessor:** R1.2 Clean Wallet UI ([`2026-06-02-admin-wallet-clean-wallet-ui.md`](2026-06-02-admin-wallet-clean-wallet-ui.md))

## Summary

R1.3 makes the Admin Wallet Receive tab issue a fresh, unused external address and rotate to the next unused index once the displayed address is credited (observed in a transaction during sync). A new `WalletService::next_receive_address` method backed by BDK's gap-aware `next_unused_address(KeychainKind::External)` is exposed via the `admin_wallet_next_receive_address` IPC command and a `useAdminWalletReceiveAddress` hook. Both wallet-panel screens now source the receive address from the backend and refresh it after sync, replacing the fragile front-end `find((a) => !a.isUsed)` 20-address window scan.

## Business Context

PRD §4.3.4 requires receive-address rotation so the signer is never shown an already-credited address (address reuse degrades privacy and operational hygiene). Through R1.2 the panel derived the receive address by scanning the first 20 external addresses for the first unused one — a heuristic that broke once all 20 were used and depended on the addresses page being refreshed after sync. R1.3 replaces it with BDK's native gap-aware derivation, which has no window ceiling and is the protocol-idiomatic source of truth for "next unused receive address".

## Deliverable

Single increment delivered via the SDD workflow (spec → branch → red/green TDD → refactor → verification → PR).

| Item | Status |
|------|--------|
| Spec `admin-wallet-receive-rotation.md` | Done |
| `WalletService::next_receive_address` (BDK `next_unused_address`) | Done |
| IPC `admin_wallet_next_receive_address` (registered in both handler sets) | Done |
| FE `nextAdminWalletReceiveAddress` + `useAdminWalletReceiveAddress` hook | Done |
| Wire both panel screens; refresh receive address after sync | Done |
| Remove front-end `find((a) => !a.isUsed)` window scan + redundant addresses IPC | Done |
| Backend tests (fresh/idempotency/watch-only) + IPC tests (state/Disabled) | Done |
| Frontend test (command name/args + hook export) | Done |
| Rust + frontend CI (fmt, clippy -D warnings, test, build, lint) | Green |

## Key Decisions

- **Rotation keyed on BDK "used", not a confirmation threshold.** BDK marks an external index used as soon as a crediting tx is observed during sync (including unconfirmed mempool). This is the idiomatic BDK model and the security-correct behavior (avoid reuse the moment a credit is seen). On regtest the e2e mines to confirm, so the displayed address has already rotated by the time the balance confirms — satisfying the §4.3.4 "done when".
- **Idempotent issue.** `next_receive_address` returns the same address on repeated calls until that address is used; merely opening the Receive tab does not leak indices.
- **Reuse `AddressDto`** — no new DTO; rotation returns `{ index, address, isUsed: false }`.
- **No new abstraction.** Method sits beside `reveal_change_address` on `WalletService`; the IPC layer uses the existing `current_or_fallback()` + `serialize_wallet_error` pattern. Signer port and commit/reveal protocol untouched.
- **Standalone `useAdminWalletAddresses` dropped from both screens** — the receive hook supersedes it for the receive row and Disabled detection; the addresses-with-balance list keeps its own composed hook. Removes one redundant IPC call per screen.

## Files Changed

**Backend (production):**

- `desktop-app/src-tauri/src/application/wallet_service.rs` — add `next_receive_address`
- `desktop-app/src-tauri/src/commands/admin_wallet.rs` — add `admin_wallet_next_receive_address` command
- `desktop-app/src-tauri/src/commands/invoke.rs` — register command in production + dev-signing handler sets

**Frontend (production):**

- `desktop-app/src/api/admin-wallet.ts` — `nextAdminWalletReceiveAddress`
- `desktop-app/src/domain/admin-wallet/hooks/use-admin-wallet-receive-address.ts` (new)
- `desktop-app/src/domain/admin-wallet/hooks/index.ts` — export hook
- `desktop-app/src/screens/proposals-dashboard-screen.tsx` — source receive address from hook; refresh after sync
- `desktop-app/src/screens/broadcast-proposal-screen.tsx` — source receive address from hook; refresh after sync

**Tests:**

- `desktop-app/src-tauri/src/application/wallet_service.rs` — 3 unit tests
- `desktop-app/src-tauri/src/commands/admin_wallet.rs` — 2 tests
- `desktop-app/src/api/admin-wallet-receive-rotation.test.ts` (new)
- `desktop-app/package.json` — new FE test script

**Documentation:**

- `docs/specs/admin-wallet-receive-rotation.md` (new)
- `docs/specs/admin-wallet-implementation-plan.md` (R1.3 marked complete; next → R1.4)

## Known Limitations (post-R1.3)

- **No persisted reveal index:** the BDK wallet is `create_wallet_no_persist`; on a fresh session the revealed index starts at 0 but `next_unused_address` recomputes the correct next-unused index from on-chain usage after the first sync. No stale address is shown post-sync.
- **R1.4 not included:** connect-time derivation-path picking is still present (canonical-paths-only cleanup is R1.4).
- **Phase 6 not included:** no QR for the receive address; Admin ID display/copy still pending.

## Links

- Implementation plan: [`admin-wallet-implementation-plan.md`](../specs/admin-wallet-implementation-plan.md) (R1.3 ✅; next: R1.4)
- Spec: [`admin-wallet-receive-rotation.md`](../specs/admin-wallet-receive-rotation.md)
- R1.2 predecessor: [`2026-06-02-admin-wallet-clean-wallet-ui.md`](2026-06-02-admin-wallet-clean-wallet-ui.md)
