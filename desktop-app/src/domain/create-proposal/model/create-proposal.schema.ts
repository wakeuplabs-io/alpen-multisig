import { z } from 'zod'
import { VK_PREDICATE_TYPES } from '@/lib/vk-predicate'
import { getActionTypeOptions } from './action-type-config'
import { getActionValidator } from './validators'

export type { VkPredicateType } from '@/lib/vk-predicate'
export { VK_PREDICATE_TYPES, VK_PREDICATE_TYPE_IDS, VK_PREDICATE_TYPE_LABELS } from '@/lib/vk-predicate'

const keyRowSchema = z.object({
	value: z.string(),
})

export function normalizeSignerKey(value: string): string {
	const trimmed = value.trim()
	const withoutPrefix = trimmed.startsWith('0x') || trimmed.startsWith('0X') ? trimmed.slice(2) : trimmed
	return withoutPrefix.toLowerCase()
}

const createProposalFormObjectSchema = z.object({
	actionType: z.enum(['vk_update', 'signer_update', 'operator_set_update', 'sequencer_key_update', 'defcon_1']),
	seqNo: z.string(),
	title: z.string().max(512, 'Title must be at most 512 characters'),
	keysToAdd: z.array(keyRowSchema),
	keysToRemove: z.array(keyRowSchema),
	threshold: z.string(),
	vkTypeId: z.enum(VK_PREDICATE_TYPES),
	newVkHex: z.string(),
	operatorsToAdd: z.array(keyRowSchema),
	operatorIndicesToRemove: z.array(keyRowSchema),
	newSequencerKeyHex: z.string(),
	defconConfirm: z.string(),
	/** The canonical signing message, resolved from Rust and mirrored here so that
	 * "the signer can see what they are signing" gates submission like any other field. */
	defconMessage: z.string(),
})

export type CreateProposalFormValues = z.infer<typeof createProposalFormObjectSchema>

export function countSignersAfterUpdate(
	currentSigners: string[],
	keysToRemove: { value: string }[],
	keysToAdd: { value: string }[],
): number {
	const removeSet = new Set(
		keysToRemove
			.map((r) => r.value.trim())
			.filter((v) => v.length > 0)
			.map(normalizeSignerKey),
	)
	const remaining = currentSigners
		.map((s) => s.trim())
		.filter((s) => s.length > 0)
		.filter((s) => !removeSet.has(normalizeSignerKey(s)))
		.map(normalizeSignerKey)
	const added = keysToAdd
		.map((r) => r.value.trim())
		.filter((v) => v.length > 0)
		.map(normalizeSignerKey)
	return new Set([...remaining, ...added]).size
}

export type BuildCreateProposalFormSchemaArgs = {
	currentMultisigSigners: string[] | null
	/** The session's authority. Decides which action types this form may produce at all. */
	authority: string
}

export function buildCreateProposalFormSchema({
	currentMultisigSigners,
	authority,
}: BuildCreateProposalFormSchemaArgs) {
	return createProposalFormObjectSchema.superRefine((data, ctx) => {
		// The action-type menu is display data. This is the rule: an authority can only draft the
		// actions it is allowed to author, whatever route or stale form state got the value here.
		// The backend refuses the rest too (AC 17), but a signer must never reach a device prompt
		// for an action their authority cannot sign.
		const allowed = getActionTypeOptions(authority).map((option) => option.actionType)
		if (!allowed.includes(data.actionType)) {
			ctx.addIssue({
				code: 'custom',
				path: ['actionType'],
				message: `This authority cannot create a ${data.actionType} proposal.`,
			})
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

		const validate = getActionValidator(data.actionType)
		validate({ data, ctx, currentMultisigSigners })
	})
}
