# Feature Delta: admin-wallet-clean-wallet-ui

**Program**: Admin Wallet — Release 1 (R1.2)
**Branch**: `feature/admin-wallet-clean-wallet-ui`
**Spec**: [docs/specs/admin-wallet-clean-wallet-ui.md](../../specs/admin-wallet-clean-wallet-ui.md)
**Evolution**: [docs/evolution/2026-06-02-admin-wallet-clean-wallet-ui.md](../../evolution/2026-06-02-admin-wallet-clean-wallet-ui.md)

---

## Implementation Summary

R1.2 brought the Admin Wallet slide-over panel to production quality by removing Phase 3 scaffold placeholders (Send, transaction history, QR expander), the dev-only "Admin tools" grouping, and roadmap/dev wording in error surfaces. The enabled panel body is **balance → receive address → addresses-with-balance → sync**. Inline empty states cover missing receive address and zero funded addresses. No backend, IPC, or protocol changes.

---

## Files Modified

### Production

**Deleted**

- `desktop-app/src/domain/admin-wallet/components/send-placeholder.tsx`
- `desktop-app/src/domain/admin-wallet/components/tx-history-list.tsx`
- `desktop-app/src/domain/admin-wallet/components/tx-history-item.tsx`
- `desktop-app/src/domain/admin-wallet/components/receive-section.tsx`

**Modified**

- `desktop-app/src/domain/admin-wallet/components/wallet-panel-content.tsx`
- `desktop-app/src/domain/admin-wallet/components/receive-address-row.tsx`
- `desktop-app/src/domain/admin-wallet/components/addresses-with-balance-list.tsx`
- `desktop-app/src/domain/admin-wallet/components/disabled-wallet-card.tsx`
- `desktop-app/src/domain/admin-wallet/model/format-admin-wallet-error.ts`

### Tests

- `desktop-app/src/domain/admin-wallet/model/__tests__/format-admin-wallet-error.test.ts`
- `desktop-app/src/domain/admin-wallet/architecture.test.ts` (Rule 4)

### Documentation

- `docs/specs/admin-wallet-clean-wallet-ui.md` (new)
- `docs/specs/admin-wallet-implementation-plan.md`
- `docs/specs/admin-wallet-ui-shell.md` (supersession note)

---

## Verification

- `npm run test:architecture` (Rule 4)
- `npm run test:model-format-admin-wallet-error`
- `npm run format:check`, `npm run lint`, `npm run build`
- Manual smoke: wallet panel on `/proposals` and broadcast — no placeholders, neutral disabled copy

---

## Deferred (unchanged scope)

| Item | Phase |
|------|-------|
| Receive rotation after credit | R1.3 |
| Send happy path | Phase 4 |
| Transaction list + RBF | Phase 5 |
| QR + Admin ID UI | Phase 6 |
