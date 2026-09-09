import type { ActionValidator } from './types'

/** BOSD descriptor for a P2TR output: type tag `04` followed by a 32-byte x-only key. */
const P2TR_BOSD_HEX_PATTERN = /^04[0-9a-fA-F]{64}$/

export const NO_OP_SAFE_HARBOUR_MESSAGE =
	'This is already the safe harbour destination, so this update would change nothing.'

/**
 * The address itself is validated in Rust — it has to be, since the descriptor the device displays
 * is derived there and bech32m parsing does not check the curve. This validator owns the two rules
 * the form can answer on its own: the field is filled in, and the destination is not the one
 * already installed.
 *
 * The no-op rule is a safety rule rather than hygiene. Upstream accepts a rotation to the current
 * address, consumes the sequence number, drains the queue entry, and the post-condition — which
 * compares addresses — then finds a match, so the proposal reports `Enacted`. On every surface this
 * application has, that is indistinguishable from a rotation that actually moved the destination.
 *
 * When `currentSafeHarbourAddress` is null the rule is off, which is safe only because the form
 * blocks submission while that read is unavailable. The two are load-bearing together.
 */
export const validateSafeHarbourAddressUpdate: ActionValidator = ({ data, ctx, currentSafeHarbourAddress }) => {
	const address = data.newSafeHarbourAddress.trim()
	if (address.length === 0) {
		ctx.addIssue({
			code: 'custom',
			path: ['newSafeHarbourAddress'],
			message: 'New safe harbour address is required',
		})
		return
	}

	// A descriptor hex is not what this field takes, and pasting one is the likely mistake for
	// anyone reading the signing message. Say so, rather than letting Rust answer "not a valid
	// Bitcoin address" about a string that is a perfectly valid destination in another form.
	if (P2TR_BOSD_HEX_PATTERN.test(address)) {
		ctx.addIssue({
			code: 'custom',
			path: ['newSafeHarbourAddress'],
			message: 'Enter the taproot address, not the descriptor hex shown on your device.',
		})
		return
	}

	// Compared as addresses rather than as descriptors: an address is what the signer typed, and
	// within one network the two forms are in one-to-one correspondence, so this answers the same
	// question with the value that is actually in hand. Bech32m is case-insensitive.
	if (currentSafeHarbourAddress !== null && currentSafeHarbourAddress.length > 0) {
		if (address.toLowerCase() === currentSafeHarbourAddress.trim().toLowerCase()) {
			ctx.addIssue({
				code: 'custom',
				path: ['newSafeHarbourAddress'],
				message: NO_OP_SAFE_HARBOUR_MESSAGE,
			})
		}
	}
}
