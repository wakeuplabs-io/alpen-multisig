/**
 * Admin ID Verification Certificate — copy and the shape of the copied block
 * (PRD 06 §3.c.i).
 *
 * The literals are transcribed from the three normative wireframes in
 * `docs/0-prd/assets/`; they live here so both surfaces that render the modal read one
 * source, the way `lib/admin-id.ts` owns the Admin ID copy. Nothing in this module
 * touches cryptography: the certificate is built and verified in Rust.
 */

export const CERTIFICATE_TITLE = 'Generate Admin ID Verification Certificate'

export const CERTIFICATE_STEP_1_HEADING = 'Step 1. Sign Admin ID'
export const CERTIFICATE_STEP_1_HELP =
	'Click the "Sign" button and confirm the signature on your hardware signer to digitally sign your Admin ID and generate your Admin ID Verification Certificate.'
export const CERTIFICATE_WAITING = 'Waiting for signature to generate Admin ID Verification Certificate...'
export const CERTIFICATE_SIGN_BUTTON = 'Sign'
export const CERTIFICATE_SIGNED_CHIP = 'Signed'
export const CERTIFICATE_COPIED = 'Copied to clipboard'

export const CERTIFICATE_STEP_2_HEADING = 'Step 2. Verify Admin ID'
export const CERTIFICATE_STEP_2_HELP =
	'Click the "Verify" button to compare and verify that the Admin ID (in bitcoin address format) that appears on your hardware signer screen matches the signed Admin ID shown above.'
/**
 * Shown instead of an enabled Verify button when the session signs with a mnemonic.
 * The step is disabled with a reason rather than hidden: a step that disappears looks
 * like a broken modal, and the signer never learns why it does not apply to them.
 */
export const CERTIFICATE_STEP_2_NO_DEVICE =
	'This session signs with a mnemonic, so there is no signer screen to compare against. Connect a hardware signer to verify your Admin ID on the device.'

/**
 * The block the copy button puts on the clipboard: the signed message on line 1, the
 * signature on line 2, no labels. Whoever receives it pastes each line as-is into a
 * verifier — a prefix to strip is exactly where a manual verification goes wrong.
 *
 * Returns an empty string until both halves exist: half a certificate verifies as
 * nothing and would only look like the real thing.
 */
export function certificateBlock(message: string, signature: string): string {
	const trimmedMessage = message.trim()
	const trimmedSignature = signature.trim()
	if (!trimmedMessage || !trimmedSignature) return ''
	return `${trimmedMessage}\n${trimmedSignature}`
}
