# Spec: Admin Wallet — Canonical connect paths (R1.4)

> Release 1, step **R1.4 — Remove connect-time derivation picking**.
> Program plan: [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) (traceability row R1.4).
> Predecessor: R1.3 Receive rotation ([`admin-wallet-receive-rotation.md`](./admin-wallet-receive-rotation.md)).

## Objective

Remove the connect-flow step where the user manually picks a derivation path / address
index from a list of 20 derived addresses (today: `adapter.listAddresses(20)` → `picking`
phase → "Use address"). On hardware-wallet (Trezor/Ledger) and mnemonic connect, derive the
**Admin ID** and register the session at the **canonical paths only**, with no manual
path-selection UI.

**Why:** PRD §3.2 specifies a single canonical derivation for the Admin ID
(`m/84'/0'/73'/0/0`, P2WPKH; Ledger uses testnet coin type `m/84'/1'/73'/0/0` on
regtest/testnet) and the Admin Wallet account (`m/86'/0'/73'`). Letting the user pick an
arbitrary index is dev scaffolding from the POC: it invites a signer to authenticate with a
key that is not on the canonical signer set, produces confusing "not a member" failures, and
diverges from the protocol's single-identity model. The Admin Wallet is already session-bound
at its canonical account path (Phases 3.7–3.8, R1.1–R1.3); this step aligns the **connect**
flow with that same canonical-only model.

## Scope

### In scope

- **Frontend connect flow:** collapse the `connect → picking → selected` state machine to
  `connect → selected`. On connect, build a single canonical signer entry (index 0, canonical
  Admin ID path) from `adapter.connect()` and advance straight to authority selection.
- Remove the `picking` phase, the `PickingPhase` component, and the per-index selection
  actions (`selectAddressIndex`, `useAddress`, `changeAddress`).
- Remove `listAddresses` from the `WalletAdapter` contract and from the Trezor, Ledger, and
  mnemonic adapters.
- Mnemonic connect derives **only** the canonical index 0 (request count `1` instead of `20`).
- Renumber the connect wizard step labels (`Step N of 4` → `Step N of 3`) and update connect
  copy that referenced "available addresses" / "address selection".
- **Backend:** remove the now-unused Tauri commands `list_hw_addresses` (Trezor) and
  `list_ledger_addresses` (Ledger), their `invoke.rs` registrations, and the
  `trezor::list_addresses` / `ledger::list_addresses` infrastructure functions.
- **E2E:** drop `pickingRowIndex` and the picking-row clicks from `login-mnemonic.mjs`; retire
  the `proposal-co-sign-row1` spec (which depended on selecting row #1 of the same mnemonic),
  including its npm script and README section.

### NOT in scope

- Admin ID display/copy UI and QR (Phase 6).
- Send-on-HW, verify-address-on-device for receive (Phase 7). The existing connect-time
  "Verify key/path on device" affordance (`verify_address_on_device`) is **kept** — it now
  verifies the canonical path only.
- `list_mnemonic_addresses` command removal — it remains as the mnemonic derivation primitive
  used by mnemonic connect.
- Changing the Admin Wallet account path or descriptor derivation (already canonical).
- Seeding a second canonical signer for multi-signer e2e — **delivered in PR #206** via `DEMO_MNEMONIC_COSIGN` and `proposal-co-sign-mnemonic`.
- Payout Administrator flows.

## Technical Design

### Current flow (before)

```
ConnectPhase ──connect()──┐
                          │ adapter.listAddresses ? 
                          ├── yes ─→ PickingPhase (Step 2/4) ──useAddress()──┐
                          │                                                  │
                          └── no  ─────────── single fallback entry ─────────┤
                                                                             ▼
                          AuthoritySelectionPhase (Step 3/4) ─→ AuthenticateSessionPhase (Step 4/4)
```

### Target flow (after)

```
ConnectPhase ──connect()──→ single canonical entry (index 0, canonical Admin ID path)
                                                                             ▼
                          AuthoritySelectionPhase (Step 2/3) ─→ AuthenticateSessionPhase (Step 3/3)
```

The "no `listAddresses`" branch that already exists in `use-hw-wallet-connect.ts` becomes the
**only** branch: `connect()` derives the canonical signer entry directly from the
`WalletAccountInfo` returned by `adapter.connect()` (`derivationPath`, `addressSample`,
`xpubOrFingerprint`) and transitions to `selected`.

### Production code

Frontend (`desktop-app/src/`):

- `domain/connect-wallet/model/hw-wallet-connect.types.ts`
  - `HwWalletPhase`: `'connect' | 'selected'` (drop `'picking'`).
  - `HwWalletConnectState`: drop `addresses` and `selectedIndex`.
- `domain/connect-wallet/hooks/use-hw-wallet-connect.ts`
  - `connect()` always builds the single canonical entry and goes to `selected`.
  - Remove `addresses`/`selectedIndex` state, `selectAddressIndex`, `useAddress`,
    `changeAddress`. Keep `connect`, `goBackToConnect`, `verifyOnDevice`, `disconnect`.
- `domain/connect-wallet/components/picking-phase.tsx` — **deleted**.
- `domain/connect-wallet/components/hw-wallet-connect.tsx`
  - Remove the `picking` branch + `PickingPhase` import; `isWidePhase` no longer keys on
    `picking`.
  - Wire the authority "Back" action to `goBackToConnect` (returns to the connect screen).
- `domain/connect-wallet/components/authority-selection-phase.tsx`
  - Rename prop `onBackToAddresses` → `onBack`; step label `Step 3 of 4` → `Step 2 of 3`.
- `domain/connect-wallet/components/authenticate-session-phase.tsx`
  - Step label `Step 4 of 4` → `Step 3 of 3`.
- `domain/connect-wallet/components/selected-phase.tsx`
  - Remove the "Change address" button and `onChangeAddress` prop (no addresses to change);
    keep "Verify key/path on device".
- `domain/connect-wallet/components/connect-phase.tsx`
  - Update copy: success subtitle "Loading available addresses…" → session-loading wording;
    "Advancing to address selection…" → "Advancing to authority selection…".
- `wallet/types.ts` — remove optional `listAddresses` from `WalletAdapter`. Keep
  `HwAddressEntry` (still the shape of the single canonical `selectedEntry`).
- `wallet/trezor-adapter.ts`, `wallet/ledger-adapter.ts` — remove `listAddresses` method.
- `wallet/mnemonic-adapter.ts` — remove `listAddresses` method; `connect()` requests
  `count: 1` from `list_mnemonic_addresses` and uses index 0.

Backend (`desktop-app/src-tauri/src/`):

- `commands/hw_wallet.rs` — remove `list_hw_addresses` and `list_ledger_addresses` commands.
- `commands/invoke.rs` — remove both registrations (production + dev-signing handler sets).
- `infrastructure/hw_wallet/trezor.rs` — remove `list_addresses`.
- `infrastructure/hw_wallet/ledger.rs` — remove `list_addresses` and
  `list_addresses_unlocked`.
- `infrastructure/hw_wallet/mod.rs` — keep `HwAddressEntry` (still used by `connect` / xpub).

### Test helpers

- `desktop-app/e2e-webdriver/test/helpers/login-mnemonic.mjs` — production-adjacent e2e helper:
  drop `pickingRowIndex` and the `e2e-picking-row-*` / `e2e-picking-continue` interactions.
- No new test helpers are exposed as Tauri commands.

## Test Cases

Tests target production functions only.

### Frontend (`use-hw-wallet-connect` behavior)

1. **Happy path — HW connect goes straight to `selected`.** After `connect()` resolves with a
   `WalletAccountInfo`, `state.phase === 'selected'`, `state.selectedEntry` holds the canonical
   path/address/pubkey, and `onConnected` is called once with the canonical info. No `picking`
   phase is ever entered.
2. **Canonical entry shape.** `selectedEntry` = `{ index: 0, derivationPath: info.derivationPath,
   address: info.addressSample, publicKeyHex: info.xpubOrFingerprint }`.
3. **Connect error path.** When `adapter.connect()` rejects, `state.error` is set,
   `connectViewState` returns to `idle`, and phase stays `connect`.
4. **Back from authority returns to connect.** `goBackToConnect()` resets phase to `connect`
   and clears `selectedEntry`/error.
5. **Adapter contract.** `WalletAdapter` no longer declares `listAddresses`; adapters compile
   without it (type-level assertion / build).

### Backend (Rust)

6. **No `list_addresses` command surface.** `list_hw_addresses` / `list_ledger_addresses` are
   absent from the invoke handler registration (compile-time: removed from `invoke.rs`).
7. **Ledger xpath regression preserved.** Existing `ledger_admin_wallet_xpub_path` tests
   (regtest→`m/86'/1'/73'`, mainnet→`m/86'/0'/73'`) still pass — proves the canonical Admin
   Wallet account path is untouched.
8. **Build is warning-free.** Removing the infra `list_addresses` fns leaves no `dead_code` or
   unused-import warnings (`cargo clippy -D warnings`).

### E2E (smoke, manual)

9. `login-mnemonic.mjs` reaches `/proposals` without any picking interaction (verified by the
   existing wallet-smoke flow once the picking step is gone).

## Module structure

No new modules. The change **removes** responsibilities rather than adding them:

- `use-hw-wallet-connect.ts` — single responsibility: drive the connect→selected state machine
  for one canonical signer. (Was previously also responsible for address-list selection.)
- `selected-phase.tsx` — display the connected canonical signer + verify-on-device.
- Adapters — single responsibility per vendor: connect + sign at the canonical path. Address
  enumeration responsibility is removed entirely.

Dependency direction is unchanged: components depend on the hook and the `WalletAdapter`
abstraction; adapters depend on the Tauri IPC bridge. No inversion is introduced.

## Known limitations (post-R1.4)

- **Multi-signer e2e (resolved in PR #206):** `proposal-co-sign-row1` was retired; replacement is
  `proposal-co-sign-mnemonic` with `DEMO_MNEMONIC_COSIGN` at the same canonical path, present in
  `asm-params` `strata_administrator.keys[1]`. After `from-scratch`, copy
  `scripts/asm-params.example.json` or re-bootstrap so `keys[1]` matches the cosign seed.
- `list_mnemonic_addresses` still derives a window when called with a larger `count`; only
  connect's `count: 1` usage changes. The command is retained as the derivation primitive.
