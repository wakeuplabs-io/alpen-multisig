// broadcast-pairs — the detail screen lists every commit + reveal pair sent (#605).

import assert from 'node:assert/strict'
import type { BroadcastAttempt } from '../../../../api/proposals'
import { describeBroadcastPairs } from '../broadcast-pairs'

const ALICE = '02' + 'a'.repeat(64)
const BOB = '03' + 'b'.repeat(64)

function attempt(broadcasterPubkey: string | null, tag: string): BroadcastAttempt {
	return { broadcasterPubkey, commitTxid: tag.repeat(64), revealTxid: tag.toUpperCase().repeat(64), createdAtMs: 0 }
}

const pairs = [attempt(ALICE, 'a'), attempt(BOB, 'b'), attempt(null, 'c')]

// In flight: the row tracks Bob's pair; the others are still out there.
const inFlight = describeBroadcastPairs(
	{ broadcastStatus: 'reveal_broadcasted', revealTxid: 'b'.repeat(64), broadcastAttempts: pairs },
	BOB,
)
assert.deepEqual(
	inFlight.map((p) => [p.sender, p.state]),
	[
		[{ kind: 'signer', pubkey: ALICE }, 'Sent'],
		[{ kind: 'you' }, 'Current'],
		[{ kind: 'unknown' }, 'Sent'],
	],
	'oldest first; the tracked pair is matched case-insensitively; another signer is carried by key, for its Admin ID',
)

// Mined: the row now tracks the winner, and the rest were not used.
const mined = describeBroadcastPairs(
	{ broadcastStatus: 'reveal_confirmed', revealTxid: 'A'.repeat(64), broadcastAttempts: pairs },
	BOB,
)
assert.deepEqual(
	mined.map((p) => p.state),
	['Mined', 'Not used', 'Not used'],
)

assert.deepEqual(describeBroadcastPairs({ broadcastStatus: 'idle' }, null), [], 'no pairs on a local proposal')

console.log('broadcast-pairs: all assertions passed')
