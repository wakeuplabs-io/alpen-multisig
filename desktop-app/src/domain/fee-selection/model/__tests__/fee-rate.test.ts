import assert from 'node:assert/strict'
import type { FeeRates } from '../../../../api/fee-rates.ts'
import { clampCustomRate, feeSats, formatSatPerVb, parseSatPerVb, resolveRate, totalFeeSats } from '../fee-rate.ts'

// ─── feeSats — golden vectors mirrored from Rust `FeeRate::fee_sats` ─────────

assert.equal(feeSats(1_000, 350), 350) // 1 sat/vB × 350 vB (exact)
assert.equal(feeSats(1_100, 350), 385) // 1_100 × 350 / 1000 = 385.0
assert.equal(feeSats(1_001, 350), 351) // 350.35 → ceil → 351
assert.equal(feeSats(100, 350), 35) // 0.1 sat/vB step

// ─── formatSatPerVb / parseSatPerVb round-trip ───────────────────────────────

assert.equal(formatSatPerVb(1_000), '1.0')
assert.equal(formatSatPerVb(1_100), '1.1')
assert.equal(formatSatPerVb(10_000_000), '10000.0')

assert.equal(parseSatPerVb('1.0'), 1_000)
assert.equal(parseSatPerVb('1.1'), 1_100)
assert.equal(parseSatPerVb('0.1'), 100)
assert.equal(parseSatPerVb('0'), null)
assert.equal(parseSatPerVb('-1'), null)
assert.equal(parseSatPerVb('abc'), null)
assert.equal(parseSatPerVb(''), null)

// ─── clampCustomRate / resolveRate / totalFeeSats ────────────────────────────

const presets: FeeRates = {
	fast: { satPerKvb: 1_200, targetBlocks: 1, marginPct: 20 },
	medium: { satPerKvb: 1_100, targetBlocks: 6, marginPct: 10 },
	slow: { satPerKvb: 1_100, targetBlocks: 12, marginPct: 5 },
	minRelaySatPerKvb: 1_000,
	maxSatPerKvb: 10_000_000,
	source: 'node',
	estimatedAtMs: 1_700_000_000_000,
	revealVbytes: 350,
	commitVbytesEstimate: 160,
	commitDustSats: 1_500,
}

assert.equal(clampCustomRate(500, presets), 1_000) // below min relay → raised
assert.equal(clampCustomRate(20_000_000, presets), 10_000_000) // above max → capped
assert.equal(clampCustomRate(5_000, presets), 5_000) // in range → unchanged

assert.equal(resolveRate({ kind: 'preset', preset: 'medium' }, presets), 1_100)
assert.equal(resolveRate({ kind: 'preset', preset: 'fast' }, presets), 1_200)
assert.equal(resolveRate({ kind: 'custom', satPerKvb: 4_200 }, presets), 4_200)

// reveal 350 vB + commit 160 vB at 1 sat/vB = 350 + 160
assert.equal(totalFeeSats(1_000, presets), 510)

console.log('fee-rate model: all assertions passed.')
