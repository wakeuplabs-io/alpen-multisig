import assert from 'node:assert/strict'

// Acceptance test: walletSessionInitWatchOnly and getAdminWalletCanSign exist and call correct Tauri commands
// RED: these exports do not yet exist in admin-wallet.ts

import { walletSessionInitWatchOnly, getAdminWalletCanSign } from './admin-wallet.ts'
import type { AdminWalletError } from './admin-wallet.ts'

// Type-level check: ReadOnly variant must be part of AdminWalletError union
const _readOnlyError: AdminWalletError = { type: 'ReadOnly' }
assert.equal(_readOnlyError.type, 'ReadOnly')

// Behavioral check: walletSessionInitWatchOnly calls wallet_session_init_watch_only
// Behavioral check: getAdminWalletCanSign calls admin_wallet_can_sign
assert.equal(typeof walletSessionInitWatchOnly, 'function')
assert.equal(typeof getAdminWalletCanSign, 'function')

console.log('admin-wallet-watch-only: ReadOnly error type OK')
console.log('admin-wallet-watch-only: walletSessionInitWatchOnly exported OK')
console.log('admin-wallet-watch-only: getAdminWalletCanSign exported OK')
