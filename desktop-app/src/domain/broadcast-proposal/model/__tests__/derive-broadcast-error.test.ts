import assert from 'node:assert/strict'
import { deriveBroadcastError } from '../broadcast-proposal.ts'

// Structured JSON parsing
const insufficientFee = deriveBroadcastError(JSON.stringify({ code: 'insufficient_fee', message: 'Fee rate too low' }))
assert.equal(insufficientFee.code, 'insufficient_fee')
assert.equal(insufficientFee.message, 'Fee rate too low')
assert.equal(insufficientFee.recovery, 'retry')

// All code → recovery mappings (single iteration, reports all at once)
const allMappings: Array<{ code: string; expected: string }> = [
	{ code: 'insufficient_fee', expected: 'retry' },
	{ code: 'mempool_rejected', expected: 'retry' },
	{ code: 'double_spend', expected: 'retry' },
	{ code: 'consensus_violation', expected: 'resubmit-reveal' },
	{ code: 'invalid_reveal', expected: 'resubmit-reveal' },
	{ code: 'orphan_commit', expected: 'resubmit-reveal' },
	{ code: 'device_disconnected', expected: 'reconnect-device' },
	{ code: 'session_expired', expected: 're-auth' },
	{ code: 'unknown_error', expected: 'retry' },
]
for (const { code, expected } of allMappings) {
	const result = deriveBroadcastError(JSON.stringify({ code, message: 'test' }))
	assert.equal(result.code, code, `${code} should parse`)
	assert.equal(result.recovery, expected, `${code} → ${expected}`)
}

// Unknown code falls back to unknown_error
const unknownCode = deriveBroadcastError(JSON.stringify({ code: 'weird_thing', message: 'huh' }))
assert.equal(unknownCode.code, 'unknown_error')
assert.equal(unknownCode.recovery, 'retry')

// Unparseable input fallback
const garbage = deriveBroadcastError('not json at all')
assert.equal(garbage.code, 'unknown_error')
assert.equal(garbage.message, 'not json at all')
assert.equal(garbage.recovery, 'retry')

console.log('derive-broadcast-error: all assertions passed.')
