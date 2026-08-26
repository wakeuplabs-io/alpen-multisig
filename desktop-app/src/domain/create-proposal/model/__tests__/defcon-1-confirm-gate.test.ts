// Defcon 1 is irreversible, so its two frontend gates get pinned: the type-to-confirm
// friction (AC 5) and the authority-keyed menu that is the only entry point to it (AC 1).

import assert from 'node:assert/strict'
import { matchesDefconConfirmation } from '../validators/defcon-1.ts'
import { getActionTypeOptions } from '../action-type-config.ts'
import { buildCreateProposalFormSchema } from '../create-proposal.schema.ts'

// AC 5 — case-insensitive, and nothing else. The contract's own rule is
// `input.toUpperCase() === "DEFCON 1"`, so there is deliberately no trim().
for (const accepted of ['DEFCON 1', 'defcon 1', 'Defcon 1']) {
	assert.equal(matchesDefconConfirmation(accepted), true, `must accept ${JSON.stringify(accepted)}`)
}
// The near-misses the contract's Edge Cases name by hand, plus the empty field.
for (const rejected of ['defcon1', 'DEFCON 1 ', ' DEFCON 1', 'DEFCON', '']) {
	assert.equal(matchesDefconConfirmation(rejected), false, `must reject ${JSON.stringify(rejected)}`)
}

// AC 1 — the council is offered Defcon 1 and nothing else...
assert.deepEqual(
	getActionTypeOptions('security_council').map((option) => option.actionType),
	['defcon_1'],
)

// ...and no other authority is offered it. `getActionTypeOptions` falls back to the Strata
// Administrator's menu for an unknown authority, so that fallback is checked too: it is the
// path a typo'd authority string would take.
for (const authority of ['strata_admin', 'sequencer_manager', 'alpen_admin', 'not_an_authority']) {
	const actionTypes = getActionTypeOptions(authority).map((option) => option.actionType)
	assert.ok(!actionTypes.includes('defcon_1'), `${authority} must not be offered defcon_1`)
}

// ...and the menu is not the only thing enforcing that. The schema itself refuses an action the
// session's authority cannot author, so no stale form state or future route can reach a device
// prompt for one. The backend refuses it too (AC 17); this is the half the signer never sees.
const draft = {
	actionType: 'defcon_1' as const,
	seqNo: '1',
	title: '',
	keysToAdd: [{ value: '' }],
	keysToRemove: [{ value: '' }],
	threshold: '2',
	vkTypeId: 'always_accept' as const,
	newVkHex: '',
	operatorsToAdd: [{ value: '' }],
	operatorIndicesToRemove: [{ value: '' }],
	newSequencerKeyHex: '',
	defconConfirm: 'DEFCON 1',
	defconMessage: 'Strata ASM Administration v1',
}

function actionTypeIssues(authority: string): number {
	const result = buildCreateProposalFormSchema({ currentMultisigSigners: [], authority }).safeParse(draft)
	if (result.success) return 0
	return result.error.issues.filter((issue) => issue.path[0] === 'actionType').length
}

assert.equal(actionTypeIssues('security_council'), 0, 'the council may author defcon_1')
assert.ok(actionTypeIssues('strata_admin') > 0, 'a non-council authority must not author defcon_1')
