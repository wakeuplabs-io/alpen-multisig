// The safe harbor address update, in the parts the frontend decides on its own
// (docs/specs/security-council-safe-harbor-address.md AC 1, 1a, 3c). The address itself
// is validated in Rust — bech32m does not check the curve, and the descriptor the device shows is
// derived there — so what is left here is the menu, the authority gate and the no-op rule.
//
// The two addresses below are real regtest taproot addresses for two different x-only keys — both
// rendered by `SafeHarborDescriptor::to_address` — so the no-op assertions cannot pass by
// comparing a string to itself, and a signer could paste either of them for real.

import assert from 'node:assert/strict'
import { getActionTypeOptions } from '../action-type-config.ts'
import { buildCreateProposalFormSchema } from '../create-proposal.schema.ts'
import { NO_OP_SAFE_HARBOR_MESSAGE } from '../validators/safe-harbor-address-update.ts'

/** Taproot output for the secp256k1 generator point — the destination the local stack ships with. */
const CURRENT_ADDRESS = 'bcrt1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqc8gma6'
/** A different destination: the taproot output for the x-only key `0x0202…02`. */
const NEW_ADDRESS = 'bcrt1pqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqn4zdly'
const DESCRIPTOR_HEX = `04${'79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798'}`

const OTHER_AUTHORITIES = ['security_council', 'sequencer_manager', 'alpen_admin'] as const

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
	newSafeHarborAddress: '',
	defconConfirm: '',
	defconMessage: '',
}

function issues(
	field: string,
	{
		authority = 'strata_admin',
		currentSafeHarborAddress,
		newSafeHarborAddress,
	}: { authority?: string; currentSafeHarborAddress: string | null; newSafeHarborAddress: string },
): string[] {
	const result = buildCreateProposalFormSchema({
		currentMultisigSigners: null,
		currentMultisigThreshold: null,
		authority,
		currentSafeHarborAddress,
	}).safeParse({
		...draft,
		actionType: 'safe_harbour_address_update',
		newSafeHarborAddress,
	})
	if (result.success) return []
	return result.error.issues.filter((issue) => issue.path[0] === field).map((issue) => issue.message)
}

// Claim 1 (AC 1) — the Strata Administrator is offered it last, so the default does not move — is
// asserted with the whole menu order in council-signer-update-retarget.test.ts.

// ─── claim 2 (AC 1a): no other authority can reach it ───

for (const authority of OTHER_AUTHORITIES) {
	const offered = getActionTypeOptions(authority).map((option) => option.actionType)
	assert.ok(
		!offered.includes('safe_harbour_address_update'),
		`claim 2 (AC 1a): ${authority} must not be offered the safe harbor address update`,
	)
}

// The menu is display data; this is the rule. It is what a stale form value or a direct route hits.
// `security_council` is the case that matters: it triggers the sweep and must not pick the
// destination.
for (const authority of OTHER_AUTHORITIES) {
	assert.ok(
		issues('actionType', { authority, currentSafeHarborAddress: null, newSafeHarborAddress: NEW_ADDRESS }).length > 0,
		`claim 2 (AC 1a): the schema must refuse a safe harbor update authored by ${authority}`,
	)
}

// ─── claim 3: the field is required ───

assert.ok(
	issues('newSafeHarborAddress', { currentSafeHarborAddress: CURRENT_ADDRESS, newSafeHarborAddress: '  ' }).length > 0,
	'claim 3: an empty destination is refused',
)

// ─── claim 4: pasting the descriptor hex is named for what it is ───

const hexIssues = issues('newSafeHarborAddress', {
	currentSafeHarborAddress: CURRENT_ADDRESS,
	newSafeHarborAddress: DESCRIPTOR_HEX,
})
assert.ok(hexIssues.length > 0, 'claim 4: the descriptor hex is not an address and is refused')
assert.match(
	hexIssues[0] ?? '',
	/descriptor hex/,
	'claim 4: the signer reads the descriptor on their device, so the error must say which of the two this field takes',
)

// ─── claim 5 (AC 3c): rotating to the destination already installed is refused ───

assert.deepEqual(
	issues('newSafeHarborAddress', {
		currentSafeHarborAddress: CURRENT_ADDRESS,
		newSafeHarborAddress: CURRENT_ADDRESS,
	}),
	[NO_OP_SAFE_HARBOR_MESSAGE],
	'claim 5 (AC 3c): the chain accepts this and reports it as Enacted, so the form is the only place it can be caught',
)

assert.deepEqual(
	issues('newSafeHarborAddress', {
		currentSafeHarborAddress: CURRENT_ADDRESS,
		newSafeHarborAddress: NEW_ADDRESS,
	}),
	[],
	'claim 5 (AC 3c): a different destination is a real change and passes — the counter-case',
)

// Bech32m is case-insensitive, and an address pasted from a screen may arrive uppercased.
assert.deepEqual(
	issues('newSafeHarborAddress', {
		currentSafeHarborAddress: CURRENT_ADDRESS,
		newSafeHarborAddress: CURRENT_ADDRESS.toUpperCase(),
	}),
	[NO_OP_SAFE_HARBOR_MESSAGE],
	'claim 5 (AC 3c): case must not be a way past the rule',
)

// ─── claim 6 (AC 3c): the rule is off when the read failed, and the form blocks submission then ───

assert.deepEqual(
	issues('newSafeHarborAddress', { currentSafeHarborAddress: null, newSafeHarborAddress: CURRENT_ADDRESS }),
	[],
	'claim 6 (AC 3c): with no current destination read there is nothing to compare against, so the rule stays silent — safe only because the form disables both CTAs in that state',
)

console.log('safe-harbor-address-update: all assertions passed')
