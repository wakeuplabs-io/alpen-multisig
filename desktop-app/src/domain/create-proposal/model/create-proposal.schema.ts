import { z } from 'zod'
import type { VkPredicateType } from '@/lib/vk-predicate'
import { VK_PREDICATE_TYPES, VK_PREDICATE_TYPE_LABELS } from '@/lib/vk-predicate'

export type { VkPredicateType } from '@/lib/vk-predicate'
export { VK_PREDICATE_TYPES, VK_PREDICATE_TYPE_IDS, VK_PREDICATE_TYPE_LABELS } from '@/lib/vk-predicate'

/** Row shape so `useFieldArray` is typed (RHF excludes primitive `string[]` from `FieldArrayPath`). */
const keyRowSchema = z.object({
	value: z.string(),
})

const compressedPubKeyHexPattern = /^(?:0x)?(?:02|03)[0-9a-fA-F]{64}$/

/** Canonical form for comparing compressed pubkeys (matches RPC `hex::encode` / signed payload). */
export function normalizeSignerKey(value: string): string {
	const trimmed = value.trim()
	const withoutPrefix = trimmed.startsWith('0x') || trimmed.startsWith('0X') ? trimmed.slice(2) : trimmed
	return withoutPrefix.toLowerCase()
}

/** condition hex length in chars (2 chars per byte) */
const VK_CONDITION_HEX_LENGTH: Partial<Record<VkPredicateType, number>> = {
	bip340_schnorr: 64, // 32 bytes — x-only pubkey
	sp1_groth16: 712, // 356 bytes — gnark VK compressed
}

const createProposalFormObjectSchema = z.object({
	actionType: z.enum(['vk_update', 'signer_update', 'operator_set_update', 'sequencer_key_update']),
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
})

export type CreateProposalFormValues = z.infer<typeof createProposalFormObjectSchema>

/** Signers after applying removals (trimmed pubkey match) and additions; duplicates collapse. */
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
	/** When null (config not loaded), threshold vs. signer count is not validated. */
	currentMultisigSigners: string[] | null
}

const EVEN_PUBKEY_HEX_PATTERN = /^[0-9a-fA-F]{64}$/
const U32_MAX = 4_294_967_295

export function buildCreateProposalFormSchema({ currentMultisigSigners }: BuildCreateProposalFormSchemaArgs) {
	return createProposalFormObjectSchema.superRefine((data, ctx) => {
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
					// Key already exists — only flag if it's not being removed in this same update
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
		} else if (data.actionType === 'operator_set_update') {
			const filledAdd = data.operatorsToAdd.filter((r) => r.value.trim().length > 0)
			const filledRemove = data.operatorIndicesToRemove.filter((r) => r.value.trim().length > 0)
			if (filledAdd.length === 0 && filledRemove.length === 0) {
				ctx.addIssue({
					code: 'custom',
					path: ['operatorsToAdd'],
					message: 'At least one operator must be added or removed',
				})
			}

			const addKeyIndexes = new Map<string, number[]>()
			for (const [index, row] of data.operatorsToAdd.entries()) {
				const key = row.value.trim()
				if (key.length === 0) continue
				if (!EVEN_PUBKEY_HEX_PATTERN.test(key)) {
					ctx.addIssue({
						code: 'custom',
						path: ['operatorsToAdd', index, 'value'],
						message: 'Operator key must be an x-only pubkey hex (32 bytes, 64 hex chars, no 02/03 prefix)',
					})
				}
				const normalized = key.toLowerCase()
				const indexes = addKeyIndexes.get(normalized) ?? []
				indexes.push(index)
				addKeyIndexes.set(normalized, indexes)
			}
			for (const indexes of addKeyIndexes.values()) {
				if (indexes.length < 2) continue
				for (const index of indexes) {
					ctx.addIssue({ code: 'custom', path: ['operatorsToAdd', index, 'value'], message: 'Duplicate operator key' })
				}
			}

			const removeIndexTracker = new Map<number, number[]>()
			for (const [index, row] of data.operatorIndicesToRemove.entries()) {
				const raw = row.value.trim()
				if (raw.length === 0) continue
				const n = Number(raw)
				if (!Number.isInteger(n) || n < 0 || n > U32_MAX) {
					ctx.addIssue({
						code: 'custom',
						path: ['operatorIndicesToRemove', index, 'value'],
						message: `Operator index must be a non-negative integer (0–${U32_MAX})`,
					})
					continue
				}
				const positions = removeIndexTracker.get(n) ?? []
				positions.push(index)
				removeIndexTracker.set(n, positions)
			}
			for (const positions of removeIndexTracker.values()) {
				if (positions.length < 2) continue
				for (const index of positions) {
					ctx.addIssue({
						code: 'custom',
						path: ['operatorIndicesToRemove', index, 'value'],
						message: 'Duplicate operator index',
					})
				}
			}
		} else if (data.actionType === 'sequencer_key_update') {
			const key = data.newSequencerKeyHex.trim()
			if (key.length === 0) {
				ctx.addIssue({ code: 'custom', path: ['newSequencerKeyHex'], message: 'New sequencer key is required' })
			} else if (!EVEN_PUBKEY_HEX_PATTERN.test(key)) {
				ctx.addIssue({
					code: 'custom',
					path: ['newSequencerKeyHex'],
					message: 'Sequencer key must be an x-only pubkey hex (32 bytes, 64 hex chars, no 02/03 prefix)',
				})
			}
		} else {
			const expectedLen = VK_CONDITION_HEX_LENGTH[data.vkTypeId]
			if (expectedLen !== undefined) {
				const hex = data.newVkHex.trim()
				if (hex.length === 0) {
					ctx.addIssue({
						code: 'custom',
						path: ['newVkHex'],
						message: `Condition hex is required for ${VK_PREDICATE_TYPE_LABELS[data.vkTypeId]}`,
					})
				} else if (!/^[0-9a-fA-F]+$/.test(hex)) {
					ctx.addIssue({ code: 'custom', path: ['newVkHex'], message: 'Must be a valid hex string' })
				} else if (hex.length !== expectedLen) {
					ctx.addIssue({
						code: 'custom',
						path: ['newVkHex'],
						message: `${VK_PREDICATE_TYPE_LABELS[data.vkTypeId]} requires exactly ${expectedLen / 2} bytes (${expectedLen} hex chars), got ${hex.length / 2}`,
					})
				}
			}
		}
	})
}
