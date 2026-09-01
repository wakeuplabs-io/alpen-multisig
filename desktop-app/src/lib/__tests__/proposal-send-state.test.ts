// proposal-send-state — the Send button follows the transaction state (#432).
//
// The reported bug: after the bundle was broadcast the status read "Approved"
// and a "Send" button was still offered, so a signer could not tell whether the
// commit+reveal bundle still needed sending. What is pinned here is that the
// button appears in exactly two states — nothing sent yet, and a failed send —
// and that every other stage says where the bundle actually is.

import assert from 'node:assert/strict'
import type { BroadcastStatus, ProposalStatus } from '../../api/proposals'
import { proposalSendState, showsSendButton, sendButtonLabel } from '../proposal-send-state'

function proposal(status: ProposalStatus, broadcastStatus: BroadcastStatus, signatures = 2, required = 2) {
	return { status, broadcastStatus, requiredSignatures: required, signatures: Array(signatures).fill({}) }
}

// ── The button shows only where the backend would accept a broadcast ──

const ready = proposalSendState(proposal('approved', 'idle'))
assert.equal(ready.kind, 'ready')
assert.equal(showsSendButton(ready), true)
assert.equal(sendButtonLabel(ready), 'Send')

// A failed broadcast is recoverable: the repo accepts a re-broadcast from
// `Idle | Failed`. Hiding the button here would strand the user.
const failed = proposalSendState(proposal('approved', 'failed'))
assert.equal(failed.kind, 'failed')
assert.equal(showsSendButton(failed), true)
assert.equal(sendButtonLabel(failed), 'Retry send')

// ── Every in-flight stage hides the button and names the leg ──

for (const stage of ['commit_broadcasted', 'commit_confirmed', 'reveal_broadcasted'] as const) {
	const state = proposalSendState(proposal('approved', stage))
	assert.equal(state.kind, 'in-flight', `${stage} must be in-flight`)
	assert.equal(showsSendButton(state), false, `${stage} must not offer Send`)
	assert.ok(state.kind === 'in-flight' && state.label.length > 0, `${stage} must carry a label`)
}

// The commit and reveal legs must be distinguishable — "how can I tell when the
// commit+reveal bundle is confirmed" is the question the issue asks.
const commit = proposalSendState(proposal('approved', 'commit_broadcasted'))
const reveal = proposalSendState(proposal('approved', 'reveal_broadcasted'))
assert.notEqual(
	commit.kind === 'in-flight' ? commit.label : '',
	reveal.kind === 'in-flight' ? reveal.label : '',
	'commit and reveal stages must read differently',
)

// ── Confirmed: nothing left to send ──

const confirmed = proposalSendState(proposal('approved', 'reveal_confirmed'))
assert.equal(confirmed.kind, 'confirmed')
assert.equal(showsSendButton(confirmed), false)
assert.match(
	confirmed.kind === 'confirmed' ? confirmed.detail : '',
	/nothing left to send/i,
	'the confirmed state must say the bundle no longer needs sending',
)

// ── Nothing to send outside the approved window ──

// Quorum on a still-pending proposal is not enough: the bundle does not exist
// until the backend approves it.
assert.equal(proposalSendState(proposal('pending', 'idle')).kind, 'unavailable')
assert.equal(proposalSendState(proposal('pending', 'idle', 0)).kind, 'unavailable')

for (const terminal of ['enacted', 'canceled', 'expired'] as const) {
	assert.equal(
		proposalSendState(proposal(terminal, 'idle')).kind,
		'unavailable',
		`${terminal} proposals must not offer Send`,
	)
	// A terminal proposal that was broadcast must not resurrect the button either.
	assert.equal(proposalSendState(proposal(terminal, 'reveal_confirmed')).kind, 'unavailable')
}

// ── Superseded: terminal, and it says which of the two ways it got there ──
//
// The chain used this proposal's sequence number for another action, so the ASM will refuse its
// transaction from here on. Two ways to arrive, and they are not the same thing to the person
// reading: a bundle whose reveal was mined reached a block and lost the race — the commit and
// reveal fees were spent — while one that never confirmed never got that far.

const supersededAfter = proposalSendState(proposal('superseded', 'reveal_confirmed'))
assert.equal(supersededAfter.kind, 'superseded')
assert.equal(showsSendButton(supersededAfter), false)
assert.match(
	supersededAfter.kind === 'superseded' ? supersededAfter.detail : '',
	/fees were spent/i,
	'a superseded bundle that was mined must say the fees were spent',
)

const supersededBefore = proposalSendState(proposal('superseded', 'idle'))
assert.equal(supersededBefore.kind, 'superseded')
assert.equal(showsSendButton(supersededBefore), false)
assert.doesNotMatch(
	supersededBefore.kind === 'superseded' ? supersededBefore.detail : '',
	/fees were spent/i,
	'a bundle that never confirmed spent no reveal fee',
)

// The label is the same either way — only the detail differs.
assert.equal(
	supersededAfter.kind === 'superseded' && supersededBefore.kind === 'superseded'
		? supersededAfter.label === supersededBefore.label
		: false,
	true,
)

// Quorum is irrelevant once the sequence number is gone: a superseded proposal that never reached
// quorum is just as dead, and must not fall through to `unavailable`.
assert.equal(proposalSendState(proposal('superseded', 'idle', 0)).kind, 'superseded')

console.log('proposal-send-state: all assertions passed')
