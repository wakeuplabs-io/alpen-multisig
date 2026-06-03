# Spec: Admin Wallet - Addresses UX (R1.6)

> **Status:** Complete — branch `feature/admin-wallet-addresses-ux`, PR [#212](https://github.com/wakeuplabs-io/alpen-multisig/pull/212).
> Evolution: [`docs/evolution/2026-06-03-admin-wallet-addresses-ux.md`](../evolution/2026-06-03-admin-wallet-addresses-ux.md).
> Manual regtest verified (per-address confirmed + unconfirmed sub-line).

Implements **Release 1, step R1.6** of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md).
Source of truth: **PRD §4.3.2**.

R1.6 closes the remaining Release 1 Admin Wallet UX gap: each funded Admin Wallet address must show its confirmed
balance and make any pending per-address effect visible.

## Objective

The Admin Wallet slide-over must show every external Admin Wallet address that currently holds a balance, with a
per-address balance breakdown that distinguishes confirmed funds from pending activity.

PRD §4.3.2 requires the signer to see:

- each Admin Wallet address that holds a balance; and
- the current balance of each address, net of unconfirmed transactions.

The existing Phase 2 read path already exposes `UtxoDto.confirmations` and derivation metadata through
`listAdminWalletUtxos()`. The current R1.2/R1.5 UI maps those UTXOs into `AddressWithBalanceView.balanceSats`, which
sums all UTXOs for a derivation index. That hides whether an address balance is confirmed, pending, or a mix of both.

**Product decision for this slice:** keep the primary row balance on confirmed sats and render pending activity as a
muted signed sub-line. This mirrors R1.5 wallet-level balance semantics:

```text
confirmed balance + signed unconfirmed line = effective balance once pending activity confirms or leaves the mempool
```

**Done when:** an address row with non-zero pending activity shows the confirmed amount as the primary value and a
`+N sats unconfirmed` or `-N sats unconfirmed` sub-line; rows with no pending activity omit that sub-line.

## Scope

### Included

- Frontend-only model and presentation changes under `desktop-app/src/domain/admin-wallet/`.
- Extend `AddressWithBalanceView` with:
  - `confirmedSats`: sum of external UTXOs for the address where `confirmations > 0`.
  - `unconfirmedSats`: sum of external UTXOs for the address where `confirmations === 0`.
  - `balanceSats`: retained as total row balance (`confirmedSats + unconfirmedSats`) only if needed for compatibility
    during the refactor.
- Update `composeAddressesWithBalance` so it buckets UTXOs by external derivation index and confirmation state.
- Filter rows by effective balance (`confirmedSats + unconfirmedSats > 0`), preserving the existing "addresses with
  balance only" policy.
- Update `AddressRow` so the balance cell renders:
  - confirmed balance as the primary BTC amount; and
  - a muted sats sub-line from existing R1.5 copy conventions when `unconfirmedSats !== 0`.
- Update `AddressesWithBalanceList` header copy to `Addresses with balance · N`.
- Keep the addresses accordion collapsed by default and preserve R1.2 empty, loading, and error states.
- Keep full address text available via the existing `title` attribute; per-row copy is optional polish only if it fits
  existing component patterns without broadening the slice.
- Add a subtle watch-only badge to `WalletPanelHeader` when the caller passes signing capability context and
  `canSign === false`.
- Use `useAdminWalletCapability` at the route/panel boundary, not inside presentational row/list components.
- Unit tests for confirmed/unconfirmed splitting in `compose-addresses-with-balance`.
- Component/contract tests for row copy or header capability copy only if the implementation extracts a pure formatter or
  existing contract-test style can cover it without adding React test dependencies.

### Not included

- New Tauri IPC commands, Rust DTO changes, BDK changes, or backend changes.
- Wallet-level unconfirmed balance line; R1.5 already shipped it.
- Internal/change address listing policy changes. R1.6 keeps the existing external-address-with-balance table.
- Pagination changes beyond the existing address page behavior.
- Send BTC, Transactions, fee bump, Receive QR, Admin ID display, or hardware receive verification.
- New runtime dependencies or React testing dependencies.
- Durable transaction classification beyond the existing `UtxoDto.confirmations` contract.

### Scope guard

If manual regtest shows unconfirmed UTXOs are not visible to the frontend, first verify the R1.5 mempool sync path
(`Emitter::mempool()` plus `apply_unconfirmed_txs`) before changing R1.6 scope. Backend changes are allowed only with
evidence that the existing R1.5 sync amendment is insufficient.

## Technical Design

### Current state

```text
useAddressesWithBalance()
  -> listAdminWalletAddresses('External')
  -> listAdminWalletUtxos()
  -> composeAddressesWithBalance(addresses, utxos)
       AddressWithBalanceView {
         index,
         address,
         balanceSats, // confirmed and unconfirmed summed together
         isUsed
       }
  -> AddressesWithBalanceList
  -> AddressRow(balanceSats)
```

### Target state

```text
useAddressesWithBalance()
  -> listAdminWalletAddresses('External')
  -> listAdminWalletUtxos()
  -> composeAddressesWithBalance(addresses, utxos)
       AddressWithBalanceView {
         index,
         address,
         confirmedSats,
         unconfirmedSats,
         balanceSats, // optional total compatibility field
         isUsed
       }
  -> AddressesWithBalanceList
  -> AddressRow(confirmedSats, unconfirmedSats)
       primary: formatBtcFromSats(confirmedSats) BTC
       sub-line: formatUnconfirmedBalanceLine(unconfirmedSats), hidden when null
```

`UtxoDto.keychain === 'Internal'` remains excluded unless the user-visible listing policy changes in a future phase.
The current `groupUtxosByDerivation` helper may remain for total-balance callers, but R1.6 should introduce or inline a
small confirmation-aware grouping helper near `composeAddressesWithBalance` so the per-address view-model owns its
derivation-to-row mapping.

### Component contracts

```ts
export type AddressWithBalanceView = {
	index: number
	address: string
	confirmedSats: number
	unconfirmedSats: number
	balanceSats: number
	isUsed: boolean
}
```

```ts
export type AddressRowProps = {
	index: number
	address: string
	confirmedSats: number
	unconfirmedSats: number
	isUsed: boolean
}
```

If all call sites can move cleanly to `confirmedSats` and `unconfirmedSats`, `balanceSats` may be removed instead of
retained. If removing it causes unrelated churn, keep it as a total compatibility field and document that presentation
must not use it as the primary row value.

### Wallet header capability

`WalletPanelHeader` already defaults to `Admin Wallet` and accepts `title`/`subtitle`. R1.6 should move route usage
toward:

```text
title: Admin Wallet
subtitle: session/signing context
badge: Watch-only when canSign === false
```

The capability hook remains at the route or panel boundary. `WalletPanelHeader` stays presentational and accepts prepared
props such as `isWatchOnly` or `capabilityLabel`.

### Production code vs. test helpers

Production functions/components:

- `composeAddressesWithBalance(addresses, utxos)` maps API DTOs into the Admin Wallet address row view-model.
- `AddressRow` renders a single prepared row.
- `AddressesWithBalanceList` renders accordion state, empty/loading/error states, and the address row table.
- `WalletPanelHeader` renders title, subtitle, close control, and optional capability badge.
- `useAdminWalletCapability` remains the API-backed hook that fetches signing capability.

Test helpers:

- `makeAddress()` and `makeUtxo()` remain fixture builders under `model/__fixtures__/`.
- Any new test-only UTXO/address builders must stay in test fixture modules and must not be exported through production
  APIs or registered as Tauri commands.

## Test Cases

Tests must target production functions or public component contracts, not test-only helpers.

### Model: `composeAddressesWithBalance`

- No addresses or no UTXOs returns an empty row list.
- Confirmed external UTXO produces `confirmedSats > 0`, `unconfirmedSats === 0`, and a row is included.
- Unconfirmed external UTXO (`confirmations === 0`) produces `confirmedSats === 0`, `unconfirmedSats > 0`, and a row is
  included.
- Mixed confirmed and unconfirmed UTXOs for the same external derivation index are split into their buckets and total to
  the effective balance.
- Multiple external derivation indices produce independent rows with no cross-address leakage.
- Internal/change UTXOs are ignored by default, preserving the external-address listing policy.
- An address with `isUsed === false` but a matching UTXO still appears, because UTXOs are the source of truth for funds.

### UI / contract

- `AddressRow` uses confirmed sats for the primary BTC value.
- `AddressRow` hides the unconfirmed sub-line when `unconfirmedSats === 0`.
- `AddressRow` shows `+N sats unconfirmed` for positive pending activity.
- `AddressesWithBalanceList` header copy is `Addresses with balance · N`.
- Existing loading, empty, and error states remain unchanged.
- `WalletPanelHeader` can show `Admin Wallet` plus session subtitle and a watch-only badge without API calls inside the
  presentational component.

### Integration / manual regtest

- Fund two external Admin Wallet indices and confirm those funds. The expanded addresses list shows both rows with
  confirmed primary balances.
- Credit one external index with an unconfirmed UTXO and refresh. That row shows the confirmed primary balance plus a
  positive unconfirmed sub-line.
- If an outgoing unconfirmed spend is represented by the existing UTXO read path, the row shows the signed pending effect
  according to the DTOs available from BDK. If the DTO does not represent negative per-address effects yet, document the
  limitation instead of inventing transaction classification in the frontend.
- Mnemonic and hardware/watch-only sessions both use the same read-only address presentation; watch-only only affects the
  header badge/signing context, not the address calculation.

### Authority isolation / signer safety

- R1.6 only displays the logged-in session's Admin Wallet data; it must not introduce authority switching or cross-session
  state.
- No private keys, mnemonics, xpub internals beyond already-displayed addresses, or signing details are logged or rendered.

### Offline fallback

- Existing wallet disabled/RPC error states remain unchanged. R1.6 does not introduce a backend dependency or require the
  orchestrator for address rendering.

## Module structure

- `desktop-app/src/domain/admin-wallet/model/compose-addresses-with-balance.ts`: owns conversion from Admin Wallet
  address/UTXO DTOs to row view-models.
- `desktop-app/src/domain/admin-wallet/model/__tests__/compose-addresses-with-balance.test.ts`: verifies the production
  mapper contract with fixture DTOs.
- `desktop-app/src/domain/admin-wallet/components/address-row.tsx`: renders one address row from prepared row props.
- `desktop-app/src/domain/admin-wallet/components/addresses-with-balance-list.tsx`: renders the collapsible address table
  and its loading/empty/error states.
- `desktop-app/src/domain/admin-wallet/components/wallet-panel-header.tsx`: renders panel title, subtitle, close button,
  and optional capability badge.
- `desktop-app/src/domain/admin-wallet/hooks/use-admin-wallet-capability.ts`: continues to own capability polling and API
  parsing; no presentation components should call it directly.

Dependency direction stays within the existing frontend domain boundary: route screens and panel composition call hooks,
hooks and model functions derive view-models, and presentational components render prepared props.

## Verification Plan

During SDD delivery, run the smallest test at each red/green step, then the full feature and frontend checks:

```bash
cd desktop-app
npm run test:model-compose-addresses-with-balance
npm run test:architecture
npm run format:check
npm run lint
npm run build
```

Before PR handoff, also run the repository pre-commit checks required by `AGENTS.md`:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Delivered

- **Model:** `groupUtxoBalancesByDerivation` splits external UTXOs by `confirmations`; `composeAddressesWithBalance` exposes
  `confirmedSats`, `unconfirmedSats`, and `balanceSats` per row.
- **UI:** `AddressRow` confirmed BTC hero + `formatUnconfirmedBalanceLine` sub-line; `Addresses with balance · N` accordion;
  per-row `CopyButton`; full address on `title`.
- **Header:** `Admin Wallet` title, `Session · … · signer` subtitle, **Watch-only** badge when `canSign === false` on
  dashboard and broadcast panels.
- **Tests:** extended compose tests, `group-utxo-balances-by-derivation` tests, address-row contract test, architecture
  Rule 6.

## Done when

- Met on regtest — funded addresses show confirmed hero; unconfirmed credit without mine shows per-row sub-line; line hidden
  when fully confirmed at address level.
- PRD §4.3.2 **PASS** in [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md); Release 1 closed for R1 slices (§4.3.3–§4.3.5 still FAIL).
- Frontend and workspace CI green.

## Release 1 closure

- `docs/specs/admin-wallet-implementation-plan.md` — R1.6 ✅; PRD status in `admin-wallet-prd-compliance.md`.
- `docs/evolution/2026-06-03-admin-wallet-addresses-ux.md` — evolution record.
- **Next program increment:** Phase 4 (Send BTC happy path).

## Links

- Program plan: [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md)
- PRD source: [`../0-prd/03-prd-update.md`](../0-prd/03-prd-update.md) (§4.3.2)
- Predecessor: [`admin-wallet-balance-ux.md`](./admin-wallet-balance-ux.md) (R1.5 wallet-level unconfirmed line)
- Core read path: [`admin-wallet-core-read-path.md`](./admin-wallet-core-read-path.md) (`UtxoDto.confirmations`, R1.5 mempool sync)
- PR: https://github.com/wakeuplabs-io/alpen-multisig/pull/212
