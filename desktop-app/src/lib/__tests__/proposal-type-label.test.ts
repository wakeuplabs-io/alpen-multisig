// proposal-type-label — the one Defcon 3 change with no compiler net.
//
// `inferProposalTypeLabel` is a chain of `if`s ending in `return 'Unknown'`, so a missing arm is
// not a build failure: it is a dashboard, a detail view and a sign-screen header that all name the
// action wrongly. It has two silent failure modes and both are asserted here — the arm left out
// (`Unknown`) and the arm copied from the line above it (`Defcon 1`), which would print the
// immediate lever's name over the timelocked one.

import assert from 'node:assert/strict'
import { inferProposalTypeLabel } from '../proposal-type-label.ts'
import type { ActionType, Proposal, ProposalKind } from '../../api/proposals.ts'

function proposal(actionType: ActionType, kind: ProposalKind = 'update', authority = 'security_council') {
	return { actionType, kind, authority } as Proposal
}

assert.equal(inferProposalTypeLabel(proposal('defcon_3')), 'Defcon 3')
assert.notEqual(inferProposalTypeLabel(proposal('defcon_3')), 'Defcon 1')
assert.notEqual(inferProposalTypeLabel(proposal('defcon_3')), 'Unknown')

assert.equal(inferProposalTypeLabel(proposal('defcon_1')), 'Defcon 1')

// A cancel is named by what it is, never by the action hex it wraps — the rule Phase 7 leans on
// once the council can cancel a queued Defcon 3.
assert.equal(inferProposalTypeLabel(proposal('defcon_3', 'cancel')), 'Cancel')

// The authority still disambiguates a multisig update, and Defcon 3 must not have disturbed it.
assert.equal(inferProposalTypeLabel(proposal('multisig_update', 'update', 'sequencer_manager')), 'Sequencer update')
assert.equal(inferProposalTypeLabel(proposal('multisig_update', 'update', 'strata_admin')), 'Signer update')

console.log('proposal-type-label: all assertions passed.')
