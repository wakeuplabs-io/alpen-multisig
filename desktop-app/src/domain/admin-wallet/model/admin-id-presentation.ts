/**
 * Presentation rules for the Admin ID (PRD §4.1).
 *
 * The Admin ID is the canonical BIP-84 authentication address surfaced as
 * `wallet.addressSample`. It authenticates the signer and signs SPS-65 messages
 * — it MUST NOT sign Bitcoin transactions or receive funds. The label and
 * safety caption literals live here as the single audited source (mirrors the
 * §4.3.5 send-copy pattern); `architecture.test.ts` Rule 9 pins them.
 */

/** Connect-flow placeholder used when no real address is available. */
const PLACEHOLDER_SIGNER = 'Mnemonic signer'

export const ADMIN_ID_LABEL = 'Admin ID'

export const ADMIN_ID_SAFETY_CAPTION = 'For authentication only — never send funds to this address.'

/**
 * True when `value` is a real, copyable Admin ID address — not missing, empty,
 * or the `'Mnemonic signer'` placeholder.
 */
export function isDisplayableAdminId(value: string | undefined): boolean {
	if (!value) return false
	const trimmed = value.trim()
	return trimmed.length > 0 && trimmed !== PLACEHOLDER_SIGNER
}
