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
	 * `true` reconstructs the *before* state from the *current* (post-rotation) config minus the
	 * added keys, rather than treating `signers`/`threshold` as the before state directly. This is
	 * correct only because enactment compares post-conditions against the target's config
	 * (Constraint 1, V3 Phase 2) — if enactment ever fell back to a seqno-shaped answer instead,
	 * this reconstruction would invert silently.
	 */
	isEnacted: boolean
}

/**
 * The pure Before/After computation shared by `useDecodedProposal` (the detail view) and
 * `useManualProposal` (`/manual`) — extracted per SDD §4.10 because both hooks built the same
 * `SignerRow[]` from the same inputs, one with an `isEnacted` branch and one without.
 *
 * Callers decide *which* config (proposal authority or retargeted target) and *which* decoded
 * action feed this — this function only shapes rows and thresholds from whatever it is handed.
 */
export function buildSignerSetChange(input: BuildSignerSetChangeInput): SignerSetChange {
	const { signers, threshold, addKeys, removeKeys, newThreshold, isEnacted } = input

	const addSet = new Set(addKeys.map((k) => k.toLowerCase()))
	const removeSet = new Set(removeKeys.map((k) => k.toLowerCase()))

	let beforeSigners: string[]
	let afterSigners: string[]

	if (isEnacted) {
		afterSigners = signers
		beforeSigners = [...signers.filter((k) => !addSet.has(k.toLowerCase())), ...removeKeys]
	} else {
		beforeSigners = signers
		afterSigners = [...signers.filter((k) => !removeSet.has(k.toLowerCase())), ...addKeys]
	}

	const rowMap = new Map<string, SignerRow>()
	for (const k of beforeSigners) {
		rowMap.set(k.toLowerCase(), { pubkey: k, inBefore: true, inAfter: false, isAdded: false, isRemoved: false })
	}
	for (const k of afterSigners) {
		const lower = k.toLowerCase()
		const existing = rowMap.get(lower)
		if (existing) {
			existing.inAfter = true
		} else {
			rowMap.set(lower, { pubkey: k, inBefore: false, inAfter: true, isAdded: true, isRemoved: false })
		}
	}
	for (const row of rowMap.values()) {
		if (row.inBefore && !row.inAfter) row.isRemoved = true
		if (!row.inBefore && row.inAfter) row.isAdded = true
	}

	return {
		rows: Array.from(rowMap.values()),
		thresholdBefore: isEnacted ? null : threshold,
		thresholdAfter: isEnacted ? threshold : newThreshold,
	}
}
