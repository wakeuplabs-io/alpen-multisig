// Phase-driven view predicates shared by the send-proposal and send-cancel screens.
//
// The expectation table is a Record keyed by BroadcastPhase, so adding a phase without deciding
// what each predicate returns for it is a compile error rather than an untested gap.

import assert from 'node:assert/strict'
import {
	isBroadcastDetailsPhase,
	isBroadcastInFlightPhase,
	isBroadcastLoadingPhase,
	isBroadcastProgressPhase,
} from '../broadcast-proposal.ts'
import type { BroadcastPhase } from '../broadcast-proposal.ts'

type Expected = { loading: boolean; details: boolean; progress: boolean; inFlight: boolean }

const table: Record<BroadcastPhase, Expected> = {
	idle: { loading: true, details: false, progress: false, inFlight: false },
	preparing: { loading: true, details: false, progress: false, inFlight: false },
	confirming: { loading: false, details: true, progress: false, inFlight: false },
	'awaiting-device': { loading: false, details: true, progress: true, inFlight: true },
	broadcasting: { loading: false, details: true, progress: true, inFlight: true },
	'awaiting-confirmation': { loading: false, details: false, progress: true, inFlight: false },
	done: { loading: false, details: false, progress: true, inFlight: false },
	error: { loading: false, details: false, progress: true, inFlight: false },
}

for (const [phase, expected] of Object.entries(table) as [BroadcastPhase, Expected][]) {
	assert.equal(isBroadcastLoadingPhase(phase), expected.loading, `loading(${phase})`)
	assert.equal(isBroadcastDetailsPhase(phase), expected.details, `details(${phase})`)
	assert.equal(isBroadcastProgressPhase(phase), expected.progress, `progress(${phase})`)
	assert.equal(isBroadcastInFlightPhase(phase), expected.inFlight, `inFlight(${phase})`)
}

// Issue #484: the cancel screen's local copy of this predicate omitted both phases, so the UI
// went blank between submit and confirmation, and a hardware signer got no device prompt.
assert.equal(isBroadcastProgressPhase('awaiting-device'), true, 'device approval must show progress')
assert.equal(isBroadcastProgressPhase('awaiting-confirmation'), true, 'confirmation polling must show progress')

console.log('broadcast phase predicates: all 8 phases OK')
