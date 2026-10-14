// proposal-read-gate — a slow, older read never lands over a newer one (#602 review).
//
// The visible load and the silent refresh write the same proposal state. Without a shared turn, a
// refresh issued before a reload could answer after it and put the older proposal back, and a
// load for the previous action id could overwrite the current one.

import assert from 'node:assert/strict'
import { createProposalReadGate } from '../proposal-read-gate.ts'

// ── A later load supersedes an earlier one (action id changed, role changed) ──
{
	const gate = createProposalReadGate()
	const first = gate.beginLoad()
	const second = gate.beginLoad()
	gate.settle(second)
	assert.equal(gate.isLatest(second), true)
	gate.settle(first)
	assert.equal(gate.isLatest(first), false, 'the earlier load answers last and must not write')
}

// ── A refresh issued before a reload must not land after it ──────────────
{
	const gate = createProposalReadGate()
	gate.settle(gate.beginLoad())
	const refresh = gate.beginRefresh()
	assert.notEqual(refresh, null)
	const reload = gate.beginLoad()
	gate.settle(reload)
	assert.equal(gate.isLatest(reload), true)
	assert.equal(gate.isLatest(refresh as number), false, 'the slower refresh must not overwrite the reload')
}

// ── No refresh while a load is in flight, so it can never strand the spinner ──
{
	const gate = createProposalReadGate()
	const load = gate.beginLoad()
	assert.equal(gate.beginRefresh(), null)
	gate.settle(load)
	assert.equal(gate.isLatest(load), true, 'the load still clears its own spinner')
	const refresh = gate.beginRefresh()
	assert.notEqual(refresh, null, 'refresh resumes once the load settles')
	assert.equal(gate.isLatest(refresh as number), true)
}

// ── A superseded load settling does not reopen the gate for refreshes ─────
{
	const gate = createProposalReadGate()
	const first = gate.beginLoad()
	gate.beginLoad()
	gate.settle(first)
	assert.equal(gate.beginRefresh(), null, 'the newer load is still in flight')
}

console.log('proposal-read-gate: all assertions passed')
