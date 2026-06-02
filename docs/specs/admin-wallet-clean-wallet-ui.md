# Spec: Admin Wallet — Clean Wallet UI (R1.2)

> Release 1, step R1.2 of [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md).
> Builds directly on the Phase 3 UI shell ([`admin-wallet-ui-shell.md`](./admin-wallet-ui-shell.md)) — same slide-over panel, no new data sources.

## Objective

Bring the Admin Wallet slide-over panel to production quality. The Phase 3 shell intentionally shipped with roadmap-leaking placeholders ("arrives in Phase 5/6", "Send is not available yet", a dev-only "Admin tools" grouping, and "dev mnemonic" wording in error copy). R1.2 removes those dev-only affordances and placeholders so the panel cleanly shows **balance, addresses, and receive** with consistent loading / empty / error states, at visual parity with the Alta `WalletPanel`.

**Why:** The panel is the signer's first view of their wallet. Future-phase placeholder copy and dev groupings make a shipped product look unfinished and leak internal roadmap phases to end users. R1.2 is purely presentational cleanup — no new behavior, no data-source changes, no protocol changes.

**Visual source of truth:** `miniwallet/Alpen-v0.1-Alta-handoff/Alpen v0.1 - Alta/WalletPanel/*.jsx`, `data.js`.

## Scope

### In scope (R1.2)

Frontend-only, confined to `desktop-app/src/domain/admin-wallet/` (plus the shared error-copy module).

1. **Remove future-phase placeholder sections from the panel body** (`wallet-panel-content.tsx`):
   - `SendPlaceholder` — disabled "Send" button + "Send is not available yet." (Send is Phase 4).
   - `TxHistoryList` — "Transaction history arrives in Phase 5 (transactions + fee-bump)." (Phase 5).
   - `ReceiveSection` — QR expander showing "QR preview unavailable." + "QR rendering arrives in Phase 6 (receive rotation)." (QR is Phase 6).
   - Delete the now-unreferenced components (`send-placeholder.tsx`, `tx-history-list.tsx`, `receive-section.tsx`, and `tx-history-item.tsx` if unreferenced) and their imports. Phase 4/5/6 will re-introduce real components when the features land.
2. **Remove the dev-only "Admin tools" grouping** (the `Admin tools` uppercase label + its border block) and promote the addresses-with-balance list to a normal, top-level section of the panel.
3. **Clean the kept sections** so the panel renders exactly: header → balance → receive address → addresses-with-balance → sync footer.
4. **Consistent loading / empty / error states** across the kept sections:
   - Balance: loading skeleton (exists); add a graceful non-throwing render for missing data.
   - Receive address: loading skeleton (exists); explicit empty state when there is no unused address (currently renders `null` — replace with a quiet "No receive address yet" line).
   - Addresses-with-balance: loading skeleton, typed-error line, and an explicit empty state ("No addresses with balance yet") when expanded with zero rows (currently renders an empty table).
   - Sync chip: keep relative-time + typed error (no change beyond copy review).
5. **Copy cleanup — remove dev/roadmap wording** in user-facing strings:
   - `format-admin-wallet-error.ts`: drop "(dev mnemonic)" / "Log in with Palabras (dev mnemonic)" phrasing; keep accurate, neutral guidance.
   - `disabled-wallet-card.tsx`: keep the connect-a-wallet guidance but neutral wording (no "dev mnemonic").
   - Remove every "arrives in Phase N" string from the panel.
6. **Visual parity pass** with the Alta `WalletPanel` for the kept sections (spacing, typography, balance toggle, copy button), within the existing Tailwind styling already in place.

### Out of scope (NOT included)

- **R1.3 — Receive rotation** (one-time-use address rotation, fresh-address issuance). R1.2 shows the current first-unused address only.
- **QR rendering** (Phase 6) — no QR library, no QR component.
- **Transactions list / fee-bump** (Phase 5).
- **Send pipeline / PSBT / Send form** (Phase 4).
- **Admin ID display/copy + QR** (Phase 6).
- **Any backend / Rust / IPC / DTO / env-var change** — none. R1.2 consumes the existing Phase 2 hooks verbatim.
- **Protocol changes** (SPS-50/51/65, commit/reveal) — none.
- **New runtime dependencies** — none.
- **New routes / navigation changes** — none; the trigger button and slide-over wiring from Phase 3 are unchanged.

## Technical Design

### Affected files

```
desktop-app/src/domain/admin-wallet/
├── components/
│   ├── wallet-panel-content.tsx          (MODIFY — remove placeholders + "Admin tools" block; reorder sections)
│   ├── receive-address-row.tsx           (MODIFY — explicit empty state instead of null)
│   ├── addresses-with-balance-list.tsx   (MODIFY — explicit empty state when expanded + zero rows)
│   ├── wallet-balance.tsx                (MODIFY — graceful render review; no behavior change expected)
│   ├── send-placeholder.tsx              (DELETE)
│   ├── tx-history-list.tsx               (DELETE)
│   ├── tx-history-item.tsx               (DELETE if unreferenced)
│   └── receive-section.tsx               (DELETE)
└── model/
    ├── format-admin-wallet-error.ts      (MODIFY — neutral copy, drop "dev mnemonic")
    └── (disabled-wallet-card.tsx)         (MODIFY — neutral copy)
```

No new files are required. **Decision:** empty-state lines are inlined per section (no shared primitive). **Decision:** the Send CTA is removed entirely for R1.2 (re-introduced in Phase 4), not kept as a disabled button.

### Panel body after cleanup (`wallet-panel-content.tsx`)

```
WalletPanelContent
├── DisabledWalletCard            (when Disabled | RegtestGuardViolation — unchanged)
└── (enabled)
    ├── WalletBalance             balance, BTC↔sats toggle, loading skeleton
    ├── ReceiveAddressRow         first-unused external address + copy | empty line | skeleton
    ├── AddressesWithBalanceList  collapsible; loading | error | empty | table
    └── SyncChip                  relative-time | typed error + Refresh
```

The `WalletPanelContent` props shrink: `receiveAddress`-for-`ReceiveSection`, and any props that fed the removed placeholders, are deleted. Props for the kept sections are unchanged.

### Component contracts (changed only)

```ts
// receive-address-row.tsx — empty state instead of returning null
type ReceiveAddressRowProps = {
  address: string        // '' → render explicit empty line, not null
  isLoading?: boolean
}
```

```ts
// addresses-with-balance-list.tsx — explicit empty state
// when isExpanded && rows !== null && rows.length === 0 && !isLoading && error === null
// render: "No addresses with balance yet" (quiet, muted), not an empty <table>.
```

### Production code vs. test helpers

- **Production:** the modified components and `format-admin-wallet-error.ts`. All are presentational/pure; no IPC, no business rules.
- **Test helpers:** existing builders under `model/__fixtures__/` (`make-utxo`, `make-address`, `make-sync-status`) — reused as-is, never imported by production (enforced by `architecture.test.ts`).

This is a deletion-and-cleanup change: no new production functions are introduced (beyond, optionally, one `wallet-empty-line` presentational component).

## Test Cases

Follow the existing `tsx` + `node:assert/strict` pattern; React-rendering tests remain blocked by the missing `vitest + @testing-library/react` dev-dependency (documented in Phase 3) — so coverage is via pure-function / contract tests plus the architecture grep test and manual smoke.

### Pure / contract

| Test | Cases |
|---|---|
| `format-admin-wallet-error.test.ts` (UPDATE) | Assert the new neutral copy per `AdminWalletError` variant; assert no string contains `dev mnemonic`; severities unchanged. |
| `architecture.test.ts` (UPDATE) | Assert no panel source imports the deleted components (`send-placeholder`, `tx-history-list`, `receive-section`); existing import-direction rules still hold (components don't import `@tauri-apps/api/core`; model doesn't import `react`; no production import of `__fixtures__/`). |
| roadmap-copy guard (NEW, in `architecture.test.ts`) | Grep `domain/admin-wallet/components/**` for `/arrives in Phase|not available yet|QR preview unavailable/` → expect zero matches. |

### Manual / smoke (PR description)

1. `cd desktop-app && npm run tauri dev`, regtest stack up, mnemonic login. Open `[Wallet]` → panel shows: balance, receive address (+copy), addresses-with-balance (expand → table), sync chip. **No** Send button, **no** transactions placeholder, **no** QR expander, **no** "Admin tools" label, **no** "arrives in Phase N" text.
2. Toggle BTC ↔ sats — formatting matches Alta.
3. Expand addresses with zero balances → "No addresses with balance yet" (not an empty table).
4. Empty receive (no unused address) → quiet empty line (not a blank gap).
5. Stop bitcoind → sync chip shows typed error; data persists.
6. Disabled environment → single Disabled card with neutral copy (no "dev mnemonic").
7. Repeat on `/proposals/:actionId/broadcast` — identical.

### Regression

- `npm run lint`, `npm run format:check`, `npm run build` green.
- Existing wallet tests (`test:hooks`, `test:wallet-binding`, model tests) pass unchanged.
- `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check` — unchanged green (no Rust changes).

## Module structure

| File | Single responsibility (after R1.2) |
|---|---|
| `components/wallet-panel-content.tsx` | Compose the enabled panel body: balance, receive address, addresses-with-balance, sync — and the Disabled card branch. |
| `components/receive-address-row.tsx` | Render the first-unused receive address with copy, plus loading and empty states. |
| `components/addresses-with-balance-list.tsx` | Render the collapsible addresses table with loading, error, and empty states. |
| `components/wallet-balance.tsx` | Render total balance with BTC↔sats toggle and loading skeleton. |
| `model/format-admin-wallet-error.ts` | Map `AdminWalletError` variants to neutral, dev-free `{ title, body, severity }`. |

Dependency direction unchanged: components depend on model types; model depends on `@/api/admin-wallet` DTOs only; no inverted imports (enforced by `architecture.test.ts`).

## Done when

- Panel shows **balance, addresses, and receive** cleanly, with consistent loading/empty/error states.
- **Zero** dev-only controls or roadmap placeholders in the panel (no Send placeholder, no Phase-N copy, no QR placeholder, no "Admin tools" grouping, no "dev mnemonic" wording).
- Visual parity with the Alta `WalletPanel` for the kept sections.
- No functional regressions; all frontend and Rust CI checks green.

## Links

- Program plan: [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) (R1.2)
- Predecessor: [`admin-wallet-ui-shell.md`](./admin-wallet-ui-shell.md) (Phase 3)
- Read path: [`admin-wallet-core-read-path.md`](./admin-wallet-core-read-path.md) (Phase 2 hooks, unchanged)
- Frontend rules: `.claude/rules/react-frontend-patterns.md`, `.claude/rules/typescript-standards.md`
- Implementation skill: `.claude/skills/react-ui-screen-implementation/SKILL.md`
- Visual source: `miniwallet/Alpen-v0.1-Alta-handoff/Alpen v0.1 - Alta/WalletPanel/*`
