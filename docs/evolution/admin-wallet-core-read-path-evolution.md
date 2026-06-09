# Evolution: admin-wallet-core-read-path (Phase 2)

**Shipped:** 2026-05-27  
**Branch:** feature/admin-wallet-phase2  
**Steps:** 8 (4 Rust + 2 React + 2 tests)

## What shipped

Phase 2 of the Admin Wallet program. Adds the BDK-backed read path for the Admin Wallet over Tauri IPC, enabling the broadcast screen to show UTXO count and last-sync timestamp, and laying the typed data backbone for the Phase 3 WalletPanel.

## Commits

| Commit | Description |
|---|---|
| `4597485` | WalletService struct + all DTOs + AdminWalletError extensions |
| `c5f7954` | get_balance, list_utxos, list_addresses methods |
| `d88d5dd` | sync(), sync_status(), background 30s loop |
| `b5e4065` | 5 Tauri IPC commands + WalletService managed state |
| `cf9801f` | TS adapter extensions + 4 React hooks |
| `a90b61c` | BroadcastDetailsCard extension + screen wiring |
| `ab42c5a` | Rust integration and guard tests |
| `827b980` | TypeScript hook and component tests |
| `6b74e35` | L1-L6 refactoring pass |

## Files added / modified

**Rust:**
- `desktop-app/src-tauri/src/application/wallet_service.rs` (NEW — WalletService, all DTOs, sync loop)
- `desktop-app/src-tauri/src/application/mod.rs` (re-export)
- `desktop-app/src-tauri/src/infrastructure/admin_wallet/wallet.rs` (AdminWalletError extended)
- `desktop-app/src-tauri/src/commands/admin_wallet.rs` (5 new IPC commands)
- `desktop-app/src-tauri/src/commands/invoke.rs` (command registration)
- `desktop-app/src-tauri/src/main.rs` (WalletService managed state)
- `desktop-app/src-tauri/tests/admin_wallet_integration.rs` (NEW — integration + guard tests)

**TypeScript/React:**
- `desktop-app/src/api/admin-wallet.ts` (5 new IPC adapter functions + types)
- `desktop-app/src/domain/admin-wallet/hooks/use-admin-wallet-balance.ts` (NEW)
- `desktop-app/src/domain/admin-wallet/hooks/use-admin-wallet-utxos.ts` (NEW)
- `desktop-app/src/domain/admin-wallet/hooks/use-admin-wallet-addresses.ts` (NEW)
- `desktop-app/src/domain/admin-wallet/hooks/use-admin-wallet-sync.ts` (NEW)
- `desktop-app/src/domain/admin-wallet/hooks/parse-admin-wallet-error.ts` (NEW — shared helper)
- `desktop-app/src/domain/admin-wallet/hooks/index.ts` (NEW — barrel)
- `desktop-app/src/domain/broadcast-proposal/components/broadcast-details-card.tsx` (utxoCount + lastSyncedAt + syncError props)
- `desktop-app/src/screens/broadcast-proposal-screen.tsx` (hook wiring)
- `desktop-app/src/domain/admin-wallet/hooks/__tests__/` (NEW — hook + component tests)

## Key decisions retained

- **Regtest-only guard**: AdminWalletError::Disabled when COMMIT_FUNDING≠admin_wallet OR BITCOIN_NETWORK≠regtest OR ALLOW_DEV_MNEMONIC_SIGNING≠1
- **Single Wallet mutex**: short-snapshot reads, sync holds the lock for full emitter cycle
- **sync_in_flight AtomicBool**: concurrent sync() calls collapse to one in-flight
- **Background loop**: 30s cadence, 5-min idle window (Rust constants, no env var)
- **No new env vars**: reuses Phase 1 set exactly (D6)
- **Secrets stay in Rust**: mnemonic never crosses IPC boundary

## Deviations from spec

None. All spec requirements implemented as specified.

## Refactoring findings (L1-L6 pass)

- **Bug fixed (L2)**: `admin_wallet_get_sync_status` → `admin_wallet_sync_status` (command name mismatch)
- **Bug fixed (L5)**: `lastError.kind` → `lastError.code` (matched actual Rust TypedError serialization)
- **Bug fixed (L5)**: `listAdminWalletAddresses` was missing `keychain` and `pageIndex` params
- **L3**: Extracted `parseAdminWalletError` to shared module (was copy-pasted in 4 hooks)

## Execution issues

- Worktree isolation caused agents to work on the base branch (without predecessor commits), leading to empty commits in some steps. Mitigated by running later steps without isolation.
- Disk-full (ENOSPC) during L3 refactoring pass; resolved by manual cleanup.
- DES execution log entries for steps 01-01, 01-02, 03-01, 03-02, 04-01, 04-02 reconstructed from git Step-ID commit trailers after log file was overwritten.

## Next phase

Phase 3 — WalletPanel (Balance/Addresses/Transactions/Receive tabs) — consumes the hooks established here.
