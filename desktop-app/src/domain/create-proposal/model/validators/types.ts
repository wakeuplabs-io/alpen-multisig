import type { z } from 'zod'
import type { CreateProposalFormValues } from '../create-proposal.schema'
import { normalizeSignerKey } from '../create-proposal.schema'

export type ActionValidatorContext = {
	data: CreateProposalFormValues
	ctx: z.RefinementCtx
	currentMultisigSigners: string[] | null
}

export type ActionValidator = (context: ActionValidatorContext) => void

export const compressedPubKeyHexPattern = /^(?:0x)?(?:02|03)[0-9a-fA-F]{64}$/
export const EVEN_PUBKEY_HEX_PATTERN = /^[0-9a-fA-F]{64}$/
export const U32_MAX = 4_294_967_295

export type ValidateSignerKeyArgs = {
	key: string
	currentSigners: string[]
	existingKeysToAdd: { value: string }[]
	keysToRemove: { value: string }[]
}

export function validateSignerKeyInput({
	key,
	currentSigners,
	existingKeysToAdd,
	keysToRemove,
}: ValidateSignerKeyArgs): string | null {
	const trimmed = key.trim()
	if (trimmed.length === 0) return 'Signer key is required'
	if (!compressedPubKeyHexPattern.test(trimmed)) {
		return 'Signer key must be compressed pubkey hex (33 bytes, 02/03..., optional 0x)'
	}

	const normalized = normalizeSignerKey(trimmed)

	const isDuplicateInAdd = existingKeysToAdd.some(
		(r) => r.value.trim().length > 0 && normalizeSignerKey(r.value) === normalized,
	)
	if (isDuplicateInAdd) return 'Duplicate signer key'

	const removedSet = new Set(
		keysToRemove
			.map((r) => r.value.trim())
			.filter((v) => v.length > 0)
			.map(normalizeSignerKey),
	)

	const existsInCurrent = currentSigners.some((s) => normalizeSignerKey(s) === normalized)
	if (existsInCurrent && !removedSet.has(normalized)) {
		return 'Signer already exists in the current set'
	}

	return null
}
