import { normalizeSignerKey, countSignersAfterUpdate } from '../create-proposal.schema'
import type { ActionValidator } from './types'
import { compressedPubKeyHexPattern } from './types'

// Upstream accepts an empty update: every rejection in `validate_update` passes with empty add
// and remove sets, `apply_update` retains the same signers and re-sets the same threshold, and
// `handle_action` advances `last_seqno` regardless. The on-chain result is a *successful* no-op —
// post-conditions match, and the proposal reports `Enacted` for an update that changed nothing.
// That is what makes this a safety rule rather than hygiene (§4.7).
export const NO_OP_UPDATE_MESSAGE =
	'This update does not change the signer set or the threshold. A no-op still enacts successfully on chain.'

export const validateSignerUpdate: ActionValidator = ({
	data,
	ctx,
	currentMultisigSigners,
	currentMultisigThreshold,
}) => {
	if (data.keysToAdd.length < 1) {
		ctx.addIssue({ code: 'custom', path: ['keysToAdd'], message: 'At least one row for keys to add' })
	}
	if (data.keysToRemove.length < 1) {
		ctx.addIssue({ code: 'custom', path: ['keysToRemove'], message: 'At least one row for keys to remove' })
	}

	for (const [index, row] of data.keysToAdd.entries()) {
		const key = row.value.trim()
		if (key.length === 0) continue
		if (!compressedPubKeyHexPattern.test(key)) {
			ctx.addIssue({
				code: 'custom',
				path: ['keysToAdd', index, 'value'],
				message: 'Signer key must be compressed pubkey hex (33 bytes, 02/03..., optional 0x)',
			})
		}
	}

	for (const [index, row] of data.keysToRemove.entries()) {
		const key = row.value.trim()
		if (key.length === 0) continue
		if (!compressedPubKeyHexPattern.test(key)) {
			ctx.addIssue({
				code: 'custom',
				path: ['keysToRemove', index, 'value'],
				message: 'Signer key must be compressed pubkey hex (33 bytes, 02/03..., optional 0x)',
			})
		}
	}

	const addKeyIndexes = new Map<string, number[]>()
	for (const [index, row] of data.keysToAdd.entries()) {
		const key = row.value.trim()
		if (key.length === 0) continue
		const normalized = normalizeSignerKey(key)
		const indexes = addKeyIndexes.get(normalized) ?? []
		indexes.push(index)
		addKeyIndexes.set(normalized, indexes)
	}

	for (const indexes of addKeyIndexes.values()) {
		if (indexes.length < 2) continue
		for (const index of indexes) {
			ctx.addIssue({
				code: 'custom',
				path: ['keysToAdd', index, 'value'],
				message: 'Duplicate signer key',
			})
		}
	}

	const removeKeyIndexes = new Map<string, number[]>()
	for (const [index, row] of data.keysToRemove.entries()) {
		const key = row.value.trim()
		if (key.length === 0) continue
		const normalized = normalizeSignerKey(key)
		const indexes = removeKeyIndexes.get(normalized) ?? []
		indexes.push(index)
		removeKeyIndexes.set(normalized, indexes)
	}

	for (const indexes of removeKeyIndexes.values()) {
		if (indexes.length < 2) continue
		for (const index of indexes) {
			ctx.addIssue({
				code: 'custom',
				path: ['keysToRemove', index, 'value'],
				message: 'Duplicate signer key',
			})
		}
	}

	for (const [normalized, addIndexes] of addKeyIndexes.entries()) {
		const removeIndexes = removeKeyIndexes.get(normalized)
		if (!removeIndexes) continue
		for (const index of addIndexes) {
			ctx.addIssue({
				code: 'custom',
				path: ['keysToAdd', index, 'value'],
				message: 'Duplicate signer key',
			})
		}
		for (const index of removeIndexes) {
			ctx.addIssue({
				code: 'custom',
				path: ['keysToRemove', index, 'value'],
				message: 'Duplicate signer key',
			})
		}
	}

	if (currentMultisigSigners !== null) {
		const currentNormalized = new Set(currentMultisigSigners.map(normalizeSignerKey))
		for (const [normalized, addIndexes] of addKeyIndexes.entries()) {
			if (!currentNormalized.has(normalized)) continue
			if (removeKeyIndexes.has(normalized)) continue
			for (const index of addIndexes) {
				ctx.addIssue({
					code: 'custom',
					path: ['keysToAdd', index, 'value'],
					message: 'Signer already exists in the current set',
				})
			}
		}
	}

	const th = data.threshold.trim()
	if (!/^\d+$/.test(th)) {
		ctx.addIssue({
			code: 'custom',
			path: ['threshold'],
			message: 'Threshold must be an integer between 1 and 255',
		})
		return
	}
	const thN = Number(th)
	if (!Number.isInteger(thN) || thN < 1 || thN > 255) {
		ctx.addIssue({
			code: 'custom',
			path: ['threshold'],
			message: 'Threshold must be an integer between 1 and 255',
		})
	} else if (currentMultisigSigners !== null) {
		const resultingSignerCount = countSignersAfterUpdate(currentMultisigSigners, data.keysToRemove, data.keysToAdd)
		const removedSet = new Set(
			data.keysToRemove
				.map((row) => row.value.trim())
				.filter((value) => value.length > 0)
				.map(normalizeSignerKey),
		)
		const remainingCurrentSigners = currentMultisigSigners.filter(
			(signer) => !removedSet.has(normalizeSignerKey(signer)),
		).length
		const addedNormalized = [
			...new Set(
				data.keysToAdd
					.map((row) => row.value.trim())
					.filter((value) => value.length > 0)
					.map(normalizeSignerKey),
			),
		]
		const addedSignersNotRemoved = addedNormalized.filter((signer) => !removedSet.has(signer)).length
		if (thN > resultingSignerCount) {
			ctx.addIssue({
				code: 'custom',
				path: ['threshold'],
				message:
					`Threshold cannot be greater than the number of signers after this update ` +
					`(${resultingSignerCount}: ${remainingCurrentSigners} current + ${addedSignersNotRemoved} added not removed).`,
			})
		}

		// AC 3b: a set question, not a count question — counting members before and after cannot
		// tell "nothing changed" from "removed one, added another". `addKeyIndexes` and
		// `removeKeyIndexes` are already keyed by normalized key with blank rows discarded, which
		// is exactly the AC's wording. Goes last, after the threshold has parsed, and only when
		// the target's current threshold is actually known (`currentMultisigThreshold === null`
		// means the config read is unavailable, and §4.8 blocks submission in that state).
		const changesKeys = addKeyIndexes.size > 0 || removeKeyIndexes.size > 0
		if (!changesKeys && currentMultisigThreshold !== null && thN === currentMultisigThreshold) {
			ctx.addIssue({ code: 'custom', path: ['keysToAdd'], message: NO_OP_UPDATE_MESSAGE })
		}
	}
}
