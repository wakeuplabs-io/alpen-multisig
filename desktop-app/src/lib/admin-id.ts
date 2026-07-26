/**
 * Admin ID presentation rules — shared across the connect flow and the Admin Wallet.
 *
 * The Admin ID **is** the signer's compressed public key (33 bytes, `02`/`03` prefix).
 * It authenticates the signer and signs SPS-65 messages — it is not a Bitcoin address,
 * cannot receive funds, and must never be used as a payment destination.
 *
 * The label and safety-caption literals live here as the single audited source
 * (mirrors the §4.3.5 send-copy pattern); `architecture.test.ts` Rule 9 pins them.
 */

import { isCompressedPubKeyHex } from '@/lib/pubkey'
import { deviceCopy } from '@/lib/device-copy'
import type { WalletVendor } from '@/wallet/types'

/** Connect-flow placeholder used when no real key is available. */
const PLACEHOLDER_SIGNER = 'Mnemonic signer'

export const ADMIN_ID_LABEL = 'Admin ID'

export const ADMIN_ID_SAFETY_CAPTION = 'For authentication only — it is a public key, not a payment address.'

/**
 * True when `value` is a real, copyable Admin ID — a compressed public key.
 * Bitcoin addresses, empty values and the `'Mnemonic signer'` placeholder are not.
 */
export function isDisplayableAdminId(value: string | undefined): boolean {
	if (!value) return false
	const trimmed = value.trim()
	if (trimmed === PLACEHOLDER_SIGNER) return false
	return isCompressedPubKeyHex(trimmed)
}

/**
 * What the connected device actually shows when the signer verifies the Admin ID.
 * Trezor and Ledger can only render addresses (and xpubs), never a raw compressed
 * public key — so the device confirms the key indirectly, via the address derived
 * from the same key and path. See issue #409 for the device-capability findings.
 */
export function adminIdVerifyCaption(vendor: WalletVendor): string {
	return `Your ${deviceCopy(vendor).label} shows the address derived from this key and path — hardware signers cannot display a raw public key.`
}

/** Truncated Admin ID for chips and dense rows (`0279…a41c`). */
export function truncateAdminId(value: string, prefix = 10, suffix = 8): string {
	if (value.length <= prefix + suffix + 1) return value
	return `${value.slice(0, prefix)}…${value.slice(-suffix)}`
}
