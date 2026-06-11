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
		lastSeenSecs: 1_700_000_000,
		...overrides,
	}
}

// ── Happy path: labels and bump capability ───────────────────────────────────

{
	const [row] = composeUnconfirmedTxRows([makeTx()])
	assert.equal(row.txid, 'a'.repeat(64))
	assert.equal(row.shortTxid, `${'a'.repeat(8)}…${'a'.repeat(6)}`)
	assert.equal(row.netLabel, '−40,141 sats')
	assert.equal(row.feeLabel, '141 sats')
	assert.equal(row.feeRateLabel, '1.0 sat/vB')
	assert.equal(row.lastSeenIso, new Date(1_700_000_000 * 1000).toISOString())
	assert.equal(row.canBump, true)
	assert.equal(row.bumpDisabledReason, null)
	assert.equal(row.currentFeeRateSatPerKvb, 1_000)
	assert.equal(row.vsizeVbytes, 141)
	console.log('compose: happy path labels OK')
}

// ── canBump matrix ───────────────────────────────────────────────────────────

{
	const [nonRbf] = composeUnconfirmedTxRows([makeTx({ isRbfSignaling: false })])
	assert.equal(nonRbf.canBump, false)
	assert.equal(nonRbf.bumpDisabledReason, 'not-rbf')

	const [governance] = composeUnconfirmedTxRows([makeTx({ isGovernanceCommit: true })])
	assert.equal(governance.canBump, false)
	assert.equal(governance.bumpDisabledReason, 'governance-commit')
	assert.equal(governance.isGovernanceCommit, true)

	// Governance wins over not-rbf when both apply.
	const [both] = composeUnconfirmedTxRows([makeTx({ isGovernanceCommit: true, isRbfSignaling: false })])
	assert.equal(both.bumpDisabledReason, 'governance-commit')
	console.log('compose: canBump matrix OK')
}

// ── Unknown fee (foreign prevout) ────────────────────────────────────────────

{
	const [row] = composeUnconfirmedTxRows([makeTx({ feeSats: null, feeRateSatPerKvb: null })])
	assert.equal(row.feeLabel, 'Unknown')
	assert.equal(row.feeRateLabel, '—')
	assert.equal(row.currentFeeRateSatPerKvb, null)
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

console.log('compose-unconfirmed-tx-rows: all tests passed.')
