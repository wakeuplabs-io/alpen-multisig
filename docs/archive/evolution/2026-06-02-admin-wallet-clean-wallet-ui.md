# Evolution: Admin Wallet — Clean Wallet UI (R1.2)

**Date:** 2026-06-02
**Branch:** `feature/admin-wallet-clean-wallet-ui`
**Commit:** `138412d`
**Spec:** [`docs/specs/admin-wallet-clean-wallet-ui.md`](../specs/admin-wallet-clean-wallet-ui.md)
**Predecessor:** Phase 3 UI shell ([`2026-05-28-admin-wallet-ui-shell.md`](2026-05-28-admin-wallet-ui-shell.md))

## Summary

R1.2 removed dev-only affordances and roadmap-leaking placeholders from the Admin Wallet slide-over panel. The panel now shows **balance → receive address → addresses-with-balance → sync** only. Placeholder components for Send, transaction history, and QR were deleted. User-facing error and disabled copy was neutralized (no "dev mnemonic" / "Palabras" / "arrives in Phase N"). Inline empty states were added for receive address and the addresses list. An architecture test Rule 4 guards against roadmap placeholder copy regressing into panel components.

## Business Context

Phase 3 intentionally shipped a scaffolded WalletPanel with future-phase placeholders so later phases could wire data without restructuring layout. By R1.1 (broadcast signing) the panel was user-visible on dashboard and broadcast screens; roadmap copy and an "Admin tools" dev grouping made the product look unfinished. R1.2 is presentational cleanup only — no IPC, protocol, or wallet-semantics changes.

## Deliverable

Single frontend-only increment (no TDD roadmap steps — spec + implementation in one PR).

| Item | Status |
|------|--------|
| Spec `admin-wallet-clean-wallet-ui.md` | Done |
| Remove Send / TxHistory / QR placeholder components | Done |
| Drop "Admin tools" grouping; reorder panel sections | Done |
| Inline empty states (receive + addresses) | Done |
| Neutralize `format-admin-wallet-error` + `disabled-wallet-card` copy | Done |
| Tests: error-copy guard + architecture Rule 4 | Done |
| Frontend CI: format, lint, build, wallet tests | Green |

## Key Decisions

- **Send CTA removed entirely** for R1.2 (not kept disabled). Phase 4 re-introduces Send with real behavior.
- **Empty states inlined** per section — no shared `WalletEmptyLine` primitive.
- **Transactions and QR deferred** — components deleted; Phase 5/6 add real sections when features land.
- **Rule 4 architecture guard** — grep panel components for roadmap/dev placeholder patterns; fails CI if copy leaks back.

## Files Changed

**Deleted (production):**

- `desktop-app/src/domain/admin-wallet/components/send-placeholder.tsx`
- `desktop-app/src/domain/admin-wallet/components/tx-history-list.tsx`
- `desktop-app/src/domain/admin-wallet/components/tx-history-item.tsx`
- `desktop-app/src/domain/admin-wallet/components/receive-section.tsx`

**Modified (production):**

- `desktop-app/src/domain/admin-wallet/components/wallet-panel-content.tsx` — panel body layout
- `desktop-app/src/domain/admin-wallet/components/receive-address-row.tsx` — empty state
- `desktop-app/src/domain/admin-wallet/components/addresses-with-balance-list.tsx` — empty state
- `desktop-app/src/domain/admin-wallet/components/disabled-wallet-card.tsx` — neutral copy
- `desktop-app/src/domain/admin-wallet/model/format-admin-wallet-error.ts` — neutral copy

**Modified (tests):**

- `desktop-app/src/domain/admin-wallet/model/__tests__/format-admin-wallet-error.test.ts` — no dev wording
- `desktop-app/src/domain/admin-wallet/architecture.test.ts` — Rule 4 roadmap-copy guard

**Documentation:**

- `docs/specs/admin-wallet-clean-wallet-ui.md` (new)
- `docs/specs/admin-wallet-implementation-plan.md` (R1.2 marked complete)
- `docs/specs/admin-wallet-ui-shell.md` (R1.2 supersession note)

## Known Limitations (post-R1.2)

- **R1.3 not included:** receive address does not rotate after incoming funds confirm.
- **Phase 4 not included:** no Send form or broadcast from the wallet panel.
- **Phase 5/6 not included:** no transaction list or QR on Receive.

## Links

- Implementation plan: [`admin-wallet-implementation-plan.md`](../specs/admin-wallet-implementation-plan.md) (R1.2 ✅; next: R1.3)
- Phase 3 shell spec: [`admin-wallet-ui-shell.md`](../specs/admin-wallet-ui-shell.md)
