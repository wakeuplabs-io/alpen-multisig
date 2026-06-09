// phaseForBroadcastStatus — pure-logic contract tests (node:assert, project tsx runner).

import assert from 'node:assert/strict'
import { phaseForBroadcastStatus, type BroadcastPhase } from '../broadcast-proposal.ts'

// ── reveal_confirmed → done ───────────────────────────────────────────────────
assert.equal(phaseForBroadcastStatus('reveal_confirmed'), 'done', 'reveal_confirmed → done')

// ── enacted proposal → done regardless of broadcast status ────────────────────
assert.equal(phaseForBroadcastStatus('reveal_broadcasted', 'enacted'), 'done', 'enacted → done')

// ── submitted-but-unconfirmed statuses → awaiting-confirmation ────────────────
for (const status of ['commit_broadcasted', 'commit_confirmed', 'reveal_broadcasted'] as const) {
	assert.equal(phaseForBroadcastStatus(status), 'awaiting-confirmation', `${status} → awaiting-confirmation`)
}

// ── idle / failed → null (do not change phase) ────────────────────────────────
assert.equal(phaseForBroadcastStatus('idle'), null, 'idle → null')
assert.equal(phaseForBroadcastStatus('failed'), null, 'failed → null')

// ── awaiting-confirmation is part of the BroadcastPhase union ─────────────────
const phase: BroadcastPhase = 'awaiting-confirmation'
assert.equal(phase, 'awaiting-confirmation', 'awaiting-confirmation is a valid BroadcastPhase')

console.log('phaseForBroadcastStatus: all OK')
