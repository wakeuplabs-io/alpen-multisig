import assert from 'node:assert/strict'
import type { Proposal } from '../../api/proposals.ts'
import { redundantDefcon1ActionIds } from '../redundant-defcon-1.ts'

function proposal(actionId: string, seqNo: number, status: Proposal['status'], actionType: string): Proposal {
	return {
		actionId,
		seqNo,
		status,
		actionType,
		authority: 'security_council',
		requiredSignatures: 2,
		actionHex: 'aa',
		title: null,
		signatures: [],
		broadcastStatus: 'reveal_confirmed',
		kind: 'update',
		targetActionId: null,
		activationHeight: null,
		updateIdInQueue: null,
		cancelProposal: null,
		createdAtMs: 0,
		updatedAtMs: 0,
		expiresAtMs: 0,
	} as unknown as Proposal
}

// The earliest enacted Defcon 1 by sequence number is the one that activated the safe harbour;
// every enacted one after it ran against a flag that was already true.
const redundant = redundantDefcon1ActionIds([
	proposal('c', 4, 'enacted', 'defcon_1'),
	proposal('a', 1, 'enacted', 'defcon_1'),
	proposal('b', 2, 'superseded', 'defcon_1'),
	proposal('d', 3, 'enacted', 'defcon_1'),
])
assert.deepEqual([...redundant].sort(), ['c', 'd'], 'every enacted Defcon 1 after the first is redundant')
assert.ok(!redundant.has('a'), 'the first enacted Defcon 1 is the one that activated the harbour')
assert.ok(!redundant.has('b'), 'a proposal that never enacted changed nothing to report')

// Another action type shares no state with the safe harbour.
assert.equal(
	redundantDefcon1ActionIds([proposal('x', 1, 'enacted', 'vk_update'), proposal('y', 2, 'enacted', 'vk_update')]).size,
	0,
	'only Defcon 1 is considered',
)

// A single enactment is the activation itself.
assert.equal(redundantDefcon1ActionIds([proposal('solo', 7, 'enacted', 'defcon_1')]).size, 0)

console.log('redundant-defcon-1: all assertions passed')
