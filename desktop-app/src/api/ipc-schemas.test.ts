import assert from 'node:assert/strict'
import { proposalSchema } from './ipc-schemas.ts'

const proposalWithNullBroadcastFields = {
	actionId: 'test-action-id',
	seqNo: 1,
	authority: 'strata_admin',
	status: 'pending',
	requiredSignatures: 2,
	actionHex: '0x01',
	signatures: [],
	broadcastStatus: 'idle',
	commitTxid: null,
	revealTxid: null,
	broadcastError: null,
}

const parsed = proposalSchema.safeParse(proposalWithNullBroadcastFields)
assert.equal(parsed.success, true, `expected parse success, got: ${parsed.success ? '' : parsed.error.message}`)
if (parsed.success) {
	assert.equal(parsed.data.commitTxid, undefined)
	assert.equal(parsed.data.revealTxid, undefined)
	assert.equal(parsed.data.broadcastError, undefined)
}

import { authChallengeSchema, authSessionSchema } from './ipc-schemas.ts'

const challenge = {
	challengeId: 'c1',
	challengeHex: 'aa',
	nonceHex: 'bb',
	domain: 'alpen-multisig',
	role: 'strata_administrator',
	issuedAtUnixMs: 1,
	expiresAtUnixMs: 2,
	sessionId: 's1',
}
assert.equal(authChallengeSchema.safeParse(challenge).success, true)

const session = {
	role: 'strata_administrator',
	signerPubkeyHex: '02' + '00'.repeat(32),
	authenticatedAtUnixMs: 1,
	expiresAtUnixMs: 2,
	membershipFetchedAtUnixMs: 1,
}
assert.equal(authSessionSchema.safeParse(session).success, true)

console.log('ipc-schemas: proposal null Option fields OK')
console.log('ipc-schemas: auth schemas OK')
