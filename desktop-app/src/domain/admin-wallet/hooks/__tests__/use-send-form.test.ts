// useSendForm — pure TypeScript contract tests (Phase 6 P6.2, PRD §4.3.5.1).
//
// SCOPE: export surface, type shapes, and source-level composition contracts
// using the project's tsx test runner (React hook behaviour needs vitest +
// @testing-library/react, which are not installed — same constraint as
// use-send.test.ts).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

// ── 1. Export surface ────────────────────────────────────────────────────────

import { useSendForm, ESTIMATE_DEBOUNCE_MS, VALIDATE_DEBOUNCE_MS } from '../use-send-form.ts'
import type { DestinationState, EstimateState } from '../use-send-form.ts'

assert.equal(typeof useSendForm, 'function', 'useSendForm must be exported as a function')
assert.equal(VALIDATE_DEBOUNCE_MS, 300, 'validate debounce window is 300 ms per spec §6.3')
assert.equal(ESTIMATE_DEBOUNCE_MS, 300, 'estimate debounce window is 300 ms per spec §6.3')
console.log('useSendForm: export surface OK')

// ── 2. DestinationState type contract ────────────────────────────────────────

const _empty: DestinationState = { status: 'empty' }
const _validating: DestinationState = { status: 'validating', address: 'a' }
const _valid: DestinationState = { status: 'valid', address: 'a' }
const _invalidAddr: DestinationState = {
	status: 'invalid',
	address: 'a',
	reason: 'invalid-address',
	expectedNetwork: 'regtest',
}
const _wrongNet: DestinationState = {
	status: 'invalid',
	address: 'a',
	reason: 'wrong-network',
	expectedNetwork: 'regtest',
}
const _unavailable: DestinationState = { status: 'unavailable', address: 'a' }
void [_empty, _validating, _valid, _invalidAddr, _wrongNet, _unavailable]
console.log('DestinationState: all variants accepted OK')

// ── 2b. EstimateState type contract (P6.3) ───────────────────────────────────

const _estIdle: EstimateState = { status: 'idle' }
const _estLoading: EstimateState = { status: 'loading' }
const _estReady: EstimateState = {
	status: 'ready',
	estimate: {
		amountSats: 30_000,
		feeSats: 705,
		feeRateSatPerKvb: 5_000,
		vsizeVbytes: 141,
		changeSats: 69_295,
		maxAmountSats: 99_295,
	},
}
const _estError: EstimateState = { status: 'error', error: { type: 'InsufficientFunds', message: 'x' } }
void [_estIdle, _estLoading, _estReady, _estError]
console.log('EstimateState: all variants accepted OK')

// ── 3. Composition contracts ─────────────────────────────────────────────────

const __dirname = dirname(fileURLToPath(import.meta.url))
const source = readFileSync(join(__dirname, '..', 'use-send-form.ts'), 'utf8')

assert.ok(!source.includes('@tauri-apps/api/core'), 'hook must not import Tauri directly')
assert.ok(source.includes('validateSendAddress'), 'hook must validate through the typed API adapter')
assert.ok(source.includes('clearTimeout'), 'debounce timer must be cleared on keystroke/unmount')
assert.ok(source.includes('validateSeq'), 'stale validation responses must be discarded via a sequence guard')
assert.ok(
	source.includes("setDestination({ status: 'unavailable'"),
	'a failing validation IPC must block Confirm via the unavailable state — never silently valid',
)

// P6.3 — estimate dry-run + Max semantics.
assert.ok(source.includes('estimateSend'), 'hook must estimate through the typed API adapter')
assert.ok(source.includes('estimateSeq'), 'stale estimate responses must be discarded via a sequence guard')
assert.ok(source.includes("preset: 'fast'"), 'fee selection must default to Fast — PRD §4.3.5.3 next-block rate')
assert.ok(source.includes('drainWallet: true'), 'Max mode must estimate/submit as a drain build')
assert.ok(
	/setAddressState\(raw\)\s*\n\s*setIsMax\(false\)/.test(source) &&
		/setAmountInputState\(raw\)\s*\n\s*setIsMax\(false\)/.test(source),
	'manual edits to destination or amount must leave Max mode',
)
console.log('useSendForm: composition contracts OK')

console.log('use-send-form.test.ts PASS')
