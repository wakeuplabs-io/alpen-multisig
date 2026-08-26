import type { ActionValidator } from './types'

/** What the signer must type out before Defcon 1 can be signed. */
export const DEFCON_1_CONFIRMATION = 'DEFCON 1'

export const DEFCON_1_CONFIRMATION_ERROR = `Type must match '${DEFCON_1_CONFIRMATION}' exactly (case-insensitive).`

/**
 * The type-to-confirm gate. Case-insensitive and nothing else: the contract's rule is
 * `input.toUpperCase() === "DEFCON 1"`, and its edge cases pin a trailing space as a
 * rejection — so trimming here would delete the gate's only evidence that the signer read
 * the form.
 */
export function matchesDefconConfirmation(input: string): boolean {
	return input.toUpperCase() === DEFCON_1_CONFIRMATION
}

export const validateDefcon1: ActionValidator = ({ data, ctx }) => {
	if (!matchesDefconConfirmation(data.defconConfirm)) {
		ctx.addIssue({ code: 'custom', path: ['defconConfirm'], message: DEFCON_1_CONFIRMATION_ERROR })
	}
}
