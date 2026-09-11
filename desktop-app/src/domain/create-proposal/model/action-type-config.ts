import { DEFCON_COPY } from '@/lib/defcon-copy'
import type { ActionType } from './create-proposal.types'

export type ActionTypeOption = {
	actionType: ActionType
	title: string
	description: string
}

const ACTION_TYPE_OPTIONS: Record<ActionType, ActionTypeOption> = {
	vk_update: {
		actionType: 'vk_update',
		title: 'Verification key update',
		description: 'Rotate the Alpen VK.',
	},
	signer_update: {
		actionType: 'signer_update',
		title: 'Signer update',
		description: 'Add / remove signers or change threshold.',
	},
	// Title matches `lib/proposal-type-label.ts`, so the menu, the dashboard and the detail view
	// say one thing. Neither the title nor the description may contain the exact substring
	// "Signer update": three WebDriver specs select the administrator's card by that text, and
	// `ActionTypeCard` renders both as `<p>` elements the same XPath matches (§6).
	council_signer_update: {
		actionType: 'council_signer_update',
		title: 'Security Council signer update',
		description: 'Add / remove council keys or change threshold.',
	},
	operator_set_update: {
		actionType: 'operator_set_update',
		title: 'Bridge Operator update',
		description: 'Add operators by key or remove by index.',
	},
	sequencer_key_update: {
		actionType: 'sequencer_key_update',
		title: 'Sequencer key update',
		description: 'Rotate the sequencer public key.',
	},
	// Both Defcon cards read their words from the shared copy table, so the menu cannot describe
	// one lever with the other's wording — the mistake Constraint 5 exists to prevent.
	defcon_1: {
		actionType: 'defcon_1',
		title: DEFCON_COPY.defcon_1.menuTitle,
		description: DEFCON_COPY.defcon_1.menuDescription,
	},
	defcon_3: {
		actionType: 'defcon_3',
		title: DEFCON_COPY.defcon_3.menuTitle,
		description: DEFCON_COPY.defcon_3.menuDescription,
	},
}

/** Action types available to each authority, in display order. The first entry is the default selection. */
const ACTION_TYPES_BY_AUTHORITY: Record<string, ActionType[]> = {
	// council_signer_update sits after signer_update and before vk_update, so the default
	// selection (the first entry) does not move.
	strata_admin: ['signer_update', 'council_signer_update', 'vk_update', 'operator_set_update'],
	sequencer_manager: ['signer_update', 'sequencer_key_update'],
	alpen_admin: ['signer_update', 'vk_update'],
	// Defcon 1 first, and therefore the council's default selection: the immediate lever is the one
	// an emergency reaches for, and a default is a decision, not an accident.
	security_council: ['defcon_1', 'defcon_3'],
}

// An authority absent from the map above gets no menu at all, never the administrator's — in
// practice this is `payout_admin`, which has no ASM role. Before council_signer_update existed,
// borrowing the administrator's list was merely wrong; now it would authorize a council rotation
// for an authority nobody enumerated, and the schema's own gate reads this same function
// (create-proposal.schema.ts), so it would authorize it too (§4.6).
const UNKNOWN_AUTHORITY_ACTION_TYPES: ActionType[] = []

/** The action type's display title — the same string the selection card carries. */
export function actionTypeTitle(actionType: ActionType): string {
	return ACTION_TYPE_OPTIONS[actionType].title
}

export function getActionTypeOptions(authority: string): ActionTypeOption[] {
	const actionTypes = ACTION_TYPES_BY_AUTHORITY[authority] ?? UNKNOWN_AUTHORITY_ACTION_TYPES
	return actionTypes.map((actionType) => ACTION_TYPE_OPTIONS[actionType])
}

export function getDefaultActionType(authority: string): ActionType {
	const options = getActionTypeOptions(authority)
	const first = options[0]
	// The unknown-authority fallback (§4.6) is an empty list, not the administrator's — correct,
	// but it makes this reachable for an authority nobody enumerated. Today no caller can actually
	// get here (every session authority that reaches this form is one of the four wired above),
	// so this is a backstop, not a repair: it fails with a message that names the authority
	// instead of a cryptic `Cannot read properties of undefined` that would take down the screen.
	if (first === undefined) {
		throw new Error(`No action types are configured for authority \`${authority}\``)
	}
	return first.actionType
}
