// proposal-type-label — the one Defcon 3 change with no compiler net.
//
// `inferProposalTypeLabel` is a chain of `if`s ending in `return 'Unknown'`, so a missing arm is
// not a build failure: it is a dashboard, a detail view and a sign-screen header that all name the
// action wrongly. Its two silent failure modes — the arm left out (`Unknown`) and the arm copied
// from the line above it — are both caught by asserting the exact label.

import assert from 'node:assert/strict'
import { inferProposalTypeLabel } from '../proposal-type-label.ts'
import type { ActionType, Proposal, ProposalKind } from '../../api/proposals.ts'

function proposal(actionType: ActionType, kind: ProposalKind = 'update', authority = 'security_council') {
	return { actionType, kind, authority } as Proposal
}

assert.equal(inferProposalTypeLabel(proposal('defcon_3')), 'Defcon 3')

assert.equal(inferProposalTypeLabel(proposal('defcon_1')), 'Defcon 1')

// A cancel is named by what it is, never by the action hex it wraps — the rule the council's
// cancel of a queued Defcon 3 leans on. The offline route sets `kind` from the decoded
// actionType, so the `kind === 'cancel'` arm is enough on every path.
assert.equal(inferProposalTypeLabel(proposal('defcon_3', 'cancel')), 'Cancel')

// The authority still disambiguates a multisig update, and Defcon 3 must not have disturbed it.
assert.equal(inferProposalTypeLabel(proposal('multisig_update', 'update', 'sequencer_manager')), 'Sequencer update')
assert.equal(inferProposalTypeLabel(proposal('multisig_update', 'update', 'strata_admin')), 'Signer update')

// A council rotation's proposal authority IS `strata_admin` (the existing `multisig_update` arm
// above derives its label from that same authority), so passing `strata_admin` explicitly here
// proves the new label comes from `actionType`, not from a copy of the authority-branching logic.
assert.equal(
	inferProposalTypeLabel(proposal('council_signer_update', 'update', 'strata_admin')),
	'Security Council signer update',
)

console.log('proposal-type-label: all assertions passed.')
