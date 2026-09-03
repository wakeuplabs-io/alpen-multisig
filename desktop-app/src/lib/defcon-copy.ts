/**
 * The two Defcon levers, as data.
 *
 * They differ in exactly three things — the confirmation string, the destructive paragraph and
 * the safe-harbour note's wording — and every other part of the create and sign flows is shared.
 * Holding those three as data is what keeps the difference honest: the copy used to be written
 * out by hand in the create form, the preview and the sign view, which is how two levels come to
 * disagree about which one can be cancelled.
 *
 * Defcon 3 must never reuse Defcon 1's *Irreversible* wording (contract Constraint 5). Telling a
 * signer that a cancelable action cannot be cancelled withholds the one lever that could stand
 * the alarm down, and trains them to discount the same warning on Defcon 1, where it is true.
 *
 * No text here names a block count or a delay in hours: the delay is `confirmation_depths.defcon3`
 * read live from the ASM (Constraint 1), so any number written down would be a second source of
 * truth that cannot be kept in step.
 */
export type DefconLevel = 'defcon_1' | 'defcon_3'

export type DefconCopy = {
	/** What the signer must type out, matched case-insensitively and with no trimming. */
	confirmation: string
	menuTitle: string
	menuDescription: string
	calloutTitle: string
	/** The destructive paragraph on the create form and its preview. */
	calloutBody: string
	/** The same warning at the moment of signing, where the signer is already committing. */
	signCalloutBody: string
	/** Shown only when the bridge is already in safe harbour. Told, never enforced. */
	safeHarbourNote: string
	signSafeHarbourNote: string
	/** The last screen before the commit and reveal fees are spent. */
	broadcastSafeHarbourNote: string
}

export const DEFCON_COPY: Record<DefconLevel, DefconCopy> = {
	defcon_1: {
		confirmation: 'DEFCON 1',
		menuTitle: 'DEFCON 1',
		menuDescription: 'Immediately sweep bridge funds to the Safe Harbor.',
		calloutTitle: 'Irreversible',
		calloutBody:
			'DEFCON 1 activates the Safe Harbor sweep immediately, taking effect in the block that the approved proposal is confirmed in. Once approved and confirmed, it cannot be canceled, and is therefore irreversible.',
		signCalloutBody:
			'DEFCON 1 activates the Safe Harbor sweep immediately, taking effect in the block that the approved proposal is confirmed in. Once approved and confirmed, it cannot be canceled, and is therefore irreversible.',
		safeHarbourNote:
			'The bridge is already in safe harbour. Another Defcon 1 does not change that — it consumes a council sequence number, costs fees, and needs a full quorum. Create one only if you have reason to believe this state is wrong.',
		signSafeHarbourNote:
			'The bridge is already in safe harbour. Signing this does not change that — it consumes a council sequence number and needs a full quorum.',
		broadcastSafeHarbourNote:
			'The bridge is already in safe harbour. Sending this does not change that — it consumes a council sequence number and costs the commit and reveal fees.',
	},
	defcon_3: {
		confirmation: 'DEFCON 3',
		menuTitle: 'DEFCON 3',
		menuDescription: 'Sweep bridge funds to the Safe Harbor after a delay. Cancelable until it activates.',
		calloutTitle: 'Delayed and cancelable',
		calloutBody:
			'DEFCON 3 sweeps bridge funds to the Safe Harbor, but not immediately. Once the approved proposal confirms, it is queued for the delay this deployment configures. Until it activates, the council can cancel it. From activation on it cannot be undone.',
		signCalloutBody:
			'Signing this approves a delayed Safe Harbor sweep. Until it activates, the council can cancel it. From activation on it cannot be undone.',
		safeHarbourNote:
			'The bridge is already in safe harbour. A DEFCON 3 does not change that — it consumes a council sequence number, costs fees, needs a full quorum, and waits out its full delay before changing nothing.',
		signSafeHarbourNote:
			'The bridge is already in safe harbour. Signing this does not change that — it waits out its full delay before changing nothing.',
		broadcastSafeHarbourNote:
			'The bridge is already in safe harbour. Sending this does not change that — it costs the commit and reveal fees, then waits out its full delay before changing nothing.',
	},
}

/**
 * The type-to-confirm gate. Case-insensitive and nothing else: the contract's rule is
 * `input.toUpperCase() === "DEFCON <n>"`, and its edge cases pin a trailing space as a rejection —
 * so trimming here would delete the gate's only evidence that the signer read the form.
 *
 * Reading the string from `DEFCON_COPY` is what makes the two gates mutually exclusive by
 * construction rather than by two constants that could drift into each other.
 */
export function matchesDefconConfirmation(level: DefconLevel, input: string): boolean {
	return input.toUpperCase() === DEFCON_COPY[level].confirmation
}

/** Narrows an action type to a Defcon level, for the screens that hold one as loose data. */
export function defconLevelOf(actionType: string | null | undefined): DefconLevel | null {
	return actionType === 'defcon_1' || actionType === 'defcon_3' ? actionType : null
}
