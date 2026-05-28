// useAddressesWithBalance hook — pure TypeScript contract test.
//
// SCOPE: Only the pure-TypeScript contracts (export surface, type shapes, forbidden imports)
// are covered here using the project's existing tsx test runner.
//
// React hook behaviour (composition of underlying hooks) CANNOT be tested without a React
// testing framework (vitest + @testing-library/react).
// Those tests are blocked by a missing dev-dependency.
// See: BLOCKED_BY_DEPENDENCY — vitest + @testing-library/react not installed.

import assert from 'node:assert/strict'

// ── 1. Export surface ────────────────────────────────────────────────────────
// Verify the hook is exported from its module and is a function.

import { useAddressesWithBalance } from '../use-addresses-with-balance.ts'

assert.equal(typeof useAddressesWithBalance, 'function', 'useAddressesWithBalance must be exported as a function')
console.log('useAddressesWithBalance: export surface OK')

// ── 2. AddressWithBalanceView type contract ───────────────────────────────────
// Verify the view type fields are accepted by the TypeScript compiler.
// This is enforced at compile time; the tsx build will fail if the shape is wrong.

import type { AddressWithBalanceView } from '../use-addresses-with-balance.ts'

const _sample: AddressWithBalanceView = {
	index: 0,
	address: 'bc1q...',
	balanceSats: 1000,
	isUsed: true,
}

void _sample
console.log('AddressWithBalanceView: all required fields accepted OK')

// ── 3. No Tauri import in hook module ─────────────────────────────────────────
// The hook must not import from @tauri-apps/api/core directly.
// It should compose useAdminWalletAddresses and useAdminWalletUtxos instead.

import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const hookSource = readFileSync(join(__dirname, '..', 'use-addresses-with-balance.ts'), 'utf8')

assert.ok(!hookSource.includes('@tauri-apps/api/core'), 'hook must not import from @tauri-apps/api/core')
console.log('useAddressesWithBalance: no Tauri direct imports OK')

// ── 4. Composes the two underlying hooks ─────────────────────────────────────
// Verify the hook source references both underlying hooks (composition contract).

assert.ok(hookSource.includes('useAdminWalletAddresses'), 'hook must compose useAdminWalletAddresses')
assert.ok(hookSource.includes('useAdminWalletUtxos'), 'hook must compose useAdminWalletUtxos')
assert.ok(hookSource.includes('composeAddressesWithBalance'), 'hook must use composeAddressesWithBalance')
console.log('useAddressesWithBalance: composition contract OK')

console.log('All useAddressesWithBalance contract tests passed.')
