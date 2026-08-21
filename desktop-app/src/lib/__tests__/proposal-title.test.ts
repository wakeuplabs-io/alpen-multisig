// proposal-title — the heading a proposal is shown under (#491).
//
// The reported bug: the Create proposal form asked for a Title, showed it in the review step, and
// then dropped it. Every screen labelled the card from the sequence number and the action type
// instead, so three queued `Proposal #N - Signer update` rows were indistinguishable and the
// proposer could not find their own work by the name they gave it.
//
// What is pinned here: an authored title wins, and every case where there is no usable title falls
// back to the derived label rather than rendering a blank heading.

import assert from 'node:assert/strict'
import type { Proposal } from '../../api/proposals'
import { buildProposalTitle, derivedProposalLabel } from '../proposal-title'

function proposal(overrides: Partial<Proposal> = {}): Proposal {
	return {
		actionId: 'a1',
		seqNo: 8,
		authority: 'strata_admin',
		status: 'pending',
		requiredSignatures: 2,
		actionHex: 'deadbeef',
		actionType: 'multisig_update',
		title: null,
		signatures: [],
		broadcastStatus: 'idle',
		kind: 'update',
		targetActionId: null,
		activationHeight: null,
		updateIdInQueue: null,
		cancelProposal: null,
		createdAtMs: 0,
		expiresAtMs: 0,
		...overrides,
	} as Proposal
}

// ── An authored title is what the reader is looking for ──

assert.equal(buildProposalTitle(proposal({ title: 'Rotate signing key Q3' })), 'Rotate signing key Q3')

// Surrounding whitespace is the author's typo, not part of the name.
assert.equal(buildProposalTitle(proposal({ title: '  Rotate signing key Q3  ' })), 'Rotate signing key Q3')

// ── No usable title falls back to the derived label, never to a blank heading ──

assert.equal(buildProposalTitle(proposal({ title: null })), 'Proposal #8 - Signer update')
assert.equal(buildProposalTitle(proposal({ title: '' })), 'Proposal #8 - Signer update')

// Whitespace-only is indistinguishable from empty on screen, so it must behave the same way.
assert.equal(buildProposalTitle(proposal({ title: '   ' })), 'Proposal #8 - Signer update')

// ── Cancel proposals are labelled from their target, with or without a title ──

assert.equal(buildProposalTitle(proposal({ kind: 'cancel', title: null })), 'Cancel #8')

// ── The derived label stays available on its own, for callers that want both ──

assert.equal(derivedProposalLabel(proposal({ title: 'Rotate signing key Q3' })), 'Proposal #8 - Signer update')
assert.equal(derivedProposalLabel(proposal({ actionType: 'vk_update' })), 'Proposal #8 - Verification key update')

console.log('proposal-title: all assertions passed')
