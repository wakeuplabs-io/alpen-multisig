// additional-pair — a signer who did not send the pair in flight can send their own (#605).
//
// A low-fee bundle can sit in the mempool while nobody else can advance it. The offer appears
// only beside a pair that is on its way, and only to a signer who has not sent one; the
// orchestrator refuses the same cases (`decide_additional_pair`).

import assert from 'node:assert/strict'
import type { BroadcastStatus, ProposalStatus } from '../../api/proposals'
import { canSendAdditionalPair, hasPairInFlight } from '../proposal-send-state'

const ALICE = '02' + 'a'.repeat(64)
const BOB = '03' + 'b'.repeat(64)
const REVEAL = 'f'.repeat(64)

function row(
	broadcastStatus: BroadcastStatus,
	senders: Array<string | null> = [ALICE],
	status: ProposalStatus = 'approved',
	revealTxid: string | undefined = REVEAL,
) {
	return {
		status,
		broadcastStatus,
		revealTxid,
		broadcastAttempts: senders.map((broadcasterPubkey) => ({ broadcasterPubkey })),
	}
}

// ── Offered beside a pair on its way, to someone who has not sent one ──
for (const status of ['commit_broadcasted', 'commit_confirmed', 'reveal_broadcasted'] as const) {
	assert.equal(hasPairInFlight(row(status)), true, `${status} is in flight`)
	assert.equal(canSendAdditionalPair(row(status), BOB), true, `Bob may send beside Alice at ${status}`)
}

// ── Not to whoever already sent one: they bump its fee instead ──
assert.equal(canSendAdditionalPair(row('reveal_broadcasted'), ALICE), false)
assert.equal(
	canSendAdditionalPair(row('reveal_broadcasted'), ALICE.toUpperCase()),
	false,
	'the pubkey compares case-insensitively',
)
assert.equal(canSendAdditionalPair(row('reveal_broadcasted', [ALICE, BOB]), BOB), false, 'one pair per signer')

// A pair registered before the backend kept track names nobody, so it does not lock anyone out.
assert.equal(canSendAdditionalPair(row('reveal_broadcasted', [null]), BOB), true)

// ── Nothing on its way, nothing to send beside ──
const emptyClaim = { ...row('commit_broadcasted', []), revealTxid: undefined }
assert.equal(canSendAdditionalPair(emptyClaim, BOB), false, 'an empty claim is a signer still at the device')
for (const status of ['idle', 'failed', 'reveal_confirmed'] as const) {
	assert.equal(canSendAdditionalPair(row(status), BOB), false, `${status} offers no second pair`)
}
assert.equal(canSendAdditionalPair(row('reveal_broadcasted', [ALICE], 'superseded'), BOB), false)
assert.equal(canSendAdditionalPair(row('reveal_broadcasted'), null), false, 'no signer, no offer')

// ── This desktop's stored bundle decides for a signer who sent one before ──
assert.equal(canSendAdditionalPair(row('reveal_broadcasted'), ALICE, false), true, 'her pair is no longer on its way')
assert.equal(canSendAdditionalPair(row('reveal_broadcasted'), ALICE, true), false, 'her bundle is still tracked')
assert.equal(canSendAdditionalPair(row('reveal_broadcasted'), BOB, true), false, 'a bundle of his own is on its way')
assert.equal(canSendAdditionalPair(row('reveal_broadcasted'), ALICE, null), false, 'unknown: as if still tracked')

// ── An orchestrator without the registration endpoint sends no list: nothing is offered ──
assert.equal(canSendAdditionalPair({ ...row('reveal_broadcasted'), broadcastAttempts: null }, BOB), false)
assert.equal(canSendAdditionalPair({ ...row('reveal_broadcasted'), broadcastAttempts: undefined }, BOB), false)

console.log('additional-pair: all assertions passed')
