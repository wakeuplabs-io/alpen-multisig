// deriveBroadcastError — structured-error parsing + recovery mapping (DDD-8).
//
// Behaviors:
//   1. Structured JSON { code, message } parses and maps code → recovery.
//   2. All known codes map to the expected recovery action.
//   3. Unrecognized codes fall back to unknown_error / retry.
//   4. Legacy bare strings degrade to unknown_error / retry (backward compatible).

import assert from 'node:assert/strict'
import { deriveBroadcastError } from '../broadcast-proposal'

// ── 1. Structured JSON parsing ───────────────────────────────────────────────

const structured = deriveBroadcastError(
	JSON.stringify({ code: 'device_disconnected', message: 'Hardware wallet not detected' }),
)
assert.equal(structured.code, 'device_disconnected')
assert.equal(structured.message, 'Hardware wallet not detected')
assert.equal(structured.recovery, 'reconnect-device')
console.log('deriveBroadcastError: parses structured error OK')

// ── 2. Code → recovery mapping ───────────────────────────────────────────────

const mappings: Array<{ code: string; expected: string }> = [
	{ code: 'insufficient_fee', expected: 'retry' },
	{ code: 'mempool_rejected', expected: 'retry' },
	{ code: 'double_spend', expected: 'retry' },
	{ code: 'consensus_violation', expected: 'resubmit-reveal' },
	{ code: 'invalid_reveal', expected: 'resubmit-reveal' },
	{ code: 'orphan_commit', expected: 'resubmit-reveal' },
	{ code: 'device_disconnected', expected: 'reconnect-device' },
	{ code: 'session_expired', expected: 're-auth' },
	{ code: 'broadcast_unavailable', expected: 'manual-broadcast' },
	{ code: 'unknown_error', expected: 'retry' },
]
for (const { code, expected } of mappings) {
	const result = deriveBroadcastError(JSON.stringify({ code, message: 'test' }))
	assert.equal(result.code, code, `code ${code} preserved`)
	assert.equal(result.recovery, expected, `code ${code} → recovery ${expected}`)
}
console.log('deriveBroadcastError: all codes map to correct recovery OK')

// ── 3. Unrecognized code falls back ──────────────────────────────────────────

const unrecognized = deriveBroadcastError(JSON.stringify({ code: 'weird_thing', message: 'huh' }))
assert.equal(unrecognized.code, 'unknown_error')
assert.equal(unrecognized.recovery, 'retry')
console.log('deriveBroadcastError: unrecognized code falls back to unknown_error OK')

// ── 4. Legacy bare-string fallback ───────────────────────────────────────────

const legacy = deriveBroadcastError('Fee rate too low')
assert.equal(legacy.code, 'unknown_error')
assert.equal(legacy.message, 'Fee rate too low')
assert.equal(legacy.recovery, 'retry')
console.log('deriveBroadcastError: legacy string fallback OK')

// ── 5. broadcast_unavailable carries tx hexes ────────────────────────────────

const unavailable = deriveBroadcastError(
	JSON.stringify({
		code: 'broadcast_unavailable',
		message: 'All broadcast channels failed.',
		commitTxHex: 'deadbeef01',
		revealTxHex: 'deadbeef02',
	}),
)
assert.equal(unavailable.code, 'broadcast_unavailable')
assert.equal(unavailable.recovery, 'manual-broadcast')
assert.equal(unavailable.commitTxHex, 'deadbeef01')
assert.equal(unavailable.revealTxHex, 'deadbeef02')
console.log('deriveBroadcastError: broadcast_unavailable carries tx hexes OK')

console.log('All deriveBroadcastError tests passed.')
