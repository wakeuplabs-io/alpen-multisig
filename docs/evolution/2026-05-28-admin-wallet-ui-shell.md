# Evolution Archive: admin-wallet-ui-shell

**Date**: 2026-05-28
**Feature branch**: feature/admin-wallet-ui-shell
**Spec**: [docs/specs/admin-wallet-ui-shell.md](../specs/admin-wallet-ui-shell.md)
**Program**: Admin Wallet — Phase 3

---

## Feature Summary

Phase 3 of the Admin Wallet program delivered a React/TypeScript slide-over WalletPanel for the Alpen desktop app. The panel exposes read-only admin wallet state — balance, receive address, UTXO list, transaction history, and a send placeholder — in a focused side panel rather than occupying the main screen layout.

The implementation ports the Alta WalletPanel design pattern into the existing Tauri/React shell with: URL-based open/section state (no additional state manager), a 240 ms CSS transition, a focus trap for accessibility, and six section components. Phase 2 hooks (useAdminWalletBalance, useAdminWalletUtxos, useAdminWalletAddresses, useAdminWalletSync) were consumed without modification. No backend changes were required; this phase is entirely frontend.

---

## Steps Completed

14 steps across 4 phases.

### Phase 01 — Model Layer
1. WalletPanelSection enum and WalletPanelState type
2. URL serialisation / deserialisation helpers
3. Transition constants (ANIMATION_DURATION_MS = 240)
4. Model unit tests (8 tests)

### Phase 02 — Hooks Layer
5. usePanelState hook (URL-driven open/close/section)
6. useFocusTrap hook (focus lock inside panel)
7. Hook contract tests (2 tests)

### Phase 03 — UI Components
8. SyncChip component
9. BalanceSection component
10. ReceiveSection component (with promoted CopyButton)
11. AddressesSection component
12. TransactionHistorySection component
13. SendSection placeholder

### Phase 04 — Screen Integration
14. WalletPanel assembly, WalletPanelContent extraction, integration into AdminDashboardScreen and BroadcastScreen

---

## Key Decisions

**URL-based panel state**: open/section stored as search params (`?wallet=open&section=balance`) so panel state survives navigation and is deep-linkable without adding a context or Zustand slice.

**WalletPanelContent extraction**: during L6 refactoring, the inner content was extracted to `WalletPanelContent` to avoid duplicating the section-rendering logic between `AdminDashboardScreen` and `BroadcastScreen`.

**CopyButton and SectionLabel promotion**: both were originally scoped to `broadcast-details-card.tsx`. Promoted to `src/components/` as they are now shared across the wallet panel and broadcast views.

**No backend changes**: Phase 3 is a pure UI shell. The data contract was already established by Phase 2 hooks; no new IPC commands or backend endpoints were introduced.

**Focus trap over dialog**: the panel uses a custom `useFocusTrap` hook rather than a `<dialog>` element to preserve animated entry/exit behaviour while remaining accessible.

---

## Files Modified

### Production (45 files created or modified)

**New model files**
- `desktop-app/src/wallet/panel/model.ts`
- `desktop-app/src/wallet/panel/url.ts`
- `desktop-app/src/wallet/panel/constants.ts`

**New hook files**
- `desktop-app/src/wallet/panel/use-panel-state.ts`
- `desktop-app/src/wallet/panel/use-focus-trap.ts`

**New component files**
- `desktop-app/src/wallet/panel/components/sync-chip.tsx`
- `desktop-app/src/wallet/panel/components/balance-section.tsx`
- `desktop-app/src/wallet/panel/components/receive-section.tsx`
- `desktop-app/src/wallet/panel/components/addresses-section.tsx`
- `desktop-app/src/wallet/panel/components/transaction-history-section.tsx`
- `desktop-app/src/wallet/panel/components/send-section.tsx`
- `desktop-app/src/wallet/panel/components/wallet-panel-content.tsx`
- `desktop-app/src/wallet/panel/wallet-panel.tsx`

**Promoted shared components**
- `desktop-app/src/components/copy-button.tsx`
- `desktop-app/src/components/section-label.tsx`

**Modified screens**
- `desktop-app/src/screens/admin-dashboard-screen.tsx`
- `desktop-app/src/screens/broadcast-screen.tsx`

### Tests
- `desktop-app/src/wallet/panel/model.test.ts` (8 unit tests)
- `desktop-app/src/wallet/panel/use-panel-state.test.ts` (2 contract tests)
- `desktop-app/src/wallet/panel/architecture.test.ts` (1 architecture compliance test)

### Docs
- `docs/specs/admin-wallet-ui-shell.md`
- `docs/archive/features/admin-wallet-ui-shell/feature-delta.md`
- `docs/evolution/2026-05-28-admin-wallet-ui-shell.md` (this file)

---

## Post-Integration Bug Fixes

Three IPC serialisation bugs were discovered and resolved after the initial delivery:

### Bug 1 — DTO camelCase mismatch (0-balance / "Never synced")
`BalanceDto`, `UtxoDto`, `AddressDto`, and `SyncStatusDto` in `wallet_service.rs` lacked
`#[serde(rename_all = "camelCase")]`. Rust serialised `confirmed_sats` / `is_used` /
`last_synced_at` while the TS frontend expected `confirmedSats` / `isUsed` / `lastSyncedAt`.
All fields arrived as `undefined` → balance showed 0 BTC, "Never synced" displayed permanently.
**Fix**: added `#[serde(rename_all = "camelCase")]` to all four DTOs.

### Bug 2 — IPC args in snake_case ("Cannot reach Bitcoin node" banner)
`listAdminWalletAddresses` in `admin-wallet.ts` sent `page_index` / `page_size` (snake_case).
Tauri v2 expects camelCase argument keys, so the command rejected the call with
"missing required key pageIndex". `parseAdminWalletError` misclassified the raw string as
`RpcUnreachable`, producing the misleading banner.
**Fix**: changed to `pageIndex` / `pageSize` in the IPC call.

### Bug 3 — Address list showed all 20 regardless of balance
`composeAddressesWithBalance` returned all paginated addresses unconditionally.
**Fix**: added `.filter(row => row.balanceSats > 0)`. Also wired `onRefreshSync` to refresh
balance, addresses, and UTXOs hooks after sync completes so the UI reflects post-sync state.

---

## Quality Gates Passed

| Gate | Result |
|------|--------|
| Model unit tests (12 suites) | PASS |
| Hook contract tests | PASS |
| Architecture compliance test | PASS |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace --lib` (84 tests) | PASS |
| `npm run format:check` | PASS |
| `npm run lint` | PASS |
| `npm run build` | PASS |

Manual smoke (requires `npm run tauri dev` + regtest stack): see spec §8.4.

---

## Lessons Learned

**URL state as panel coordinator**: storing open/section state in the URL eliminated the need for a React context or external state slice while providing deep-link capability. For panels that must survive navigation this pattern is low-overhead and testable without a DOM.

**Extract content early**: the WalletPanelContent extraction was prompted by the need to embed the panel in a second screen. Identifying shared content surfaces at the component-design stage rather than during integration would save one refactoring step.

**Component promotion path**: tracking which helper components are scoped vs shared from the start avoids late-stage file moves. CopyButton and SectionLabel were always logically shared; treating them as such from Phase 03 onwards would have been cleaner.

**Hook tests as contracts**: writing hook tests as behavioural contracts (open → section, close → URL cleared) before implementing the hooks caught parameter-handling issues early and doubled as documentation of expected URL shape.
