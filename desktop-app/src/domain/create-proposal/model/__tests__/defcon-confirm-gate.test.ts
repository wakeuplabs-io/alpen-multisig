// The two frontend gates on the Defcon levers: the type-to-confirm friction (AC 5) and the
// authority-keyed menu that is the only entry point to them (AC 1, AC 1a).
//
// The pure matcher is pinned next to the copy it reads, in `lib/__tests__/defcon-copy.test.ts`.
// What this file adds is the same claims *through the schema* — the only place that catches a
// validator entry wired to the wrong level, which is exactly the mistake sharing one gate
// function makes possible.

import assert from 'node:assert/strict'
import { DEFCON_COPY } from '@/lib/defcon-copy'
import { getActionTypeOptions } from '../action-type-config.ts'
import { buildCreateProposalFormSchema } from '../create-proposal.schema.ts'
import type { ActionType } from '../create-proposal.types.ts'

const draft = {
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
	defconMessage: 'Strata ASM Administration v1',
}

function issuesOn(
	field: 'actionType' | 'defconConfirm',
	{ authority, actionType, defconConfirm }: { authority: string; actionType: ActionType; defconConfirm: string },
): number {
	const result = buildCreateProposalFormSchema({ currentMultisigSigners: [], authority }).safeParse({
		...draft,
		actionType,
		defconConfirm,
	})
	if (result.success) return 0
	return result.error.issues.filter((issue) => issue.path[0] === field).length
}

// AC 5 — each level's gate accepts its own string...
for (const level of ['defcon_1', 'defcon_3'] as const) {
	assert.equal(
		issuesOn('defconConfirm', {
			authority: 'security_council',
			actionType: level,
			defconConfirm: DEFCON_COPY[level].confirmation,
		}),
		0,
		`${DEFCON_COPY[level].confirmation} must arm ${level}`,
	)
}

// ...and refuses the other's. Typing DEFCON 1 into a Defcon 3 draft leaves the CTA disabled, and
// the reverse. This is the property a shared form component could break silently: it holds even if
// the matcher is correct, because it also proves each validator entry carries its own level.
for (const [actionType, other] of [
	['defcon_1', 'defcon_3'],
	['defcon_3', 'defcon_1'],
] as const) {
	assert.ok(
		issuesOn('defconConfirm', {
			authority: 'security_council',
			actionType,
			defconConfirm: DEFCON_COPY[other].confirmation,
		}) > 0,
		`${DEFCON_COPY[other].confirmation} must not satisfy the ${actionType} gate`,
	)
}

// AC 1 — the council's menu, in display order. The order is the claim, not just the membership:
// the first entry is the default selection, and Defcon 1 being the council's default is deliberate.
assert.deepEqual(
	getActionTypeOptions('security_council').map((option) => option.actionType),
	['defcon_1', 'defcon_3'],
)

// AC 1a — no other authority is offered either lever. `getActionTypeOptions` falls back to the
// Strata Administrator's menu for an unknown authority, so that fallback is checked too: it is the
// path a typo'd authority string would take.
for (const authority of ['strata_admin', 'sequencer_manager', 'alpen_admin', 'not_an_authority']) {
	const actionTypes = getActionTypeOptions(authority).map((option) => option.actionType)
	for (const level of ['defcon_1', 'defcon_3'] as const) {
		assert.ok(!actionTypes.includes(level), `${authority} must not be offered ${level}`)
	}
}

// ...and the menu is not the only thing enforcing that. The schema itself refuses an action the
// session's authority cannot author, so no stale form state or future route can reach a device
// prompt for one. The backend refuses it too (AC 2); this is the half the signer never sees.
for (const level of ['defcon_1', 'defcon_3'] as const) {
	const defconConfirm = DEFCON_COPY[level].confirmation
	assert.equal(
		issuesOn('actionType', { authority: 'security_council', actionType: level, defconConfirm }),
		0,
		`the council may author ${level}`,
	)
	assert.ok(
		issuesOn('actionType', { authority: 'strata_admin', actionType: level, defconConfirm }) > 0,
		`a non-council authority must not author ${level}`,
	)
}

console.log('defcon confirm gate: per-level gates and the authority menu OK')
