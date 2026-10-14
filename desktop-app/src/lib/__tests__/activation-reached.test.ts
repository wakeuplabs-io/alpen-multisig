// activation-reached — a proposal past its activation block is re-read, and the countdown stops
// saying "imminent" about a block the ASM has already processed (#602).
//
// The tip is polled every 15 s; the proposal list is not. Without this cue the card kept its
// pre-activation snapshot ("Awaiting enactment") while the current block moved past the activation
// block, and the backend — which promotes to Enacted on read — was never asked again.

import assert from 'node:assert/strict'
import { isActivationReached } from '../proposal-status.ts'
import type { BroadcastStatus, ProposalStatus } from '../../api/proposals.ts'
import { ACTIVATION_REACHED_LABEL, activationTimeLabel } from '../activation-time-label.ts'

function proposal(
	activationHeight: number | null,
	status: ProposalStatus = 'approved',
	broadcastStatus: BroadcastStatus = 'reveal_confirmed',
) {
	return { status, broadcastStatus, activationHeight }
}

// ── Reached at the activation block and after it, not before ───────────────
{
	assert.equal(isActivationReached(proposal(133), 132), false)
	assert.equal(isActivationReached(proposal(133), 133), true)
	assert.equal(isActivationReached(proposal(133), 134), true)
}

// ── Only a proposal that is still awaiting enactment ───────────────────────
{
	assert.equal(isActivationReached(proposal(133, 'enacted'), 134), false)
	assert.equal(isActivationReached(proposal(133, 'canceled'), 134), false)
	assert.equal(isActivationReached(proposal(133, 'approved', 'reveal_broadcasted'), 134), false)
}

// ── Unknown height or tip never triggers a read ────────────────────────────
{
	assert.equal(isActivationReached(proposal(null), 134), false)
	assert.equal(isActivationReached(proposal(133), null), false)
}

// ── The countdown label: time while ahead, never "imminent" once reached ───
{
	assert.equal(activationTimeLabel(133, null), null)
	assert.equal(activationTimeLabel(133, undefined), null)
	assert.equal(activationTimeLabel(133, 127), '~1h 0m')
	assert.equal(activationTimeLabel(133, 133), ACTIVATION_REACHED_LABEL)
	assert.equal(activationTimeLabel(133, 134), ACTIVATION_REACHED_LABEL)
	assert.ok(!ACTIVATION_REACHED_LABEL.includes('imminent'))
}

console.log('activation-reached: all assertions passed')
