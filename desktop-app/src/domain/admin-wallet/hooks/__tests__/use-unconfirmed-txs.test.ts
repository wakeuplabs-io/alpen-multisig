// useUnconfirmedTxs / useBumpFee — pure TypeScript contract tests (Phase 5, PRD §4.3.3).
//
// SCOPE: export surface, type shapes, and source-level composition contracts using
// the project's tsx test runner (React hook behaviour needs vitest + @testing-library/react,
// which are not installed — same constraint as use-addresses-with-balance.test.ts).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

// ── 1. Export surface ────────────────────────────────────────────────────────

import { useUnconfirmedTxs } from '../use-unconfirmed-txs.ts'
import { useBumpFee } from '../use-bump-fee.ts'
import type { BumpFeeState } from '../use-bump-fee.ts'

assert.equal(typeof useUnconfirmedTxs, 'function', 'useUnconfirmedTxs must be exported as a function')
assert.equal(typeof useBumpFee, 'function', 'useBumpFee must be exported as a function')
console.log('hooks: export surface OK')

// ── 2. BumpFeeState type contract ────────────────────────────────────────────

const _idle: BumpFeeState = { status: 'idle' }
const _submitting: BumpFeeState = { status: 'submitting' }
const _success: BumpFeeState = {
	status: 'success',
	result: { newTxid: 'a', targetTxid: 'b', feeSats: 705, feeRateSatPerKvb: 5_000, method: 'cpfp' },
}
const _error: BumpFeeState = { status: 'error', error: { type: 'TxNotReplaceable', message: 'x' } }
void [_idle, _submitting, _success, _error]
console.log('BumpFeeState: all variants accepted OK')

// ── 3. Composition contracts ─────────────────────────────────────────────────

const __dirname = dirname(fileURLToPath(import.meta.url))
const listSource = readFileSync(join(__dirname, '..', 'use-unconfirmed-txs.ts'), 'utf8')
const bumpSource = readFileSync(join(__dirname, '..', 'use-bump-fee.ts'), 'utf8')

assert.ok(!listSource.includes('@tauri-apps/api/core'), 'list hook must not import Tauri directly')
assert.ok(!bumpSource.includes('@tauri-apps/api/core'), 'bump hook must not import Tauri directly')
assert.ok(listSource.includes('composeUnconfirmedTxRows'), 'list hook must map DTOs through the view-model')
assert.ok(listSource.includes('parseAdminWalletError'), 'list hook must parse tagged errors')
assert.ok(bumpSource.includes('parseAdminWalletError'), 'bump hook must parse tagged errors')
assert.ok(
	listSource.includes('useCallback(() => setTick'),
	'list refresh must be a stable useCallback (panel on-open effect dependency rule)',
)
console.log('hooks: composition contracts OK')

console.log('use-unconfirmed-txs/use-bump-fee: all tests passed.')
