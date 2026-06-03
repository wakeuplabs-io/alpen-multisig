# Spec: Admin Wallet - Balance UX (R1.5)

> **Status:** Complete — branch `feature/admin-wallet-balance-ux`, PR [#211](https://github.com/wakeuplabs-io/alpen-multisig/pull/211).
> Evolution: [`docs/evolution/2026-06-03-admin-wallet-balance-ux.md`](../evolution/2026-06-03-admin-wallet-balance-ux.md).

Implements **Release 1, step R1.5** of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md).
Source of truth: **PRD §4.3.1**.

R1.5 closes the wallet-level balance requirement before R1.6 adds the per-address balance breakdown.

## Objective

The Admin Wallet slide-over must show a signer the confirmed Admin Wallet balance as the primary balance and, when
present, the signed net effect of unconfirmed wallet activity as a separate muted line.

PRD §4.3.1 requires the signer to see:

- the total BTC balance of the wallet net of unconfirmed send and receive transactions; and
- the net balance of all unconfirmed send and receive transactions.

The existing read path already exposes `BalanceDto.confirmedSats`, `BalanceDto.unconfirmedSats`, and `BalanceDto.totalSats`.
The current R1.2 UI only renders `confirmedSats` through a single `WalletBalance.balanceSats` prop, so pending receives or
outgoing spends are invisible in the wallet panel. R1.5 makes that pending effect visible without changing the backend,
IPC commands, BDK sync behavior, or wallet session model.

**Product decision for this slice:** keep the hero balance on `confirmedSats` and render `unconfirmedSats` as the separate
signed line. This preserves the current conservative hero display and satisfies the PRD's separate unconfirmed-net
requirement without folding pending funds into the most prominent amount. The unconfirmed line is the signer's bridge to
the PRD "net of unconfirmed" wording: confirmed hero + signed pending delta = eventual total once pending transactions
confirm or leave the mempool.

**Done when:** with non-zero unconfirmed wallet activity, the panel shows the confirmed hero plus a signed
`+N sats unconfirmed` or `-N sats unconfirmed` line; with zero unconfirmed activity, the line is absent.

## Scope

### Included

- Frontend-only balance presentation in `desktop-app/src/domain/admin-wallet/` and the two existing wallet-panel route
  entry points.
- `WalletBalance` receives both `confirmedSats` and `unconfirmedSats`.
- `WalletPanelContent` passes both balance buckets to `WalletBalance`.
- `proposals-dashboard-screen.tsx` and `broadcast-proposal-screen.tsx` pass `BalanceDto.confirmedSats` and
  `BalanceDto.unconfirmedSats` from `useAdminWalletBalance()`.
- The balance hero keeps the R1.2 Alta-style typography, spacing, loading skeleton, and BTC/sats toggle behavior.
- A muted tertiary line appears only when `unconfirmedSats !== 0`.
- The tertiary line uses PRD-facing language: `unconfirmed`. It must not mention mempool internals, BDK, phase names, or
  roadmap state.
- The signed sats formatting reuses existing model formatting where possible (`formatSignedSats`) instead of adding a
  duplicate formatter.
- Header polish is limited to the wallet title copy if implemented: prefer `Admin Wallet` as primary title and the
  existing session/signing context as subtitle. No capability badge is included in this slice.

### Not included

- Per-address confirmed/unconfirmed splitting or row rendering. That is R1.6.
- New Tauri IPC commands or `BalanceDto` field changes (existing contract unchanged).
- New balance fetch hooks or new API calls. Consume `BalanceDto` verbatim.
- Send, transactions, fee bump, QR rendering, Admin ID display, new tabs, or route changes.
- Watch-only or signing-capability badges. If needed, defer to R1.6 where addresses/capability context is in scope.
- New runtime dependencies or React testing dependencies.
- Durable persistence or transaction classification beyond the existing `unconfirmedSats` value.

### Scope amendment (delivered with R1.5)

Manual regtest showed block-only `do_sync` never populated `unconfirmedSats` or mempool-driven receive rotation. **Shipped fix:** after the block loop in `WalletService::do_sync`, call `bdk_bitcoind_rpc::Emitter::mempool()` and `wallet.apply_unconfirmed_txs`. No IPC/DTO change; enables PRD §4.3.1 and R1.3 mempool semantics on regtest without mining.

## Technical Design

### Current state

```text
useAdminWalletBalance()
  -> getAdminWalletBalance()
  -> BalanceDto { confirmedSats, unconfirmedSats, totalSats }

proposals-dashboard-screen.tsx
  -> balanceSats = balanceHook.data?.confirmedSats ?? 0
  -> WalletPanelContent(balanceSats)
  -> WalletBalance(balanceSats)

broadcast-proposal-screen.tsx
  -> balanceSats = balanceHook.data?.confirmedSats ?? 0
  -> WalletPanelContent(balanceSats)
  -> WalletBalance(balanceSats)
```

`BalanceDto.unconfirmedSats` is currently dropped at the screen boundary.

### Target state

```text
useAdminWalletBalance()
  -> BalanceDto { confirmedSats, unconfirmedSats, totalSats }

screen route
  -> confirmedBalanceSats = balanceHook.data?.confirmedSats ?? 0
  -> unconfirmedBalanceSats = balanceHook.data?.unconfirmedSats ?? 0
  -> WalletPanelContent(confirmedBalanceSats, unconfirmedBalanceSats)
  -> WalletBalance(confirmedSats, unconfirmedSats)
       hero: confirmedSats
       secondary toggle line: alternate confirmed amount format
       tertiary line: formatSignedSats(unconfirmedSats) + " unconfirmed", hidden when zero
```

`BalanceDto.totalSats` remains available at the API boundary but is intentionally not used for the hero in this slice.
This avoids changing the main number from R1.2 and keeps pending activity visually distinct. If a later product decision
wants the hero to show total including unconfirmed, that should be a separate spec change because it changes the meaning
of the largest wallet number.

### Component contracts

```ts
// wallet-balance.tsx
export type WalletBalanceProps = {
	confirmedSats: number
	unconfirmedSats: number
	isLoading: boolean
}
```

Rendering rules:

- `isLoading === true`: keep the current skeleton exactly. Do not render stale or zero balance text during loading.
- `confirmedSats` is the hero value. The BTC/sats toggle continues to switch only the hero and the existing secondary
  line for confirmed balance.
- `unconfirmedSats === 0`: render no tertiary line, preserving the quiet R1.2 panel when there is no pending activity.
- `unconfirmedSats > 0`: render `+N sats unconfirmed` in muted mono styling.
- `unconfirmedSats < 0`: render `-N sats unconfirmed` using the existing typographic minus convention from
  `formatSignedSats` if reused (`−N sats`). The product copy may visually be read as negative; tests should accept the
  exact formatter output.
- Non-finite values should not normally arrive from `BalanceDto`. If the formatter receives one, existing formatter
  behavior (`—`) is acceptable; do not add defensive UI branches that hide data silently.

Recommended markup placement:

```text
WalletBalance
  hero row: large BIZ_UDPMincho amount + unit
  toggle button: "Show sats" / "Show BTC"
  secondary line: alternate confirmed amount
  tertiary line: signed unconfirmed amount, muted, hidden when zero
  sr-only: primary confirmed balance and, when present, unconfirmed balance
```

Suggested tertiary style:

```tsx
<div className="mt-1 font-mono text-[12px] text-[#6b7280]">
	{formatSignedSats(unconfirmedSats)} unconfirmed
</div>
```

Use the final class names that best match the existing component after implementation, but keep the line less prominent
than the hero and at least as readable as the secondary line.

### Screen and panel wiring

`WalletPanelContentProps` should carry prepared UI buckets:

```ts
export type WalletPanelContentProps = {
	disabledError: AdminWalletError | null
	confirmedBalanceSats: number
	unconfirmedBalanceSats: number
	isBalanceLoading: boolean
	// existing props unchanged...
}
```

`WalletPanelContent` stays a composition component: it forwards balance props to `WalletBalance` and keeps disabled-state,
receive-address, addresses, and sync rendering unchanged.

`proposals-dashboard-screen.tsx`:

- replace `balanceSats={balanceHook.data?.confirmedSats ?? 0}` with explicit confirmed and unconfirmed props.
- keep `walletDisabledError` derivation unchanged.
- keep refresh behavior unchanged: after sync, refresh balance, receive address, and addresses-with-balance.

`broadcast-proposal-screen.tsx`:

- update `WalletPanelData` from `balanceSats` to `confirmedBalanceSats` and `unconfirmedBalanceSats`.
- update `useWalletPanelData` to expose both values from `balanceHook.data`.
- pass both values to `WalletPanelContent`.
- keep `isAdminWalletMode` disabled handling unchanged.

No other wallet-panel entry points are expected. An implementation-time search for `WalletPanelContent` must confirm all
call sites were updated.

### Header polish decision

The implementation MAY update both panel headers from:

```tsx
<WalletPanelHeader title={`Session · ${sessionTimeLabel}`} subtitle={signerLabel} />
```

to:

```tsx
<WalletPanelHeader title="Admin Wallet" subtitle={`Session · ${sessionTimeLabel} · ${signerLabel}`} />
```

or an equivalent compact subtitle that preserves the same session and signer information.

This is optional. If it creates layout churn or truncated copy risk, leave the current header unchanged. Do not introduce
capability badges or watch-only status in R1.5.

### Visual and copy contract

- Hero typography remains `font-['BIZ_UDPMincho'] text-[28px]`.
- Toggle copy remains `Show sats` / `Show BTC`.
- The unconfirmed line copy is exactly suffix-based: `<signed sats> unconfirmed`.
- Use lowercase `unconfirmed`.
- Positive line starts with `+`.
- Negative line starts with the formatter's minus glyph.
- Zero line is absent, not `+0 sats unconfirmed`.
- Do not show "pending", "mempool", "unconfirmed UTXO", "BDK", "R1.5", "Phase", or "arrives in" in user-facing panel copy.

### Production code vs. test helpers

**Production functions/components**

- `WalletBalance` (presentational): renders confirmed balance and optional signed unconfirmed line.
- `WalletPanelContent` (composition): forwards confirmed and unconfirmed balance props to `WalletBalance`.
- `formatSignedSats` (existing pure model helper): formats signed sats with plus/minus. It may be reused directly.
- Route-level wiring in `proposals-dashboard-screen.tsx` and `broadcast-proposal-screen.tsx`: maps `BalanceDto` to
  panel props.

**Test helpers**

- Existing `node:assert/strict` tests under `domain/admin-wallet/model/__tests__/`.
- Any new test-only helper for source scanning must live inside a `.test.ts` file, not production code.
- No test helper is exposed as a Tauri command, API helper, hook return value, or production module export.

There are no Rust production functions and no new IPC commands in this feature.

## Test Cases

Tests target production functions and stable source contracts only.

### Pure model tests

`format-signed-sats.test.ts` already verifies:

- `formatSignedSats(1_234) -> "+1,234 sats"`
- `formatSignedSats(-48_250_000) -> "−48,250,000 sats"`
- non-finite values return `—`

Update or extend only if the implementation adds a dedicated helper such as `formatUnconfirmedBalanceLine(sats)`.

If a dedicated helper is introduced, test:

- `1_234 -> "+1,234 sats unconfirmed"`
- `-48_250_000 -> "−48,250,000 sats unconfirmed"`
- `0 -> null` or equivalent "do not render" value
- `NaN` / `Infinity -> "— unconfirmed"` only if preserving `formatSignedSats` behavior; otherwise avoid passing
  non-finite values into the helper.

### Architecture / source-contract tests

Update `desktop-app/src/domain/admin-wallet/architecture.test.ts` or add a similarly scoped source-contract check only if
it provides stable value without brittle JSX snapshots.

Recommended checks:

- `wallet-panel-content.tsx` passes both `confirmedSats` and `unconfirmedSats` (or the selected prop names) to
  `WalletBalance`.
- `wallet-balance.tsx` imports/reuses `formatSignedSats` or a local helper that is tested.
- user-facing wallet panel copy still has no roadmap placeholders (`arrives in Phase`, `not available yet`,
  `QR preview unavailable`, `Admin tools`) via the existing R1.2 guard.

Avoid tests that assert exact Tailwind class strings unless they guard a product-critical visual contract.

### Type/build tests

The TypeScript build must catch all updated prop contracts:

- `WalletBalanceProps` requires `confirmedSats`, `unconfirmedSats`, and `isLoading`.
- `WalletPanelContentProps` requires both balance buckets.
- `WalletPanelData` in `broadcast-proposal-screen.tsx` exposes both balance buckets.
- all `WalletPanelContent` and `WalletBalance` call sites compile.

### Manual / regtest smoke

Document these in the PR test plan. Run when a regtest stack is available:

1. Login with mnemonic or hardware-wallet-backed session and open the wallet panel.
2. With no unconfirmed wallet activity, verify the panel shows the confirmed hero and alternate-format secondary line, with
   no unconfirmed line.
3. Send funds to the displayed receive address without mining a block; sync the wallet. Verify the panel shows a positive
   `+N sats unconfirmed` line and the confirmed hero has not increased yet.
4. Mine a block and sync again. Verify the unconfirmed line disappears and the confirmed hero reflects the credited funds.
5. Create an unconfirmed outgoing Admin Wallet movement when the send path is available, or use a fixture/dev path if
   available. Verify the line is negative, e.g. `−N sats unconfirmed`.
6. Stop or break chain RPC after data has loaded. Verify stale balance remains visible and the existing `SyncChip` error
   behavior is unchanged.
7. Repeat panel open on `/proposals` and `/proposals/:actionId/broadcast`; both render the same balance treatment.

### Regression commands

Because the change is frontend-only, required local checks for this feature are:

```bash
cd desktop-app
npm run test:model-format-signed-sats
npm run test:architecture
npm run format:check
npm run lint
npm run build
```

The full SDD verification phase still runs the repository-wide commands from the skill. No Rust changes are expected; if
any Rust check fails due to unrelated environment or pre-existing issues, document it separately and do not mask it with
frontend changes.

## Module structure

No new modules are required. Changes should remain in existing files, each with one clear responsibility:

| File | Single responsibility after R1.5 |
|---|---|
| `components/wallet-balance.tsx` | Render the wallet-level confirmed balance, BTC/sats toggle, loading skeleton, and optional signed unconfirmed line. |
| `components/wallet-panel-content.tsx` | Compose the enabled wallet panel body and pass prepared balance/address/sync props to presentational children. |
| `screens/proposals-dashboard-screen.tsx` | Compose dashboard data hooks and route-level wallet-panel props for the proposals dashboard. |
| `screens/broadcast-proposal-screen.tsx` | Compose broadcast flow hooks and route-level wallet-panel props for the broadcast screen. |
| `model/format-signed-sats.ts` | Format a signed sats delta for UI copy. |
| `architecture.test.ts` | Enforce stable admin-wallet dependency and copy contracts where source-level checks are appropriate. |

Dependency direction remains:

```text
screens -> domain hooks / domain components -> domain model helpers
api/admin-wallet.ts -> Tauri IPC boundary
components -/-> api/admin-wallet.ts
model -/-> react
```

`WalletBalance` must not import `BalanceDto` or call `useAdminWalletBalance`; it receives view-ready numeric props. This
keeps transport DTOs at the screen/hook boundary and preserves presentational component isolation.

## Signer safety / session compatibility

- No private keys, xpubs, PSBTs, signatures, or device prompts are introduced or displayed.
- The same UI behavior applies to mnemonic sessions, hardware-wallet sessions, and watch-only sessions because it consumes
  the read-only `BalanceDto`.
- Disabled wallet states remain driven by existing `Disabled` and `RegtestGuardViolation` errors.
- Sync failure handling remains unchanged: visible stale data plus `SyncChip` error, not destructive clearing.
- Authority context is unchanged; the wallet panel remains reachable from the existing Strata/Alpen administrator routes.

## Edge cases and decisions

- **Zero unconfirmed:** hide the line. The user does not need noise for a zero delta.
- **Positive unconfirmed:** incoming pending movement, display with `+`.
- **Negative unconfirmed:** outgoing pending movement or net outgoing effect, display with minus.
- **Netting:** render the existing `unconfirmedSats` net value exactly. Do not attempt to split incoming vs outgoing in
  R1.5.
- **Total sats:** do not compute or display `confirmedSats + unconfirmedSats` in this slice. `BalanceDto.totalSats`
  remains unused unless the spec is revised.
- **Formatting:** use locale grouping for sats through the existing formatter. BTC precision remains delegated to
  `formatBtcFromSats`.
- **Accessibility:** keep the existing `sr-only` primary balance text and include the unconfirmed line in screen-reader
  text when it is visible.
- **Visual hierarchy:** unconfirmed line must be less prominent than the hero; it should inform, not alarm.

## Done when

- The wallet panel hero still shows confirmed balance.
- A separate signed unconfirmed line appears only when `unconfirmedSats !== 0`.
- The line says `unconfirmed` and uses positive/negative signed sats copy.
- Dashboard and broadcast wallet panels behave consistently.
- Existing loading, disabled, stale data, sync error, receive address, and addresses-with-balance behavior is unchanged.
- PRD §4.3.1 can be marked PASS in the spec-compliance matrix.
- Frontend checks are green; full SDD verification is reported.

## Links

- Program plan: [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) (R1.5)
- PRD source: [`../0-prd/03-prd-update.md`](../0-prd/03-prd-update.md) (§4.3.1)
- Predecessor UI cleanup: [`admin-wallet-clean-wallet-ui.md`](./admin-wallet-clean-wallet-ui.md) (R1.2)
- Receive rotation: [`admin-wallet-receive-rotation.md`](./admin-wallet-receive-rotation.md) (R1.3)
- Next slice: [`admin-wallet-addresses-ux.md`](./admin-wallet-addresses-ux.md) (R1.6, per-address unconfirmed breakdown)
- Frontend rules: `.claude/rules/react-frontend-patterns.md`, `.claude/rules/typescript-standards.md`
- Visual source: `miniwallet/Alpen-v0.1-Alta-handoff/Alpen v0.1 - Alta/WalletPanel/*`
