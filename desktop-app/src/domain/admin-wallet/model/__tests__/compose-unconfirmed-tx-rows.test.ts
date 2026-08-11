// composeUnconfirmedTxRows — pure view-model mapping tests (Phase 5, PRD §4.3.3).

import assert from 'node:assert/strict'
import { composeUnconfirmedTxRows } from '../compose-unconfirmed-tx-rows.ts'
import type { UnconfirmedTxDto } from '@/api/admin-wallet'

function makeTx(overrides: Partial<UnconfirmedTxDto> = {}): UnconfirmedTxDto {
	return {
		txid: 'a'.repeat(64),
		sentSats: 100_000,
		receivedSats: 59_859,
		netSats: -40_141,
		feeSats: 141,
		feeRateSatPerKvb: 1_000,
		vsizeVbytes: 141,
		isRbfSignaling: true,
		isGovernanceCommit: false,
		bumpMethod: 'rbf',
		packageFeeSats: null,
		packageVsizeVbytes: null,
		packageFeeRateSatPerKvb: null,
		maxBumpRateSatPerKvb: null,
		lastSeenSecs: 1_700_000_000,
		...overrides,
	}
}

function makeGovernanceTx(overrides: Partial<UnconfirmedTxDto> = {}): UnconfirmedTxDto {
	return makeTx({
		isGovernanceCommit: true,
		bumpMethod: 'cpfp',
		packageFeeSats: 471,
		packageVsizeVbytes: 270,
		packageFeeRateSatPerKvb: 1_745,
		maxBumpRateSatPerKvb: 2_914_622,
		...overrides,
	})
}

// ── Happy path: labels and bump capability ───────────────────────────────────

{
	const [row] = composeUnconfirmedTxRows([makeTx()])
	assert.equal(row.txid, 'a'.repeat(64))
	assert.equal(row.shortTxid, `${'a'.repeat(8)}…${'a'.repeat(6)}`)
	assert.equal(row.netLabel, '−40,141 sats')
	assert.equal(row.feeLabel, '141 sats')
	assert.equal(row.feeRateLabel, '1.0 sat/vB')
	assert.equal(row.usesPackageStats, false)
	assert.equal(row.lastSeenIso, new Date(1_700_000_000 * 1000).toISOString())
	assert.equal(row.bumpMethod, 'rbf')
	assert.equal(row.canBump, true)
	assert.equal(row.bumpDisabledReason, null)
	assert.equal(row.currentFeeRateSatPerKvb, 1_000)
	assert.equal(row.currentFeeSats, 141)
	assert.equal(row.vsizeVbytes, 141)
	console.log('compose: happy path labels OK')
}

// ── canBump / bumpMethod matrix ──────────────────────────────────────────────

{
	const [nonRbf] = composeUnconfirmedTxRows([makeTx({ isRbfSignaling: false, bumpMethod: null })])
	assert.equal(nonRbf.canBump, false)
	assert.equal(nonRbf.bumpDisabledReason, 'not-rbf')

	// Governance commits are bumpable via CPFP — even when not RBF-signaling.
	const [governance] = composeUnconfirmedTxRows([makeGovernanceTx({ isRbfSignaling: false })])
	assert.equal(governance.canBump, true)
	assert.equal(governance.bumpMethod, 'cpfp')
	assert.equal(governance.bumpDisabledReason, null)
	assert.equal(governance.isGovernanceCommit, true)
	console.log('compose: canBump/bumpMethod matrix OK')
}

// ── Governance rows display package fee/rate and expose package floor ────────

{
	const [row] = composeUnconfirmedTxRows([makeGovernanceTx()])
	assert.equal(row.usesPackageStats, true)
	assert.equal(row.feeLabel, '471 sats')
	assert.equal(row.feeRateLabel, '1.7 sat/vB')
	assert.equal(row.currentFeeRateSatPerKvb, 1_745)
	assert.equal(row.currentFeeSats, 471)
	assert.equal(row.vsizeVbytes, 270)
	console.log('compose: governance package stats OK')
}

// ── Governance row without package stats falls back to the commit's own ──────

{
	const [row] = composeUnconfirmedTxRows([
		makeGovernanceTx({ packageFeeSats: null, packageVsizeVbytes: null, packageFeeRateSatPerKvb: null }),
	])
	assert.equal(row.usesPackageStats, false)
	assert.equal(row.feeLabel, '141 sats')
	assert.equal(row.feeRateLabel, '1.0 sat/vB')
	assert.equal(row.currentFeeRateSatPerKvb, 1_000)
	assert.equal(row.vsizeVbytes, 141)
	// F-009 (PR #290) turned this around: without package stats the reveal is not in the
	// graph yet, so the bump is held back until a sync loads it rather than failing later.
	assert.equal(row.canBump, false, 'a CPFP row cannot be bumped until its package stats load')
	assert.equal(row.bumpDisabledReason, 'cpfp-stats-unavailable')
	console.log('compose: governance fallback OK')
}

// ── Unknown fee (foreign prevout) ────────────────────────────────────────────

{
	const [row] = composeUnconfirmedTxRows([makeTx({ feeSats: null, feeRateSatPerKvb: null })])
	assert.equal(row.feeLabel, 'Unknown')
	assert.equal(row.feeRateLabel, '—')
	assert.equal(row.currentFeeRateSatPerKvb, null)
	assert.equal(row.currentFeeSats, null)
	console.log('compose: unknown fee OK')
}

// ── Missing last-seen ────────────────────────────────────────────────────────

{
	const [row] = composeUnconfirmedTxRows([makeTx({ lastSeenSecs: null })])
	assert.equal(row.lastSeenIso, null)
	console.log('compose: missing last-seen OK')
}

// ── Order preserved (backend sorts newest-first) ─────────────────────────────

{
	const rows = composeUnconfirmedTxRows([makeTx({ txid: 'b'.repeat(64) }), makeTx({ txid: 'c'.repeat(64) })])
	assert.equal(rows[0].txid, 'b'.repeat(64))
	assert.equal(rows[1].txid, 'c'.repeat(64))
	console.log('compose: order preserved OK')
}

// ── #431: the per-row bump ceiling reaches the form ──────────────────────────

{
	const [cpfp] = composeUnconfirmedTxRows([makeGovernanceTx()])
	assert.equal(cpfp.maxBumpRateSatPerKvb, 2_914_622, 'a CPFP row carries the ceiling its child can honour')

	const [rbf] = composeUnconfirmedTxRows([makeTx()])
	assert.equal(rbf.maxBumpRateSatPerKvb, null, 'an RBF row keeps the general ceiling')
	console.log('compose: per-row bump ceiling OK')
}

console.log('compose-unconfirmed-tx-rows: all tests passed.')
