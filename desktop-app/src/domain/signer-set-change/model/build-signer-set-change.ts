import { normalizePubkey } from '@/lib/pubkey'

export type SignerRow = {
	pubkey: string
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
	/** The target's current signer set — pre-rotation, unless `isEnacted`. */
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
	const addSet = new Set(addKeys.map(normalizePubkey))
	const removeSet = new Set(removeKeys.map(normalizePubkey))

	const beforeSigners = isEnacted
		? [...signers.filter((key) => !addSet.has(normalizePubkey(key))), ...removeKeys]
		: signers
	const afterSigners = isEnacted
		? signers
		: [...signers.filter((key) => !removeSet.has(normalizePubkey(key))), ...addKeys]

	const rowMap = new Map<string, SignerRow>()
	for (const pubkey of beforeSigners) {
		const normalized = normalizePubkey(pubkey)
		rowMap.set(normalized, {
			pubkey: normalized,
			inBefore: true,
			inAfter: false,
			isAdded: false,
			isRemoved: false,
		})
	}
	for (const pubkey of afterSigners) {
		const normalized = normalizePubkey(pubkey)
		const existing = rowMap.get(normalized)
		if (existing) {
			existing.inAfter = true
		} else {
			rowMap.set(normalized, {
				pubkey: normalized,
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
