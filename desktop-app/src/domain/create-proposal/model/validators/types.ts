import type { z } from 'zod'
import type { CreateProposalFormValues } from '../create-proposal.schema'
import { normalizeSignerKey } from '../create-proposal.schema'
import { compressedPubKeyHexPattern } from '@/lib/pubkey'
import { adminIdHrp } from '@/lib/admin-id'

export type ActionValidatorContext = {
	data: CreateProposalFormValues
	ctx: z.RefinementCtx
	currentMultisigSigners: string[] | null
	currentMultisigThreshold: number | null
	/** The bridge's current safe harbor destination as an address; null when unread. */
	currentSafeHarborAddress: string | null
}

export type ActionValidator = (context: ActionValidatorContext) => void

export { compressedPubKeyHexPattern }

export const ADMIN_ID_IS_PUBKEY_MESSAGE = "Enter the signer's Admin ID (P2WPKH address), not a public key"
export const ADMIN_ID_SHAPE_MESSAGE = 'Admin ID must be a P2WPKH address (bc1q…, tb1q… or bcrt1q…)'

/**
 * The error for one typed Admin ID, or null. AC 3: the field takes a P2WPKH address on the active
 * network, and a compressed public key — the format it used to take — gets its own message.
 *
 * `expectedHrp` is the HRP of the current signer set, which Rust renders on the chain's network;
 * null when that set is unread, and then only the shape is checked here. This is a form check:
 * the checksum and the network are verified again in Rust when the action is built.
 */
export function adminIdInputError(value: string, expectedHrp: string | null): string | null {
	const trimmed = value.trim()
	if (compressedPubKeyHexPattern.test(trimmed)) return ADMIN_ID_IS_PUBKEY_MESSAGE
	const hrp = adminIdHrp(trimmed)
	if (hrp === null) return ADMIN_ID_SHAPE_MESSAGE
	if (expectedHrp !== null && hrp !== expectedHrp) {
		return `Admin ID must be for this chain's network (${expectedHrp}1q…)`
	}
	return null
}

/** The HRP the chain renders Admin IDs with, read off the current signer set. */
export function chainAdminIdHrp(currentSigners: string[] | null): string | null {
	for (const signer of currentSigners ?? []) {
		const hrp = adminIdHrp(signer)
		if (hrp !== null) return hrp
	}
	return null
}
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
	if (trimmed.length === 0) return 'Admin ID is required'
	const inputError = adminIdInputError(trimmed, chainAdminIdHrp(currentSigners))
	if (inputError) return inputError

	const normalized = normalizeSignerKey(trimmed)

	const isDuplicateInAdd = existingKeysToAdd.some(
		(r) => r.value.trim().length > 0 && normalizeSignerKey(r.value) === normalized,
	)
	if (isDuplicateInAdd) return 'Duplicate signer'

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
