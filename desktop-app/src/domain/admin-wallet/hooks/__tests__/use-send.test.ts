// useSend — pure TypeScript contract tests (Phase 6, PRD §4.3.5).
//
// SCOPE: export surface, type shapes, and source-level composition contracts using
// the project's tsx test runner (React hook behaviour needs vitest + @testing-library/react,
// which are not installed — same constraint as use-unconfirmed-txs.test.ts).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

// ── 1. Export surface ────────────────────────────────────────────────────────

import { useSend } from '../use-send.ts'
import type { SendState } from '../use-send.ts'

assert.equal(typeof useSend, 'function', 'useSend must be exported as a function')
console.log('useSend: export surface OK')

// ── 2. SendState type contract ───────────────────────────────────────────────

const _idle: SendState = { status: 'idle' }
const _submitting: SendState = { status: 'submitting' }
const _success: SendState = {
	status: 'success',
	result: {
		txid: 'ab',
		amountSats: 30_000,
		feeSats: 705,
		feeRateSatPerKvb: 5_000,
		vsizeVbytes: 141,
		changeSats: 69_295,
		drained: false,
	},
}
const _error: SendState = { status: 'error', error: { type: 'InvalidAddress', message: 'x' } }
const _wrongNetwork: SendState = { status: 'error', error: { type: 'WrongNetwork', message: 'x' } }
const _invalidAmount: SendState = { status: 'error', error: { type: 'InvalidAmount', message: 'x' } }
const _belowDust: SendState = { status: 'error', error: { type: 'AmountBelowDust', message: 'x' } }
void [_idle, _submitting, _success, _error, _wrongNetwork, _invalidAmount, _belowDust]
console.log('SendState: all variants (incl. Phase 6 error codes) accepted OK')

// ── 3. Composition contracts ─────────────────────────────────────────────────

const __dirname = dirname(fileURLToPath(import.meta.url))
const sendSource = readFileSync(join(__dirname, '..', 'use-send.ts'), 'utf8')

assert.ok(!sendSource.includes('@tauri-apps/api/core'), 'send hook must not import Tauri directly')
assert.ok(sendSource.includes("from '@/api/admin-wallet'"), 'send hook must go through the typed API adapter')
assert.ok(sendSource.includes('sendFromAdminWallet'), 'send hook must call the admin_wallet_send adapter')
assert.ok(sendSource.includes('parseAdminWalletError'), 'send hook must surface typed AdminWalletError values')
console.log('useSend: composition contracts OK')

// ── 4. Reject/retry contract (PRD §4.3.5.5.1) ───────────────────────────────
// The hook owns no field state — an error transition must not reset anything
// beyond its own machine, so the form retains values for retry or back-out.

assert.ok(!sendSource.includes('useEffect'), 'useSend must be a pure submit machine (no effects)')
assert.ok(
	!/setAddress|setAmount/.test(sendSource),
	'useSend must not own or reset form field state (reject keeps the form intact)',
)
console.log('useSend: reject/retry contract OK')

console.log('use-send.test.ts PASS')
