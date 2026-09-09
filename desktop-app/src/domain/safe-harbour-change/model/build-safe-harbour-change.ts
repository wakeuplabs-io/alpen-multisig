/** One destination, in the two forms a signer needs: the one they recognise and the one they verify. */
export type SafeHarbourDestination = {
	/** Bech32m address on the active network. Empty when it could not be rendered. */
	address: string
	/** BOSD descriptor hex — what the signer's device displays, and therefore what they compare. */
	addressHex: string
}

export type SafeHarbourChange = {
	/** The destination being replaced. Null once the rotation is enacted: see below. */
	from: SafeHarbourDestination | null
	to: SafeHarbourDestination
}

export type BuildSafeHarbourChangeInput = {
	/** The bridge's live destination, or null when it could not be read from chain. */
	installed: SafeHarbourDestination | null
	/** The destination this action carries, decoded from its hex. */
	proposed: SafeHarbourDestination
	isEnacted: boolean
}

/**
 * Shapes the destination change shown on the detail view and the cancel screen — the two surfaces
 * where a signer who never opened the create form has to decide.
 *
 * Returns `null` when the installed destination could not be read, rather than degrading to the
 * proposed one alone. On a screen whose whole point is *this replaces that*, a lone address with no
 * stated role reads as either of the two, and the reader cannot tell which. No section is the
 * honest answer: the comparison is unavailable, so it is not offered.
 *
 * `installed` is the bridge's **live** destination, which is why enactment changes the shape rather
 * than only the labels: once the rotation applied, the live value *is* what it installed, so a
 * Before/After would print the same address twice and read as a rotation that did nothing. The
 * pre-rotation destination is not recoverable from the action, so it is not invented —
 * `build-signer-set-change.ts` draws the same line for the same reason.
 */
export function buildSafeHarbourChange(input: BuildSafeHarbourChangeInput): SafeHarbourChange | null {
	const { installed, proposed, isEnacted } = input
	if (installed === null) return null
	if (isEnacted) return { from: null, to: proposed }
	return { from: installed, to: proposed }
}
