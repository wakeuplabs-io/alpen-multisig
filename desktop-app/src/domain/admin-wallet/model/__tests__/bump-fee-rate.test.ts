// bump-fee-rate helpers — pure validation tests (Phase 5, PRD §4.3.3).

import assert from 'node:assert/strict'
import {
	effectiveMaxBumpRate,
	isValidBumpRate,
	minBumpRateSatPerKvb,
	suggestedBumpRateSatPerKvb,
} from '../bump-fee-rate.ts'

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

// ── effectiveMaxBumpRate (#431) ──────────────────────────────────────────────

assert.equal(effectiveMaxBumpRate(10_000_000, null), 10_000_000, 'an RBF row keeps the general ceiling')
assert.equal(
	effectiveMaxBumpRate(10_000_000, 3_255_002),
	3_255_002,
	"a CPFP row is capped by its child's ceiling, not the PRD one",
)
assert.equal(
	effectiveMaxBumpRate(2_000_000, 3_255_002),
	2_000_000,
	'a lower general ceiling still wins — the two are limits, not alternatives',
)
console.log('effectiveMaxBumpRate OK')

console.log('bump-fee-rate: all tests passed.')
