// proposal-display-status — Defcon 1 never reads "Approved" (AC 9), and Defcon 3's queued
// lifecycle reads correctly (AC 7 / AC 10 of docs/specs/security-council-defcon-3.md).
//
// PRD 06 §5.2.2 carves Defcon 1 out of the Approved/Canceled lifecycle. The backend still moves it
// to `approved` at quorum, so the carve-out is a display rule, and it has to hold for every one of
// every state it can hold — not only the one the screens happen to render first. `superseded`
// arrived after this test did, which is exactly the way a carve-out like this goes stale.
//
// Both predicates below are keyed on the action, not on the authority: Defcon 1 and Defcon 3 share
// one authority, but Defcon 3 carries a real, configurable delay and a real Approved state. The way
// this regresses is somebody "simplifying" a predicate back onto the shared authority — and no
// assertion in this file before Defcon 3 got its own would have noticed.

import assert from 'node:assert/strict'
import { PROPOSAL_STATUS_STYLE, proposalDisplayStatus, showsActivationCountdown } from '../proposal-status.ts'
import type { ActionType, BroadcastStatus, ProposalStatus } from '../../api/proposals.ts'

function proposal(status: ProposalStatus, actionType: ActionType, broadcastStatus: BroadcastStatus = 'idle') {
	return { status, actionType, broadcastStatus }
}

// ── AC 9: no Defcon 1 state renders the word "Approved" ─────────────────────
{
	const states: ProposalStatus[] = ['pending', 'approved', 'enacted', 'expired', 'canceled', 'superseded']
	for (const status of states) {
		const display = proposalDisplayStatus(proposal(status, 'defcon_1'))
		const label = PROPOSAL_STATUS_STYLE[display].label
		assert.ok(!label.includes('Approved'), `defcon_1 in ${status} must not render "Approved", got "${label}"`)
	}

	assert.equal(proposalDisplayStatus(proposal('approved', 'defcon_1')), 'quorum_reached')
	assert.equal(PROPOSAL_STATUS_STYLE.quorum_reached.label, 'Quorum reached')
}

// ── The carve-out is Defcon 1's alone, and `awaiting_enactment` still wins ───
// A regression here is visible on every card in the app, not only the council's.
{
	assert.equal(proposalDisplayStatus(proposal('approved', 'multisig_update')), 'approved')
	assert.equal(proposalDisplayStatus(proposal('pending', 'defcon_1')), 'pending')

	const revealConfirmed: BroadcastStatus = 'reveal_confirmed'
	assert.equal(proposalDisplayStatus(proposal('approved', 'defcon_1', revealConfirmed)), 'awaiting_enactment')
	assert.equal(proposalDisplayStatus(proposal('approved', 'vk_update', revealConfirmed)), 'awaiting_enactment')
}

// ── A depth-0 action counts down to nothing, so it counts down to nothing ────
{
	assert.equal(showsActivationCountdown({ status: 'approved', activationHeight: 101, actionType: 'defcon_1' }), false)
	assert.equal(
		showsActivationCountdown({ status: 'approved', activationHeight: 101, actionType: 'multisig_update' }),
		true,
	)
	assert.equal(showsActivationCountdown({ status: 'approved', activationHeight: null, actionType: 'vk_update' }), false)
	assert.equal(showsActivationCountdown({ status: 'pending', activationHeight: 101, actionType: 'vk_update' }), false)
}

// ── A queued Defcon 3 counts down like any other lever, and degrades the same ─
{
	assert.equal(showsActivationCountdown({ status: 'approved', activationHeight: 101, actionType: 'defcon_3' }), true)
	assert.equal(showsActivationCountdown({ status: 'approved', activationHeight: null, actionType: 'defcon_3' }), false)
	assert.equal(showsActivationCountdown({ status: 'pending', activationHeight: 101, actionType: 'defcon_3' }), false)
}

// ── An approved Defcon 3 reads Approved, never Quorum reached ───────────────
// Defcon 1's carve-out must not spread to the lever that has a real Approved state.
{
	assert.equal(proposalDisplayStatus(proposal('approved', 'defcon_3')), 'approved')
	assert.equal(PROPOSAL_STATUS_STYLE.approved.label, 'Approved')
}

// ── Once the reveal confirms, a queued Defcon 3 reads Awaiting enactment ────
{
	const revealConfirmed: BroadcastStatus = 'reveal_confirmed'
	assert.equal(proposalDisplayStatus(proposal('approved', 'defcon_3', revealConfirmed)), 'awaiting_enactment')
}

console.log('proposal-display-status: all assertions passed.')
