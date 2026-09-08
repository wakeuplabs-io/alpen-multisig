// V3 Phase 3 — the create-proposal form retargets to the Security Council (Constraint 2).
//
// Eight claims, pinned in `docs/specs/security-council-signer-update-phase-3.md` §7.1. The
// fixture's two signer sets differ in both membership and cardinality on purpose: with equal
// sizes, half of these assertions would pass by accident.
//
// Red at HEAD: neither `council_signer_update` (the form's ActionType) nor `multisigTargetAuthority`
// exists yet. Both land in the vocabulary commit that follows this one.

import assert from 'node:assert/strict'
import { getActionTypeOptions, getDefaultActionType } from '../action-type-config.ts'
import { buildCreateProposalFormSchema } from '../create-proposal.schema.ts'
import { multisigTargetAuthority } from '../multisig-target.ts'
import { removesCurrentMembers } from '../validators/signer-update.ts'

const KEY_A = `02${'a'.repeat(64)}`
const KEY_B = `02${'b'.repeat(64)}`
const KEY_C = `02${'c'.repeat(64)}`
const KEY_X = `02${'1'.repeat(64)}`
const KEY_Y = `02${'2'.repeat(64)}`
const KEY_Z = `02${'3'.repeat(64)}`
const KEY_W = `02${'4'.repeat(64)}`

const ADMIN = { signers: [KEY_A, KEY_B, KEY_C], threshold: 3 }
const COUNCIL = { signers: [KEY_X, KEY_Y, KEY_Z, KEY_W], threshold: 2 }

const OTHER_AUTHORITIES = ['security_council', 'sequencer_manager', 'alpen_admin'] as const
const ALL_AUTHORITIES = ['strata_admin', 'security_council', 'sequencer_manager', 'alpen_admin'] as const

const draft = {
	seqNo: '1',
	title: '',
	keysToAdd: [{ value: '' }],
	keysToRemove: [{ value: '' }],
	threshold: '',
	vkTypeId: 'always_accept' as const,
	newVkHex: '',
	operatorsToAdd: [{ value: '' }],
	operatorIndicesToRemove: [{ value: '' }],
	newSequencerKeyHex: '',
	defconConfirm: '',
	defconMessage: '',
}

function issuesOn(
	field: string,
	{
		authority,
		currentMultisigSigners,
		overrides,
	}: { authority: string; currentMultisigSigners: string[] | null; overrides: Record<string, unknown> },
): number {
	const result = buildCreateProposalFormSchema({
		currentMultisigSigners,
		currentMultisigThreshold: null,
		authority,
	}).safeParse({
		...draft,
		actionType: 'council_signer_update',
		...overrides,
	})
	if (result.success) return 0
	return result.error.issues.filter((issue) => issue.path[0] === field).length
}

// Claim 1 (AC 1) — the council entry joins the administrator's menu, after signer_update and
// before vk_update, and the default selection does not move: it is the first entry, unchanged.
assert.deepEqual(
	getActionTypeOptions('strata_admin').map((option) => option.actionType),
	['signer_update', 'council_signer_update', 'vk_update', 'operator_set_update'],
	'claim 1: strata_admin menu order',
)
assert.equal(getDefaultActionType('strata_admin'), 'signer_update', 'claim 1: default selection unchanged')

// Claim 2 (AC 1a) — no other authority is offered the entry, including the case that matters
// most: the council itself must not be able to rotate itself. `not_an_authority` walks §4.6's
// fallback path.
for (const authority of [...OTHER_AUTHORITIES, 'not_an_authority']) {
	const actionTypes = getActionTypeOptions(authority).map((option) => option.actionType)
	assert.ok(!actionTypes.includes('council_signer_update'), `claim 2: ${authority} must not be offered the entry`)
}

// Claim 3 (AC 1a, through the schema) — the half the menu does not cover: stale form state or a
// direct route still cannot author the action against a session the schema itself refuses.
assert.equal(
	issuesOn('actionType', { authority: 'strata_admin', currentMultisigSigners: ADMIN.signers, overrides: {} }),
	0,
	'claim 3: strata_admin may author a council rotation',
)
for (const authority of OTHER_AUTHORITIES) {
	assert.ok(
		issuesOn('actionType', { authority, currentMultisigSigners: null, overrides: {} }) > 0,
		`claim 3: ${authority} must not author a council rotation`,
	)
}

// Claim 4 (Constraint 2) — the pure function every retargeted consumer shares. The test that
// fails if someone "simplifies" it to read the session instead of the action.
for (const authority of ALL_AUTHORITIES) {
	assert.equal(
		multisigTargetAuthority('council_signer_update', authority),
		'security_council',
		`claim 4: council_signer_update always targets the council, from ${authority}`,
	)
	assert.equal(
		multisigTargetAuthority('signer_update', authority),
		authority,
		`claim 4: every other action type targets the session, from ${authority}`,
	)
}

// Claim 5 (AC 3) — one draft, two contexts: the "already exists" rule reads whichever config the
// draft was validated against, not a fixed one.
function addKeyIssues(currentMultisigSigners: string[], key: string): number {
	return issuesOn('keysToAdd', {
		authority: 'strata_admin',
		currentMultisigSigners,
		overrides: { keysToAdd: [{ value: key }], threshold: String(currentMultisigSigners.length) },
	})
}
assert.ok(addKeyIssues(COUNCIL.signers, KEY_X) > 0, 'claim 5: X already exists on the council')
assert.equal(addKeyIssues(ADMIN.signers, KEY_X), 0, 'claim 5: X does not exist on the administrator')
assert.ok(addKeyIssues(ADMIN.signers, KEY_A) > 0, 'claim 5: A already exists on the administrator')
assert.equal(addKeyIssues(COUNCIL.signers, KEY_A), 0, 'claim 5: A does not exist on the council')

// Claim 6 (AC 3a) — the differing cardinalities are what make this discriminate: removing two of
// the council's four members leaves a threshold of 3 unreachable; the same two names, read
// against the administrator's three (where they are not members at all), do not.
function removeIssues(currentMultisigSigners: string[], removeKeys: string[], threshold: number): number {
	return issuesOn('threshold', {
		authority: 'strata_admin',
		currentMultisigSigners,
		overrides: { keysToRemove: removeKeys.map((value) => ({ value })), threshold: String(threshold) },
	})
}
assert.ok(removeIssues(COUNCIL.signers, [KEY_X, KEY_Y], 3) > 0, 'claim 6: 2 of 4 council members removed, threshold 3')
assert.equal(removeIssues(ADMIN.signers, [KEY_X, KEY_Y], 3), 0, 'claim 6: same names, read against the administrator')

// ---------------------------------------------------------------------------------------------
// Claims 7 and 8 stay red on purpose past this commit — see SDD §8, commits 5 and 6. Uncomment
// each block (and its import above, for claim 8) once its commit lands.
// ---------------------------------------------------------------------------------------------

// Claim 7 (AC 3b) — the no-op rule reads the target's *current* threshold, not any other one.
function noOpIssues(
	currentMultisigSigners: string[],
	currentMultisigThreshold: number,
	overrides: Record<string, unknown>,
): number {
	const result = buildCreateProposalFormSchema({
		currentMultisigSigners,
		currentMultisigThreshold,
		authority: 'strata_admin',
	}).safeParse({ ...draft, actionType: 'council_signer_update', ...overrides })
	if (result.success) return 0
	return result.error.issues.filter((issue) => issue.path[0] === 'keysToAdd').length
}
assert.ok(
	noOpIssues(COUNCIL.signers, COUNCIL.threshold, {
		keysToAdd: [{ value: '' }],
		keysToRemove: [{ value: '' }],
		threshold: String(COUNCIL.threshold),
	}) > 0,
	'claim 7: blank rows and an unchanged threshold is a no-op',
)
assert.equal(
	noOpIssues(COUNCIL.signers, COUNCIL.threshold, {
		keysToAdd: [{ value: '' }],
		keysToRemove: [{ value: '' }],
		threshold: String(COUNCIL.threshold + 1),
	}),
	0,
	'claim 7: blank rows, threshold actually changes — the mandatory counter-case',
)
assert.equal(
	noOpIssues(COUNCIL.signers, COUNCIL.threshold, {
		keysToAdd: [{ value: KEY_A }],
		keysToRemove: [{ value: '' }],
		threshold: String(COUNCIL.threshold),
	}),
	0,
	'claim 7: a real add with an unchanged threshold is allowed',
)
assert.equal(
	noOpIssues(COUNCIL.signers, COUNCIL.threshold, {
		keysToAdd: [{ value: '' }],
		keysToRemove: [{ value: '' }],
		threshold: String(ADMIN.threshold),
	}),
	0,
	"claim 7: blank rows at the administrator's threshold, not the council's, is allowed",
)

// Claim 8 (AC 11) — the consequence-stating predicate reads the council's roster, not the
// administrator's.
assert.equal(removesCurrentMembers(COUNCIL.signers, [{ value: KEY_X }]), true, 'claim 8: X is a current council member')
assert.equal(removesCurrentMembers(COUNCIL.signers, [{ value: KEY_A }]), false, 'claim 8: A is not on the council')

console.log('council signer update retarget: all assertions passed')
