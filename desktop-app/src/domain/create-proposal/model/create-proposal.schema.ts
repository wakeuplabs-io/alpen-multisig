import { z } from 'zod'

/** Row shape so `useFieldArray` is typed (RHF excludes primitive `string[]` from `FieldArrayPath`). */
const keyRowSchema = z.object({
	value: z.string(),
})

const compressedPubKeyHexPattern = /^(?:0x)?(?:02|03)[0-9a-fA-F]{64}$/

function normalizeSignerKey(value: string): string {
	const trimmed = value.trim()
	const withoutPrefix = trimmed.startsWith('0x') || trimmed.startsWith('0X') ? trimmed.slice(2) : trimmed
	return withoutPrefix.toLowerCase()
}

const createProposalFormObjectSchema = z.object({
	actionType: z.enum(['vk_update', 'signer_update']),
	seqNo: z.string(),
	title: z.string().max(512, 'Title must be at most 512 characters'),
	keysToAdd: z.array(keyRowSchema),
	keysToRemove: z.array(keyRowSchema),
	threshold: z.string(),
	newVkHex: z.string(),
})

export type CreateProposalFormValues = z.infer<typeof createProposalFormObjectSchema>

/** Signers after applying removals (trimmed pubkey match) and additions; duplicates collapse. */
export function countSignersAfterUpdate(
	currentSigners: string[],
	keysToRemove: { value: string }[],
	keysToAdd: { value: string }[],
): number {
	const removeSet = new Set(keysToRemove.map((r) => r.value.trim()).filter((v) => v.length > 0))
	const remaining = currentSigners.filter((s) => !removeSet.has(s.trim()))
	const added = keysToAdd.map((r) => r.value.trim()).filter((v) => v.length > 0)
	return new Set([...remaining.map((s) => s.trim()), ...added]).size
}

export type BuildCreateProposalFormSchemaArgs = {
	/** When null (config not loaded), threshold vs. signer count is not validated. */
	currentMultisigSigners: string[] | null
}

export function buildCreateProposalFormSchema({ currentMultisigSigners }: BuildCreateProposalFormSchemaArgs) {
	return createProposalFormObjectSchema.superRefine((data, ctx) => {
		if (data.keysToAdd.length < 1) {
			ctx.addIssue({ code: 'custom', path: ['keysToAdd'], message: 'At least one row for keys to add' })
		}
		if (data.keysToRemove.length < 1) {
			ctx.addIssue({ code: 'custom', path: ['keysToRemove'], message: 'At least one row for keys to remove' })
		}

		const seqNoTrim = data.seqNo.trim()
		if (seqNoTrim.length === 0) {
			ctx.addIssue({ code: 'custom', path: ['seqNo'], message: 'Sequence number is required' })
		} else if (!/^\d+$/.test(seqNoTrim)) {
			ctx.addIssue({ code: 'custom', path: ['seqNo'], message: 'Must be a non-negative integer' })
		} else {
			const seqNo = Number(seqNoTrim)
			if (!Number.isInteger(seqNo) || seqNo < 0) {
				ctx.addIssue({
					code: 'custom',
					path: ['seqNo'],
					message: 'Sequence number must be a valid non-negative integer',
				})
			}
		}

		if (data.actionType === 'signer_update') {
			const normalizedAdds = data.keysToAdd.map((row) => row.value.trim()).filter((value) => value.length > 0)
			const normalizedRemoves = data.keysToRemove.map((row) => row.value.trim()).filter((value) => value.length > 0)

			if (normalizedAdds.length === 0 && normalizedRemoves.length === 0) {
				ctx.addIssue({
					code: 'custom',
					path: ['keysToAdd'],
					message: 'Provide at least one signer key to add or remove',
				})
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
				const removedSet = new Set(data.keysToRemove.map((row) => row.value.trim()).filter((value) => value.length > 0))
				const remainingCurrentSigners = currentMultisigSigners.filter((signer) => !removedSet.has(signer.trim())).length
				const addedSet = new Set(data.keysToAdd.map((row) => row.value.trim()).filter((value) => value.length > 0))
				const addedSignersNotRemoved = Array.from(addedSet).filter((signer) => !removedSet.has(signer)).length
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
		} else if (data.newVkHex.trim().length === 0) {
			ctx.addIssue({ code: 'custom', path: ['newVkHex'], message: 'New verification key is required' })
		}
	})
}
