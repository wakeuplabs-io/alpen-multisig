import { describe, it, expect } from 'vitest'
import { deriveBroadcastError } from '../broadcast-proposal'

describe('deriveBroadcastError', () => {
	describe('structured JSON parsing', () => {
		it('parses a structured error with code, message, and recovery', () => {
			const result = deriveBroadcastError(JSON.stringify({ code: 'device_disconnected', message: 'Hardware wallet not detected' }))
			expect(result.code).toBe('device_disconnected')
			expect(result.message).toBe('Hardware wallet not detected')
			expect(result.recovery).toBe('reconnect-device')
		})

		it('maps all error codes to correct recovery actions', () => {
			const mappings: Array<{ code: string; expected: string }> = [
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
			for (const { code, expected } of mappings) {
				const result = deriveBroadcastError(JSON.stringify({ code, message: 'test' }))
				expect(result.code).toBe(code)
				expect(result.recovery).toBe(expected)
			}
		})

		it('falls back to unknown_error for unrecognized codes', () => {
			const result = deriveBroadcastError(JSON.stringify({ code: 'weird_thing', message: 'huh' }))
			expect(result.code).toBe('unknown_error')
			expect(result.recovery).toBe('retry')
		})
	})

	describe('legacy string fallback', () => {
		it('returns unknown_error for bare error strings (backward compatible)', () => {
			const result = deriveBroadcastError('Fee rate too low')
			expect(result.code).toBe('unknown_error')
			expect(result.message).toBe('Fee rate too low')
			expect(result.recovery).toBe('retry')
		})
	})
})
