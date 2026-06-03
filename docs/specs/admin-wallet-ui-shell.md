# Spec: Admin Wallet UI Shell (Phase 3)

> **⚠️ Panel placeholders superseded in R1.2** ([`admin-wallet-clean-wallet-ui.md`](./admin-wallet-clean-wallet-ui.md)).
> Send, transaction-history, and QR placeholder sections described below were removed; the production panel shows balance, receive address, addresses-with-balance, and sync only. Error/disabled copy no longer references "dev mnemonic" or roadmap phases.
>
> **⚠️ Guard condition updated in Phase 3.6** ([`admin-wallet-commit-funding-only.md`](./admin-wallet-commit-funding-only.md)).
> The Disabled-card copy and dev recipes below referencing `COMMIT_FUNDING=admin_wallet` are obsolete:
> the `COMMIT_FUNDING` env var was removed. The Admin Wallet is enabled by
> `BITCOIN_NETWORK=regtest` + `ALLOW_DEV_MNEMONIC_SIGNING=1` only. The panel UI design is otherwise unchanged.

## Objective

Port the Alta `WalletPanel` into the desktop-app as a **slide-over modal** that any authenticated screen (dashboard, broadcast) can open from a header button. The panel renders read-only Admin Wallet state — balance, addresses with per-address balance, transactions placeholder, receive placeholder, send placeholder — consuming the Phase 2 IPC/hooks (`useAdminWalletBalance`, `useAdminWalletUtxos`, `useAdminWalletAddresses`, `useAdminWalletSync`) as-is.

**Why:** Phase 3 of the Admin Wallet program (see [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) §4 Phase 3) is the visual surface where the signer first sees their wallet. It is **read-only by design**: Send goes in Phase 4, QR + receive rotation in Phase 6, HW signing in Phase 7. By shipping the slide-over now with all sections present (including placeholders for the deferred phases) we lock the visual contract early and reduce churn in Phases 4–8.

**Visual source of truth:** `miniwallet/Alpen-v0.1-Alta-handoff/Alpen v0.1 - Alta/WalletPanel/*.jsx` and `data.js`. The repo has no `branding/` folder — Alta is canonical.

**Related specs:** [Phase 1 — commit funding](./admin-wallet-regtest-commit-funding.md) · [Phase 2 — core read path](./admin-wallet-core-read-path.md) · [Broadcast protocol (unchanged)](./proposal-broadcast-commit-reveal.md).

## Scope

### In scope (Phase 3)

- **Slide-over modal** `<WalletPanel />` rendered above the current screen with backdrop, focus trap, `Escape` to close, and a 240 ms slide animation (ported from Alta `WalletPanel.jsx`).
- **Entry point:** a "Wallet" button added to the `ScreenShell` `headerContent` slot of `proposals-dashboard-screen.tsx` and `broadcast-proposal-screen.tsx` (and only those screens — wallet-connect, sign and cancel flows do not gain the button in this phase). Click toggles the panel open/closed.
- **URL state:** `?wallet=open` opens the panel; `?walletSection=<section>` selects which collapsible section is expanded. Both are parsed from React Router 6 `useSearchParams`. Refresh restores the panel.
- **Sections inside the panel** (apiladas verticalmente, orden Alta + extensiones Phase 3 / PRD):
  1. `WalletPanelHeader` — truncated Admin Wallet first-unused address (or session timer) + close button.
  2. `WalletBalance` — total BTC ↔ sats toggle. Primary value + secondary value + unit toggle button (Alta-literal).
  3. `ReceiveAddressRow` — current first-unused external address (full text, monospace, copy-to-clipboard with toast).
  4. `AddressesWithBalanceList` — **collapsible** ("All addresses with balance · N"); when expanded shows a table of `{ index, address, balanceSats, isUsed }` derived locally from UTXOs grouped by `derivationIndex`. Default collapsed.
  5. `ReceiveSection` — toggle button "Receive ▾" that expands into an explicit placeholder block: *"QR rendering arrives in Phase 6 (receive rotation)."* with the address copyable below. No QR library added.
  6. `TxHistoryList` (with `TxHistoryItem` ported but rendered in empty state): *"Transaction history arrives in Phase 5 (transactions + fee-bump)."* The components are ported now so Phase 5 only wires data.
  7. `SendPlaceholder` — *"Send arrives in Phase 4 (regtest Send happy path)."* Disabled CTA. No form fields.
  8. `SyncChip` (footer) — `Last sync · 12s ago` + manual `Refresh` button calling `useAdminWalletSync().triggerSync()`. On error from `syncStatus.lastError`, replace the relative time with a typed error string (`AdminWalletError` → friendly copy).
- **Per-section state mapping** to Phase 2 hooks:
  | Section | Phase 2 hook(s) |
  |---|---|
  | `WalletBalance` | `useAdminWalletBalance` |
  | `ReceiveAddressRow` | `useAdminWalletAddresses('External', 0, 20)` (pick first `!isUsed`) |
  | `AddressesWithBalanceList` | `useAdminWalletAddresses(...)` ⊕ `useAdminWalletUtxos()` via derived view-model |
  | `ReceiveSection` | same as `ReceiveAddressRow` |
  | `TxHistoryList` | (none — empty state) |
  | `SendPlaceholder` | (none) |
  | `SyncChip` | `useAdminWalletSync` |
- **Error and loading surfaces** per section: `AdminWalletError` mapped to friendly copy in a shared util (`format-admin-wallet-error.ts`); loading skeleton stripes; empty states explicit. `AdminWalletError.Disabled` collapses the panel body to a single info card "Admin Wallet is not enabled for this environment" so the user gets one coherent state regardless of which hook returned it.
- **Tests** following the existing project pattern (`tsx` scripts under `desktop-app/src/**/*.test.ts(x)`, registered as `npm run test:*` in `package.json`). React-rendering tests remain blocked by the missing `vitest + @testing-library/react` dev-dependency (already documented in `domain/admin-wallet/hooks/__tests__/use-admin-wallet-hooks.test.ts`); this spec does **not** introduce a new test framework.

### Out of scope (deferred to later phases)

- Send pipeline (build/sign/broadcast PSBT, fee form, validation) — Phase 4.
- RBF / fee-bump / unconfirmed transactions list with real data — Phase 5.
- QR rendering and one-time-use receive rotation policy — Phase 6.
- Admin ID (`m/84'/0'/73'/0/0`) display and "view on device" CTA — Phase 6.
- Hardware wallet PSBT preview — Phase 7.
- Shared Send + governance broadcast UX refactor — Phase 8.
- Remote testnet/mainnet RPC, network presets, capability flags — Phase 9.
- New backend env vars, new Tauri commands, new DTOs, new error variants — none in this phase.
- New runtime dependencies (no `qrcode.react`, no `react-query`, no test framework).
- Adding the Wallet button to `wallet-connect-screen.tsx`, `sign-screen.tsx`, `create-proposal-screen.tsx`, `cancel-proposal-*` screens (out of scope for this phase — only dashboard and broadcast).
- Backend Rust changes — Phase 3 is **frontend-only**.

## Requirements Alignment

- **Authorities:** Strata Administrator and Alpen Administrator only (current program scope). The panel renders identically for both; authority context comes from `useSession().selectedRole` and is not duplicated inside the panel.
- **Two-key model unchanged:** the panel reads from the **Admin Wallet** (`m/86'/0'/73'/n/n`, P2TR). It never references or signs with the Admin ID (`m/84'/0'/73'/0/0`).
- **Signer safety carry-over:** all sections are side-effect-free. The only write paths are `useAdminWalletSync().triggerSync()` (chain RPC sync) and `navigator.clipboard.writeText()` (copy to clipboard). No keys, mnemonics, or PSBTs are touched by Phase 3.
- **PRD §4.3 coverage (Phase 3 scope):**
  | PRD subsection | Phase 3 coverage | Coverage by |
  |---|---|---|
  | §4.3.1 Balance (net of unconfirmed) | ✅ R1.5 | `WalletBalance` hero = `confirmedSats`; tertiary `unconfirmedSats` line; mempool in `do_sync` |
  | §4.3.2 Addresses with balance | ✓ | `AddressesWithBalanceList` (derived from UTXOs) |
  | §4.3.3 Transactions + fee-bump | placeholder | `TxHistoryList` empty state → Phase 5 |
  | §4.3.4 Receive (address + QR + rotation) | partial (address + copy) | `ReceiveSection` placeholder → Phase 6 |
  | §4.3.5 Send | placeholder | `SendPlaceholder` → Phase 4 |
- **No protocol changes.** Phase 3 makes zero modifications to SPS-50/51/65 or to commit/reveal.

## State Model

Phase 3 introduces **no in-process or persistent state**. All wallet data is fetched via Phase 2 hooks (transient React state). The only new persisted state is in the URL query string:

| URL param | Values | Default | Effect |
|---|---|---|---|
| `wallet` | `open` \| absent | absent | Panel visibility |
| `walletSection` | `addresses` \| `receive` \| `transactions` \| `send` \| absent | absent | Which collapsible section is expanded by default when the panel opens |

The panel itself does **not** add this state to React Context; each screen consumes `useWalletPanelState()` which reads/writes `URLSearchParams` via React Router 6.

## Product Flow

### Opening the panel

1. The signer is on `/proposals` (dashboard) or `/proposals/:actionId/broadcast` (broadcast screen).
2. The header shows a compact button `[Wallet]` (icon + label) inside `ScreenShell.headerContent`, placed left of the Session chip.
3. Click → `setSearchParams({ wallet: 'open' })` → panel slides in from the right over 240 ms; backdrop fades in; focus moves to the first focusable element inside the panel; `Tab` / `Shift+Tab` cycle inside the panel; `Escape` closes.
4. Underlying screen remains visible (Alta semantics) — backdrop click also closes.

### While open

- `WalletBalance` shows total BTC; tap "Show sats" to toggle to sats. Secondary line shows the inverse unit (Alta-literal).
- `ReceiveAddressRow` shows the first-unused external address; copy button copies + briefly toggles to a check icon (Alta-literal).
- `AddressesWithBalanceList` is collapsed by default; expand to see the derived per-address balance table.
- `ReceiveSection` toggle expands into a placeholder + copyable address.
- `TxHistoryList` shows empty state with explicit Phase 5 copy.
- `SendPlaceholder` shows explicit Phase 4 copy.
- `SyncChip` ticks every 15 s (re-render only, not re-fetch); `Refresh` triggers `triggerSync()`.

### Closing the panel

`Escape`, close button, or backdrop click → `setSearchParams({ wallet: undefined, walletSection: undefined })` → reverse animation 240 ms → focus restored to the trigger button.

### Error states

| Trigger | Surface |
|---|---|
| `AdminWalletError.Disabled` from any hook | Panel body collapses to a single info card: *"Admin Wallet is not available. Log in with Palabras (dev mnemonic) to bind the wallet to your session."* |
| `AdminWalletError.RpcUnreachable` | Sync chip shows `Sync error: cannot reach Bitcoin node`. Sections render last known data with a `(stale)` indicator. |
| `AdminWalletError.RpcAuthFailed` | Sync chip shows `Sync error: RPC auth failed`. Same stale-data fallback. |
| `AdminWalletError.DescriptorParseError` | Panel body shows fatal card: *"Admin Wallet descriptor invalid — check the mnemonic used at login (Palabras)."* No partial render. |
| `AdminWalletError.SyncIncomplete` | Sync chip shows the typed `message`; data renders normally. |
| `AdminWalletError.RegtestGuardViolation` | Same body collapse as `Disabled` with the typed `message`. |

## Technical Design

### Module layout

```
desktop-app/src/
├── App.tsx                                            (UNCHANGED — no new routes)
├── screens/
│   ├── proposals-dashboard-screen.tsx                 (MODIFIED — add Wallet header button + <WalletPanel />)
│   └── broadcast-proposal-screen.tsx                  (MODIFIED — same)
└── domain/admin-wallet/
    ├── components/                                    (NEW)
    │   ├── wallet-panel.tsx                            # slide-over container, focus trap, ESC, backdrop
    │   ├── wallet-panel-header.tsx                     # title + close
    │   ├── wallet-panel-trigger.tsx                    # the "Wallet" header button
    │   ├── wallet-balance.tsx                          # BTC ↔ sats toggle
    │   ├── receive-address-row.tsx                     # first-unused address + copy
    │   ├── addresses-with-balance-list.tsx             # collapsible table
    │   ├── address-row.tsx                             # single row (re-used by list)
    │   ├── receive-section.tsx                         # placeholder + copy
    │   ├── tx-history-list.tsx                         # empty-state container
    │   ├── tx-history-item.tsx                         # row (ported but not rendered with data)
    │   ├── send-placeholder.tsx                        # disabled CTA + copy
    │   ├── sync-chip.tsx                               # Last-sync label + Refresh
    │   └── disabled-wallet-card.tsx                    # body collapse for Disabled / RegtestGuardViolation
    ├── hooks/                                         (existing Phase 2 hooks unchanged)
    │   ├── use-wallet-panel-state.ts                   # NEW — URL search-params for open + section
    │   └── use-addresses-with-balance.ts               # NEW — derived view-model combining addresses + UTXOs
    └── model/                                         (NEW)
        ├── format-btc-from-sats.ts                     # Alta-literal magnitude-based decimals
        ├── format-signed-sats.ts                       # signed sats with unicode minus
        ├── trunc-address.ts                            # `bc1p…edrcr` truncation
        ├── trunc-txid.ts                               # `a7f3c9…ab4c32` truncation
        ├── relative-time.ts                            # "12s ago" / "5m ago" / "2h ago"
        ├── group-utxos-by-derivation.ts                # pure aggregation, returns Map<index, sats>
        ├── compose-addresses-with-balance.ts           # AddressDto[] ⊕ UtxoDto[] → AddressWithBalanceView[]
        ├── format-admin-wallet-error.ts                # AdminWalletError → { title, body, severity }
        └── view-models.ts                              # re-exports + the AddressWithBalanceView type
```

### Frontend hooks (additions only)

```ts
// use-wallet-panel-state.ts
export type WalletPanelSection = 'addresses' | 'receive' | 'transactions' | 'send'

export function useWalletPanelState(): {
  isOpen: boolean
  expandedSection: WalletPanelSection | null
  open(section?: WalletPanelSection): void
  close(): void
  setExpandedSection(section: WalletPanelSection | null): void
}
```

```ts
// use-addresses-with-balance.ts
export type AddressWithBalanceView = {
  index: number
  address: string
  balanceSats: number
  isUsed: boolean
}

export function useAddressesWithBalance(opts?: { keychain?: KeychainDto; pageIndex?: number; pageSize?: number }): {
  data: AddressWithBalanceView[] | null
  isLoading: boolean
  error: AdminWalletError | null
  refresh(): void
}
```

`useAddressesWithBalance` composes the two existing hooks and applies `composeAddressesWithBalance` to produce the view-model. **No new IPC.**

### Model module — pure functions

All under `domain/admin-wallet/model/`. Pure, side-effect free, fully unit-testable in tsx scripts:

| Function | Signature | Behavior |
|---|---|---|
| `formatBtcFromSats` | `(sats: number) => string` | Alta-literal: `≥100` → 4 decimals, `≥1` → 6 decimals, else → 8 decimals; non-finite → `'—'` |
| `formatSignedSats` | `(n: number) => string` | `'−1,234 sats'` / `'+1,234 sats'`; non-finite → `'—'`; uses `Number.toLocaleString` |
| `truncAddress` | `(addr: string) => string` | `length ≤ 14` passthrough; else `${addr.slice(0,5)}…${addr.slice(-4)}` |
| `truncTxid` | `(txid: string) => string` | `length ≤ 16` passthrough; else `${txid.slice(0,8)}…${txid.slice(-6)}` |
| `relativeTime` | `(iso: string, now: Date) => string` | Mirror `broadcast-details-card.relativeTime` but with explicit `now` for testability |
| `groupUtxosByDerivation` | `(utxos: UtxoDto[]) => Map<number, number>` | Sums `valueSats` per `derivationIndex`; ignores `keychain === 'Internal'` by default (configurable via opts) |
| `composeAddressesWithBalance` | `(addresses: AddressDto[], utxos: UtxoDto[]) => AddressWithBalanceView[]` | Pure join by index; default `balanceSats = 0` when no UTXO matches |
| `formatAdminWalletError` | `(err: AdminWalletError) => { title: string; body: string; severity: 'fatal' \| 'warning' \| 'info' }` | Single mapping table for every error variant; `Disabled` → info-fatal collapse, `RpcUnreachable`/`RpcAuthFailed` → warning, `DescriptorParseError` → fatal, `SyncIncomplete` → warning, `RegtestGuardViolation` → info-fatal |

### Presentational components — contracts

All components receive prepared props and emit intents through callbacks. **No `invoke()` imports, no business rules.**

```ts
// wallet-panel.tsx
type WalletPanelProps = {
  isOpen: boolean
  onClose(): void
  panelId?: string
  children: ReactNode
}
```

```ts
// wallet-panel-trigger.tsx (the header button)
type WalletPanelTriggerProps = {
  isOpen: boolean
  onToggle(): void
}
```

```ts
// wallet-balance.tsx
type WalletBalanceProps = {
  balanceSats: number
  isLoading: boolean
}
```

```ts
// addresses-with-balance-list.tsx
type AddressesWithBalanceListProps = {
  rows: AddressWithBalanceView[] | null
  isLoading: boolean
  error: AdminWalletError | null
  isExpanded: boolean
  onToggle(): void
}
```

```ts
// sync-chip.tsx
type SyncChipProps = {
  syncStatus: SyncStatusDto | null
  isRefreshing: boolean
  error: AdminWalletError | null
  onRefresh(): void
  now?: Date  // injected for tests; defaults to new Date()
}
```

### Slide-over implementation notes (port from Alta `WalletPanel.jsx`)

- Backdrop + panel use Tailwind `transition` + `transform` classes. `entered` state toggled with `requestAnimationFrame` double-RAF (Alta-literal) to ensure the initial transform applies before the transition.
- Focus trap: collect focusable elements via `FOCUSABLE = 'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])'`; cycle on `Tab`/`Shift+Tab`; restore focus on close.
- `Escape` keydown listener on `document` while open.
- `WALLET_SLIDE_TRANSITION_MS = 240` constant in the component file.
- `panelId = 'wallet-slide-dialog'` default.
- ARIA: `role="dialog"`, `aria-modal="true"`, `aria-labelledby="wallet-panel-title"`.

### Integration in dashboard and broadcast screens

```tsx
// proposals-dashboard-screen.tsx (sketch — actual JSX preserves existing structure)
const { isOpen, expandedSection, open, close, setExpandedSection } = useWalletPanelState()
// ... existing logic
return (
  <ScreenShell
    headerContent={
      <>
        <WalletPanelTrigger isOpen={isOpen} onToggle={() => (isOpen ? close() : open())} />
        {/* existing authority chip + session chip + disconnect */}
      </>
    }
  >
    {/* existing dashboard body */}
    <WalletPanel isOpen={isOpen} onClose={close} panelId="wallet-slide-dialog">
      {/* sections — each reads its own Phase 2 hook */}
    </WalletPanel>
  </ScreenShell>
)
```

Identical wiring in `broadcast-proposal-screen.tsx`. The panel children are the same in both screens — a future refactor (out of scope) may extract a `<WalletPanelBody />` if duplication grows beyond two call sites.

### Reuse / promotion of existing primitives

`broadcast-details-card.tsx` already contains:

- `CopyButton` (lines 23–43)
- `SectionLabel` (lines 45–47)
- `relativeTime` (lines 49–58)
- `LastSyncLabel` (lines 60–69)

These are **promoted** to `desktop-app/src/components/` as part of Phase 3:

- `components/copy-button.tsx`
- `components/section-label.tsx`

(`relativeTime` becomes a pure function in `domain/admin-wallet/model/relative-time.ts`; `LastSyncLabel` becomes `sync-chip.tsx` inside admin-wallet.)

`broadcast-details-card.tsx` is updated to import from the new locations; its public props and behavior do not change.

### IPC / backend

**No changes.** Phase 3 uses Phase 2 IPC verbatim:

- `admin_wallet_get_balance`
- `admin_wallet_list_utxos`
- `admin_wallet_list_addresses`
- `admin_wallet_sync`
- `admin_wallet_sync_status`

No new Tauri commands. No new DTOs. No new env vars. No new Rust dependencies.

### Production code vs. test helpers

- **Production:** every file under `domain/admin-wallet/{components,hooks,model}/*` (excluding fixtures) and the two promoted primitives in `components/`.
- **Test helpers:** typed builders for tests live under `domain/admin-wallet/model/__fixtures__/` (e.g. `make-utxo`, `make-address`, `make-sync-status`). They are:
  - Never re-exported from `domain/admin-wallet/{hooks,components}/index.ts`.
  - Never imported by production files (enforced by a test that greps for `__fixtures__` imports under non-test paths).
  - Never registered as Tauri commands (frontend-only, no IPC involved at all).

## Test Cases

All tests follow the existing `tsx` script pattern (`node:assert/strict`), registered via `npm run test:<name>` in `desktop-app/package.json`.

### `model/` — pure functions (full coverage)

| Test file | Cases |
|---|---|
| `format-btc-from-sats.test.ts` | `0` → `'0.00000000'`; `99_999_999` → `'0.99999999'`; `100_000_000` → `'1.000000'`; `10_000_000_000` → `'100.0000'`; `NaN` → `'—'`; `-1_234` → negative formatted symmetrically |
| `format-signed-sats.test.ts` | `0` → `'+0 sats'`; `1_234` → `'+1,234 sats'`; `-48_250_000` → `'−48,250,000 sats'` (unicode `−`); `NaN` → `'—'`; non-finite → `'—'` |
| `trunc-address.test.ts` | `''` → `''`; `'bc1p'` (≤14) → passthrough; `'bc1pwxlpge5x8n3z7c0hcusmt2jy5fgq9kly6w0s2rflmakrgjtp0w0qxh66u2'` → `'bc1pw…66u2'` |
| `trunc-txid.test.ts` | `''` → `''`; 16-char → passthrough; 64-char → `'a7f3c9e2…6ab4c32'` |
| `relative-time.test.ts` | `now − 30s` → `'30s ago'`; `now − 5m` → `'5m ago'`; `now − 2h` → `'2h ago'`; future → guard returns `'just now'`; invalid ISO → `'—'` |
| `group-utxos-by-derivation.test.ts` | Empty → empty map; single → `{0: 100_000}`; two UTXOs same index → sums; mixed keychains → only external by default; opts `{ includeInternal: true }` → both |
| `compose-addresses-with-balance.test.ts` | Addresses with no UTXOs → all balances `0`; address index `0` with UTXO `1.25 BTC` → `balanceSats: 125_000_000`; address marked `isUsed: false` but has UTXO → balance reflected (UTXO is the truth) |
| `format-admin-wallet-error.test.ts` | One assertion per `AdminWalletError` variant — exact `{ title, body, severity }` shape; severity mapping enforced |

### `hooks/` — contract tests (limited, document blocked-by-dependency)

| Test file | Cases |
|---|---|
| `use-wallet-panel-state.test.ts` | URL-parse → `{ isOpen: true, expandedSection: 'addresses' }` from `?wallet=open&walletSection=addresses`; `open('receive')` → URL becomes `?wallet=open&walletSection=receive`; `close()` → strips both params; unknown `walletSection` value → `expandedSection: null` (graceful); same patterns as `domain/admin-wallet/hooks/__tests__/use-admin-wallet-hooks.test.ts` — pure-TS contract tests only; React rendering blocked by missing `vitest + @testing-library/react`. |
| `use-addresses-with-balance.test.ts` | Pure composition: given mocked outputs of the two underlying hooks (returned synchronously via a thin test-only mock module), the derived view-model equals `composeAddressesWithBalance(addresses, utxos)`. Loading flag: `true` if either underlying hook is loading. Error: the first non-null `AdminWalletError`. |

### Architecture tests (lightweight)

| Test file | Cases |
|---|---|
| `architecture.test.ts` | Grep-based: no file under `domain/admin-wallet/components/` imports from `@tauri-apps/api/core` or `@/api/admin-wallet`; no file under `domain/admin-wallet/model/` imports `react`; no production file under `domain/admin-wallet/` imports from `__fixtures__/`. |

### Manual / smoke (in PR description)

1. `cd desktop-app && npm run tauri dev` with `COMMIT_FUNDING=admin_wallet`, regtest stack up. Open `/proposals`, click `[Wallet]` → panel slides in; verify balance, address row, expandable addresses list with per-address sats, receive placeholder, tx history empty state, send placeholder, sync chip ticks.
2. Toggle BTC ↔ sats; verify formatting matches Alta.
3. Refresh chip → `last_synced_at` advances; force-stop bitcoind → `Sync error: cannot reach Bitcoin node`; data persists with `(stale)` flag.
4. Refresh browser with `?wallet=open&walletSection=addresses` → panel reopens with addresses expanded.
5. Press `Escape`, click backdrop, click close — all close the panel and restore focus to the trigger.
6. With `COMMIT_FUNDING` unset → trigger shows panel; body collapses to Disabled card.
7. Repeat on `/proposals/:actionId/broadcast` — identical behavior.

### Regression

- `npm run lint`, `npm run format:check`, `npm run build` green.
- Existing `npm run test:ipc-schemas`, `npm run test:wallet-binding`, `npm run test:hooks` still pass (no test renaming or framework change).
- `broadcast-details-card.tsx` byte-identical in rendered output (the `CopyButton` and `SectionLabel` extractions are pure refactors; PR description includes a screenshot comparison).
- `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check` — unchanged green (no Rust changes).

## Module structure

For each new file/module, single-responsibility sentence (one line, enforced by review):

| File | Single responsibility |
|---|---|
| `components/wallet-panel.tsx` | Render the slide-over container with backdrop, focus trap, and ESC handling. |
| `components/wallet-panel-header.tsx` | Render the panel's title row and close button. |
| `components/wallet-panel-trigger.tsx` | Render the header button that toggles the panel open/closed. |
| `components/wallet-balance.tsx` | Render total balance with BTC↔sats unit toggle. |
| `components/receive-address-row.tsx` | Render a single address row with copy-to-clipboard. |
| `components/address-row.tsx` | Render one row of `(index, address, balanceSats, isUsed)` for the list. |
| `components/addresses-with-balance-list.tsx` | Render the collapsible table of addresses with per-address balance. |
| `components/receive-section.tsx` | Render the receive placeholder block with copyable first-unused address. |
| `components/tx-history-list.tsx` | Render the transactions section in its empty state with Phase 5 copy. |
| `components/tx-history-item.tsx` | Render one transaction row (ported now, unused with data until Phase 5). |
| `components/send-placeholder.tsx` | Render the send placeholder block with disabled CTA. |
| `components/sync-chip.tsx` | Render the last-sync relative-time label + Refresh button + typed error. |
| `components/disabled-wallet-card.tsx` | Render the body-collapse state for `Disabled` / `RegtestGuardViolation`. |
| `hooks/use-wallet-panel-state.ts` | Read/write URL search params for panel visibility and expanded section. |
| `hooks/use-addresses-with-balance.ts` | Derive `AddressWithBalanceView[]` by composing balance + addresses + UTXOs. |
| `model/format-btc-from-sats.ts` | Format integer sats to BTC string with Alta-literal magnitude-based decimals. |
| `model/format-signed-sats.ts` | Format signed integer sats to a localized string with unicode minus. |
| `model/trunc-address.ts` | Truncate a Bitcoin address to head…tail for display. |
| `model/trunc-txid.ts` | Truncate a Bitcoin txid to head…tail for display. |
| `model/relative-time.ts` | Format an ISO timestamp to "Ns/Nm/Nh ago" with an injectable `now`. |
| `model/group-utxos-by-derivation.ts` | Aggregate UTXO `valueSats` by `derivationIndex` into a `Map<number, number>`. |
| `model/compose-addresses-with-balance.ts` | Join `AddressDto[]` with grouped UTXOs to produce `AddressWithBalanceView[]`. |
| `model/format-admin-wallet-error.ts` | Map `AdminWalletError` variants to `{ title, body, severity }` for UI. |
| `model/view-models.ts` | Re-export view-model types and selected mappers as the model module's public surface. |
| `components/copy-button.tsx` (promoted) | Render a button that copies a string to clipboard and toggles a brief "Copied!" label. |
| `components/section-label.tsx` (promoted) | Render the uppercase section-label typography used across forms and panels. |

Verify dependency direction: components depend on model types; model types depend on `@/api/admin-wallet` DTOs only (boundary types); no model file imports a component or hook; no component file imports `@tauri-apps/api/core` directly. The architecture grep test enforces this.

## Manual fallback

Phase 3 adds no broadcast or signing path. The Phase 1 manual hex export from the broadcast flow remains unchanged. If the wallet panel cannot render (e.g. `DescriptorParseError`), the user can still proceed with proposal signing and the legacy `bitcoind`-funded broadcast — the panel is observational, not on the critical path.

## Open Questions Resolved (this spec)

| # | Question | Resolution |
|---|---|---|
| 1 | Tabs vs slide-over | **Slide-over modal (Alta literal)** — sections apiladas verticalmente; URL state for open + expanded section. The plan's "tabs" wording is reinterpreted as "sections within the panel". |
| 2 | Transactions tab content | Components ported now in empty state; data wiring deferred to Phase 5. |
| 3 | Receive tab without QR | Explicit placeholder + copyable address. QR library introduced in Phase 6. |
| 4 | URL-search-params for state | **Yes** — `?wallet=open&walletSection=<section>`. |
| 5 | Header entry point | **`ScreenShell.headerContent` button** in dashboard and broadcast only; left-rail nav deferred. |
| 6 | PRD §4.3.2 per-address balance gap | **Derive locally** from `useAdminWalletUtxos` aggregated by `derivationIndex`. No Phase 2 changes. |
| 7 | Promote `CopyButton` / `SectionLabel` / `LastSyncLabel` from broadcast-details-card | **Yes** — move to `components/` and `domain/admin-wallet/components/sync-chip.tsx`; `broadcast-details-card.tsx` updates imports. |

## Links

- Program phases: [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md)
- Phase 1 spec: [`admin-wallet-regtest-commit-funding.md`](./admin-wallet-regtest-commit-funding.md)
- Phase 2 spec (precursor): [`admin-wallet-core-read-path.md`](./admin-wallet-core-read-path.md)
- Protocol broadcast (unchanged): [`proposal-broadcast-commit-reveal.md`](./proposal-broadcast-commit-reveal.md)
- PRD references: `docs/0-prd/03-prd-update.md` §4.1–4.3 (Phase 3 covers §4.3.1, §4.3.2; placeholders for §4.3.3, §4.3.4, §4.3.5)
- Architecture overview: `docs/architecture/overview.md` §Component Architecture → Desktop App
- Frontend rules: `.claude/rules/react-frontend-patterns.md`, `.claude/rules/typescript-standards.md`
- Implementation skill: `.claude/skills/react-ui-screen-implementation/SKILL.md`
- Visual source of truth: `miniwallet/Alpen-v0.1-Alta-handoff/Alpen v0.1 - Alta/WalletPanel/*.jsx`, `data.js`, `colors_and_type.css`, `app.css`
