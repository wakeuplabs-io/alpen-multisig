import { z } from 'zod'

/** Row shape so `useFieldArray` is typed (RHF excludes primitive `string[]` from `FieldArrayPath`). */
const keyRowSchema = z.object({
	value: z.string(),
})

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
				if (thN > resultingSignerCount) {
					ctx.addIssue({
						code: 'custom',
						path: ['threshold'],
						message: `Threshold cannot be greater than the number of signers after this update (${resultingSignerCount}).`,
					})
				}
			}
		} else if (data.newVkHex.trim().length === 0) {
			ctx.addIssue({ code: 'custom', path: ['newVkHex'], message: 'New verification key is required' })
		}
	})
}
