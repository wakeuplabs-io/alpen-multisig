import assert from 'node:assert/strict'
import { proposalSendState, showsSendButton } from '../../../../lib/proposal-send-state.ts'
import { buildSyntheticProposal } from '../synthetic-proposal.ts'

const importData = {
	actionHex: '00',
	seqNo: 7,
	authority: 'strata_admin',
	sighashHex: 'ab'.repeat(32),
	actionType: 'multisig_update' as const,
}
const signature = { signerPubkey: '02aa', signatureHex: 'ff', source: 'local' as const }

const belowQuorum = buildSyntheticProposal({
	importData,
	localSignatures: [signature],
	requiredSignatures: 2,
	hasQuorum: false,
})
assert.equal(belowQuorum.status, 'pending', 'below quorum the manual proposal stays pending')
assert.equal(showsSendButton(proposalSendState(belowQuorum)), false, 'below quorum there is nothing to send yet')

const atQuorum = buildSyntheticProposal({
	importData,
	localSignatures: [signature, { ...signature, signerPubkey: '02bb' }],
	requiredSignatures: 2,
	hasQuorum: true,
})
assert.equal(atQuorum.status, 'approved', 'quorum stands in for the approval the orchestrator would give')
assert.equal(atQuorum.broadcastStatus, 'idle')
assert.equal(showsSendButton(proposalSendState(atQuorum)), true, 'a manual proposal at quorum offers Send (#605)')
assert.deepEqual(
	atQuorum.signatures,
	[
		{ signerPubkey: '02aa', signatureHex: 'ff' },
		{ signerPubkey: '02bb', signatureHex: 'ff' },
	],
	'the detail view gets the signatures without their local/pasted source',
)

console.log('synthetic-proposal: ok')
