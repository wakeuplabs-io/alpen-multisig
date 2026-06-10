// useAdminWalletInfo — pure-logic contract tests (tsx runner; React rendering is
// BLOCKED_BY_DEPENDENCY — vitest + @testing-library/react not installed).
//
// The hook polls the cached wallet info so the broadcast funding card follows syncs
// triggered anywhere (wallet panel Refresh, background sync). What IS verified here:
//   1. Module exports resolve.
//   2. nextAdminWalletInfoState keeps the previous reference when nothing changed
//      (the 5s poll must not re-render consumers on every tick) and swaps it when
//      the balance or address moves.
//   3. The hook source polls via setInterval and clears it on cleanup.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { useAdminWalletInfo, nextAdminWalletInfoState } from '../use-admin-wallet-info.ts'

assert.equal(typeof useAdminWalletInfo, 'function', 'useAdminWalletInfo must be exported')
assert.equal(typeof nextAdminWalletInfoState, 'function', 'nextAdminWalletInfoState must be exported')
console.log('useAdminWalletInfo: exports OK')

// ── Reference-stable updates ──────────────────────────────────────────────────
const prev = { address: 'bcrt1p-a', balanceSats: 1000 }

assert.equal(
	nextAdminWalletInfoState(prev, { address: 'bcrt1p-a', balanceSats: 1000 }),
	prev,
	'unchanged info must keep the previous reference',
)
assert.notEqual(
	nextAdminWalletInfoState(prev, { address: 'bcrt1p-a', balanceSats: 2000 }),
	prev,
	'a balance change must produce a new reference',
)
assert.notEqual(
	nextAdminWalletInfoState(prev, { address: 'bcrt1p-b', balanceSats: 1000 }),
	prev,
	'an address rotation must produce a new reference',
)
const fromNull = nextAdminWalletInfoState(null, { address: 'bcrt1p-a', balanceSats: 0 })
assert.deepEqual(fromNull, { address: 'bcrt1p-a', balanceSats: 0 }, 'null previous state must adopt the fetched info')
const fromUndefined = nextAdminWalletInfoState(undefined, { address: 'bcrt1p-a', balanceSats: 0 })
assert.deepEqual(fromUndefined, { address: 'bcrt1p-a', balanceSats: 0 }, 'error state must recover to fetched info')
console.log('useAdminWalletInfo: reference-stable update OK')

// ── Polling contract ──────────────────────────────────────────────────────────
const __dirname = dirname(fileURLToPath(import.meta.url))
const hookSource = readFileSync(join(__dirname, '..', 'use-admin-wallet-info.ts'), 'utf8')

assert.ok(hookSource.includes('setInterval(fetchInfo'), 'hook must poll the wallet info on an interval')
assert.ok(hookSource.includes('clearInterval'), 'hook must clear the poll interval on cleanup')
console.log('useAdminWalletInfo: polling contract OK')

console.log('All useAdminWalletInfo contract tests passed.')
