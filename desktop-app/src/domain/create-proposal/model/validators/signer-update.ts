import { normalizeSignerKey, countSignersAfterUpdate } from '../create-proposal.schema'
import type { ActionValidator } from './types'
import { compressedPubKeyHexPattern } from './types'

export const validateSignerUpdate: ActionValidator = ({ data, ctx, currentMultisigSigners }) => {
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
	}
}
