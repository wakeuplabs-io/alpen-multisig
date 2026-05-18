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

console.log('ipc-schemas: proposal null Option fields OK')
