import type { DecodedAction } from '@/api/signing'

type MultisigUpdate = Extract<DecodedAction, { kind: 'multisig_update' }>

/**
 * What a multisig update actually changes, so the review screen lists only that
 * (feedback 2026-07-01 #423: a proposal that only adds a signer still reported
 * "New threshold: 2", which the signer never asked for).
 */
export type MultisigUpdateChanges = {
	/** Whether the threshold row is worth showing at all. */
	showThreshold: boolean
	newThreshold: number
	addKeys: string[]
	removeKeys: string[]
	/** False only for a proposal that changes nothing the signer can see. */
	hasAnyChange: boolean
}

/**
 * `currentThreshold` is `null` while the on-chain config is still loading or could
 * not be read. In that case the threshold is shown: hiding a value we cannot prove
 * is unchanged would withhold information from the signer, which is the worse failure.
 */
export function multisigUpdateChanges(action: MultisigUpdate, currentThreshold: number | null): MultisigUpdateChanges {
	const showThreshold = currentThreshold === null || action.newThreshold !== currentThreshold
	const addKeys = action.addKeys.filter((key) => key.trim().length > 0)
	const removeKeys = action.removeKeys.filter((key) => key.trim().length > 0)

	return {
		showThreshold,
		newThreshold: action.newThreshold,
		addKeys,
		removeKeys,
		hasAnyChange: showThreshold || addKeys.length > 0 || removeKeys.length > 0,
	}
}
