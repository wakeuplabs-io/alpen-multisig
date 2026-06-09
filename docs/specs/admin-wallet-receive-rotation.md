# Spec: Admin Wallet — Receive rotation (R1.3)

Implements **Release 1, step R1.3** of the [Admin Wallet implementation plan](./admin-wallet-implementation-plan.md). Source of truth: **PRD §4.3.4**.

## Objective

The Receive tab must surface a **fresh, unused** Admin Wallet receive address and **rotate** to the next unused index once the currently displayed address has been credited (its script appears in an observed transaction during sync). This prevents address reuse and matches BDK's gap-limit derivation model.

**Done when:** On regtest, after incoming funds to the displayed receive address confirm, the displayed receive address rotates to the next unused external index.

## Scope

### Included

- A production `WalletService` method that returns the **next unused external (receive) address** using BDK's native gap-aware derivation (`next_unused_address`).
- A Tauri IPC command exposing it, plus the frontend API wrapper and a thin React hook.
- Wiring the wallet panel (both `proposals-dashboard-screen` and `broadcast-proposal-screen`) to source the receive address from this command, replacing the fragile front-end `find((a) => !a.isUsed)` window scan.
- Refresh of the receive address after a sync so rotation is observable in the UI.

### NOT included

- QR rendering for the receive address (Phase 6).
- Admin ID display/copy (Phase 6).
- Persistence of the revealed index across process restarts (BDK wallet is `create_wallet_no_persist`; the next-unused index is recomputed from chain state on sync — see Edge cases).
- Any change to commit/reveal protocol, signing, or the internal (change) keychain (`reveal_change_address` is untouched).
- Send flow (Phase 4).

## Technical Design

### Rotation semantics (decision)

BDK's `Wallet::next_unused_address(KeychainKind::External)` returns the **lowest external index that has not yet been used**, revealing addresses as needed. "Used" is set by BDK when the script is observed in any transaction during `apply_block_*` (sync), including a still-unconfirmed mempool tx.

- The method is **idempotent**: repeated calls return the *same* address until that address is used.
- Once a sync observes a crediting transaction for the displayed address, the next call returns the next index → **rotation**.

**Decision:** rotation is keyed on BDK "used" (observed-in-a-tx), not on a confirmation-count threshold. Rationale: avoiding address reuse the moment a credit is seen is the security-correct behavior, and it is the idiomatic BDK model. On regtest the e2e mines a block to confirm the credit; the displayed address has rotated by the time the balance confirms, satisfying the PRD §4.3.4 "done when". This nuance is documented in code and in the evolution note.

### Production code vs. test helpers

**Production functions**

- `WalletService::next_receive_address(&self) -> Result<AddressDto, AdminWalletError>`
  - Locks the wallet, calls `wallet.next_unused_address(KeychainKind::External)`, maps to `AddressDto { index, address, is_used: false }`.
  - Reuses the existing `AddressDto` DTO (no new DTO type).
- Tauri command `admin_wallet_next_receive_address(wallet_session) -> Result<AddressDto, String>`
  - Resolves the service via `current_or_fallback()` (same pattern as the other read commands) and serializes errors with the existing `serialize_wallet_error`.
- Frontend `nextAdminWalletReceiveAddress(): Promise<ApiResult<AddressDto>>` in `api/admin-wallet.ts`.
- Frontend hook `useAdminWalletReceiveAddress()` (fetch + `refresh()`), mirroring `useAdminWalletBalance`/`useAdminWalletAddresses`.

**Test helpers** (test-only, never registered as commands)

- The existing `load_admin_wallet(TEST_MNEMONIC, …)` fixture used across `WalletService` tests.

### IPC contract

| Command | Args | Returns |
|---|---|---|
| `admin_wallet_next_receive_address` | none | `AddressDto` `{ index, address, isUsed }` |

`AddressDto` already exists on both sides (`serde rename_all = camelCase` → `index`, `address`, `isUsed`).

### Flow

```text
Receive tab opens / sync completes
  └─ useAdminWalletReceiveAddress()
       └─ admin_wallet_next_receive_address (IPC)
            └─ WalletService::next_receive_address()
                 └─ wallet.next_unused_address(External)  ← BDK gap-aware reveal
       ⇒ AddressDto (lowest unused external index)

Funds sent to displayed address → admin_wallet_sync observes the tx
  └─ BDK marks that index "used"
       ⇒ next call to next_receive_address returns the next index (rotation)
```

### Frontend wiring

- Replace `const receiveAddress = addressesHook.data?.find((a) => !a.isUsed)?.address ?? null` in both screens with the hook's `address`.
- In the panel `onRefreshSync` handler, also call the receive hook's `refresh()` after `triggerSync()` so the displayed address rotates once the credit is observed.

## Test Cases

Tests target production functions only.

**Backend (`WalletService`, unit/`tokio::test` with `load_admin_wallet` fixture)**

- Happy path: `next_receive_address()` on a fresh wallet returns external index `0`, `is_used = false`, address starts with the network HRP (`bcrt1p…` on regtest).
- Idempotency: two consecutive `next_receive_address()` calls **return the same address** (no use observed in between) — the rotation guard. (Contrast with the existing `reveal_change_address` test, which asserts *distinct* addresses on the internal keychain.)
- Watch-only compatibility: `next_receive_address()` on a `new_watch_only` wallet returns Ok (pure derivation, no signing / no `ReadOnly`).
- Network HRP: address is parseable and matches `self.network`.

**IPC command (`commands/admin_wallet.rs`)**

- `admin_wallet_next_receive_address` is importable and takes `tauri::State<WalletSession>` (compile-time check, matching existing command tests).
- Disabled session: with an empty `WalletSession`, the command surfaces the tagged `Disabled` error via `serialize_wallet_error` (consistent with the other read commands using `current_or_fallback`).

**Frontend (vitest/node tests, mirroring existing admin-wallet hook tests)**

- `nextAdminWalletReceiveAddress` is exported and invokes the `admin_wallet_next_receive_address` command.
- `useAdminWalletReceiveAddress` is exported, exposes `{ address, isLoading, error, refresh }`, and parses `AdminWalletError`.
- Architecture test: screens no longer compute the receive address via `find((a) => !a.isUsed)` (guards against regressing to the window-scan heuristic) — only if an architecture test already exists for this domain; otherwise covered by the hook unit test.

**E2E / regtest (manual or webdriver smoke, documented)**

- Send funds to the displayed receive address, mine to confirm, re-sync → the displayed receive address advances to the next unused index.

## Module structure

No new modules. Changes live in existing files, each retaining a single responsibility:

- `application/wallet_service.rs` — add `next_receive_address` next to `reveal_change_address` (both are "derive an address from a keychain"). One sentence: *owns BDK wallet read/derive/sign operations.*
- `commands/admin_wallet.rs` — add the thin IPC command. One sentence: *maps admin-wallet IPC calls to `WalletService`, serializing typed errors.*
- `commands/invoke.rs` — register the new command in the same handler groups as the other read commands.
- `api/admin-wallet.ts` — add the API wrapper. One sentence: *typed IPC bridge for admin-wallet commands.*
- `domain/admin-wallet/hooks/use-admin-wallet-receive-address.ts` (new, ~40 lines) — one hook, one responsibility: *fetch + refresh the next receive address.* Exported from `hooks/index.ts`.
- `screens/proposals-dashboard-screen.tsx`, `screens/broadcast-proposal-screen.tsx` — swap the receive-address source and add the hook's refresh to `onRefreshSync`.

**Dependency direction:** screens → hook → api → IPC → `WalletService` → BDK. Business logic (`WalletService`) depends only on the `bdk_wallet` abstraction; the IPC layer depends on `WalletService`, not the reverse. No new abstraction or trait is introduced (matches existing read-path pattern; signer port unaffected).

## Signer safety / session compatibility

- No private key material is touched; `next_unused_address` is pure public-derivation and works identically for mnemonic, HW (watch-only), and software-signer sessions.
- No new `WalletSession` lifecycle behavior; the command uses the existing `current_or_fallback()` guard and returns `Disabled` when no session is active.

## Edge cases (notes)

- **Unconfirmed funds:** BDK marks an address used as soon as a crediting tx is observed during sync (mempool-included). The displayed receive address therefore rotates on first observation, before confirmation — intended (prevents reuse). **Requires mempool sync in `do_sync`** (added in R1.5 — see [`admin-wallet-balance-ux.md`](./admin-wallet-balance-ux.md)); block-only sync does not rotate until a block is mined.
- **Repeated sync / repeated calls:** idempotent; the same next-unused address is returned until it is used. No index "leak" from merely opening the Receive tab.
- **App restart / session change:** the BDK wallet is non-persistent (`create_wallet_no_persist`); on a fresh session the revealed index starts at 0 but `next_unused_address` recomputes the correct next-unused index from on-chain usage after the first sync. Logout drops the `WalletService`; login rebuilds it. No stale address is shown after a sync.
- **Address window exhaustion:** the old front-end heuristic could return `null` once all 20 windowed addresses were used; the BDK-native method has no such ceiling (gap-limit aware), removing that failure mode.
