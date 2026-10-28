import { normalizeAdminId } from '@/lib/admin-id'

export type SignerRow = {
	/** The signer's Admin ID, normalized: the ASM lists signers by Admin ID, and so do decoded actions. */
	adminId: string
	inBefore: boolean
	inAfter: boolean
	isAdded: boolean
	isRemoved: boolean
}

export type SignerSetChange = {
	rows: SignerRow[]
	thresholdBefore: number | null
	thresholdAfter: number
}

export type BuildSignerSetChangeInput = {
	/** The target's current signer set as Admin IDs — pre-rotation, unless `isEnacted`. */
	signers: string[]
	/** The target's current threshold — pre-rotation, unless `isEnacted`. */
	threshold: number
	addKeys: string[]
	removeKeys: string[]
	newThreshold: number
	/**
	 * `true` reconstructs the before state from the current post-rotation config. The historical
	 * threshold remains unknown because it cannot be reconstructed from the enacted action.
	 */
	isEnacted: boolean
}

/**
 * Shapes the canonical signer-set diff shared by creation, detail, manual, and cancellation
 * surfaces. Callers remain responsible for supplying the action's target authority config.
 */
export function buildSignerSetChange(input: BuildSignerSetChangeInput): SignerSetChange {
	const { signers, threshold, addKeys, removeKeys, newThreshold, isEnacted } = input
	const addSet = new Set(addKeys.map(normalizeAdminId))
	const removeSet = new Set(removeKeys.map(normalizeAdminId))

	const beforeSigners = isEnacted
		? [...signers.filter((id) => !addSet.has(normalizeAdminId(id))), ...removeKeys]
		: signers
	const afterSigners = isEnacted
		? signers
		: [...signers.filter((id) => !removeSet.has(normalizeAdminId(id))), ...addKeys]

	const rowMap = new Map<string, SignerRow>()
	for (const adminId of beforeSigners) {
		const normalized = normalizeAdminId(adminId)
		rowMap.set(normalized, {
			adminId: normalized,
			inBefore: true,
			inAfter: false,
			isAdded: false,
			isRemoved: false,
		})
	}
	for (const adminId of afterSigners) {
		const normalized = normalizeAdminId(adminId)
		const existing = rowMap.get(normalized)
		if (existing) {
			existing.inAfter = true
		} else {
			rowMap.set(normalized, {
				adminId: normalized,
				inBefore: false,
				inAfter: true,
				isAdded: true,
				isRemoved: false,
			})
		}
	}
	for (const row of rowMap.values()) {
		row.isRemoved = row.inBefore && !row.inAfter
		row.isAdded = !row.inBefore && row.inAfter
	}

	return {
		rows: Array.from(rowMap.values()),
		thresholdBefore: isEnacted ? null : threshold,
		thresholdAfter: isEnacted ? threshold : newThreshold,
	}
}
