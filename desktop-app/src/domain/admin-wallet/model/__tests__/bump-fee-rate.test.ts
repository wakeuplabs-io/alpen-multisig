// bump-fee-rate helpers — pure validation tests (Phase 5, PRD §4.3.3).

import assert from 'node:assert/strict'
import { isValidBumpRate, minBumpRateSatPerKvb, suggestedBumpRateSatPerKvb } from '../bump-fee-rate.ts'

// ── minBumpRateSatPerKvb ─────────────────────────────────────────────────────

assert.equal(minBumpRateSatPerKvb(1_000, 1_000), 1_100, 'one 0.1 sat/vB step above current')
assert.equal(minBumpRateSatPerKvb(null, 1_000), 1_000, 'unknown current falls back to min relay')
console.log('minBumpRateSatPerKvb OK')

// ── suggestedBumpRateSatPerKvb ───────────────────────────────────────────────

assert.equal(suggestedBumpRateSatPerKvb(5_000, 1_100), 5_000, 'fast preset above min wins')
assert.equal(suggestedBumpRateSatPerKvb(1_000, 1_100), 1_100, 'min bump wins over a too-low preset')
assert.equal(suggestedBumpRateSatPerKvb(null, 1_100), 1_100, 'missing presets fall back to min bump')
console.log('suggestedBumpRateSatPerKvb OK')

// ── isValidBumpRate ──────────────────────────────────────────────────────────

assert.equal(isValidBumpRate(1_100, 1_100, 10_000_000), true, 'at min is valid')
assert.equal(isValidBumpRate(10_000_000, 1_100, 10_000_000), true, 'at max is valid')
assert.equal(isValidBumpRate(1_000, 1_100, 10_000_000), false, 'below min is invalid')
assert.equal(isValidBumpRate(10_000_100, 1_100, 10_000_000), false, 'above max is invalid')
assert.equal(isValidBumpRate(null, 1_100, 10_000_000), false, 'unparseable input is invalid')
console.log('isValidBumpRate OK')

console.log('bump-fee-rate: all tests passed.')
