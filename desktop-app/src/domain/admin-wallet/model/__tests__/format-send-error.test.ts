// format-send-error — byte-exact PRD §4.3.5.1 copy tests (Phase 6 P6.2).
//
// The two destination error strings are PRD MUSTs. These assertions are
// intentionally byte-for-byte: any copy drift must fail CI.

import assert from 'node:assert/strict'
import {
	SEND_DESTINATION_UNAVAILABLE_COPY,
	SEND_INSUFFICIENT_FUNDS_COPY,
	formatSendDestinationError,
} from '../format-send-error.ts'

// PRD: "Destination must be a bitcoin address."
assert.equal(formatSendDestinationError('invalid-address', 'regtest'), 'Destination must be a bitcoin address.')
// The generic copy must not leak the network word.
assert.equal(formatSendDestinationError('invalid-address', 'mainnet'), 'Destination must be a bitcoin address.')

// PRD: "Destination must be a [correct network] bitcoin address."
assert.equal(formatSendDestinationError('wrong-network', 'regtest'), 'Destination must be a regtest bitcoin address.')
assert.equal(formatSendDestinationError('wrong-network', 'mainnet'), 'Destination must be a mainnet bitcoin address.')
assert.equal(formatSendDestinationError('wrong-network', 'testnet'), 'Destination must be a testnet bitcoin address.')

// Neutral IPC-unavailable copy exists and is not one of the PRD strings.
assert.ok(SEND_DESTINATION_UNAVAILABLE_COPY.length > 0)
assert.ok(!SEND_DESTINATION_UNAVAILABLE_COPY.startsWith('Destination must be'))

// PRD §4.3.5.2: the over-balance error is exactly "Insufficient funds".
assert.equal(SEND_INSUFFICIENT_FUNDS_COPY, 'Insufficient funds')

console.log('format-send-error.test.ts PASS (PRD §4.3.5.1 + §4.3.5.2 copy byte-exact)')
