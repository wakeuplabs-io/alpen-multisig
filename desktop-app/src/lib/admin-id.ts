/**
 * Admin ID presentation rules — shared across the connect flow and the Admin Wallet.
 *
 * PRD 06 §3.b.ii.2: the Admin ID **is** a P2WPKH bitcoin address derived at
 * `m/84'/0'/73'/0/0`. It authenticates the signer and signs admin subprotocol
 * messages — it must never receive funds and must never sign a bitcoin transaction.
 *
 * This module is in the **expand** step of a parallel change (spec:
 * `docs/specs/admin-id-as-bitcoin-address.md`). Every surface still passes the
 * compressed public key it was given in July, so both shapes are accepted and the
 * captions are chosen from the shape of the value instead of being one constant.
 * Surfaces migrate one commit at a time; the contract step then drops the key branch.
 *
 * The copy literals live here as the single audited source (mirrors the §4.3.5
 * send-copy pattern); `architecture.test.ts` Rule 9 pins them.
 */

import { isCompressedPubKeyHex } from '@/lib/pubkey'
import { deviceCopy } from '@/lib/device-copy'
import type { WalletVendor } from '@/wallet/types'

/** Connect-flow placeholder used when no real key is available. */
const PLACEHOLDER_SIGNER = 'Mnemonic signer'

/**
 * Segwit address shape: a known HRP, the `1` separator, then bech32 data characters
 * (`b`, `i`, `o` and `1` are outside the charset). Case-insensitive per BIP-173.
 *
 * This is a **display guard, not a consensus validator**: it stops the UI from
 * labelling a placeholder or a stray string as an identity, nothing more. The
 * authority on the Admin ID's correctness is the device, via verify-on-device.
 */
const BECH32_ADDRESS_PATTERN = /^(?:bc|tb|bcrt)1[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{11,}$/i

export const ADMIN_ID_LABEL = 'Admin ID'

/** What the Admin ID is, for the surfaces that have not migrated yet. */
export type AdminIdShape = 'address' | 'pubkey' | 'none'

/** Classify an Admin ID value; `'none'` means there is nothing displayable. */
export function adminIdShape(value: string | undefined): AdminIdShape {
	if (!value) return 'none'
	const trimmed = value.trim()
	if (trimmed === PLACEHOLDER_SIGNER) return 'none'
	if (BECH32_ADDRESS_PATTERN.test(trimmed)) return 'address'
	if (isCompressedPubKeyHex(trimmed)) return 'pubkey'
	return 'none'
}

/**
 * True when `value` is a real, copyable Admin ID. Narrows the type, so callers that
 * guard on it can render the value without asserting it is present.
 */
export function isDisplayableAdminId(value: string | undefined): value is string {
	return adminIdShape(value) !== 'none'
}

/**
 * The signer-safety warning, chosen from what the value actually is. An address that
 * must never receive funds and a public key that cannot receive them at all are
 * different warnings, and showing one against the other would be a lie on a caption
 * the signer is meant to rely on. Unknown values get the address wording: it is both
 * the requirement and the stricter of the two.
 */
export function adminIdSafetyCaption(value: string | undefined): string {
	return adminIdShape(value) === 'pubkey'
		? 'For authentication only — it is a public key, not a payment address.'
		: 'For authentication only — never send funds to this address.'
}

/**
 * What the connected device actually shows when the signer verifies the Admin ID.
 *
 * With the Admin ID as an address the device renders the Admin ID **itself**, which is
 * what makes the comparison direct. While a surface still shows the compressed key, the
 * device can only confirm it indirectly — no supported signer can render a raw public
 * key — so the caption keeps saying so until that surface migrates. See #409.
 */
export function adminIdVerifyCaption(vendor: WalletVendor, adminId: string | undefined): string {
	const device = deviceCopy(vendor).label
	return adminIdShape(adminId) === 'pubkey'
		? `Your ${device} shows the address derived from this key and path — hardware signers cannot display a raw public key.`
		: `Your ${device} shows this Admin ID itself, derived from the seed it holds.`
}

/**
 * True when the address the device rendered is the one the app expects for the same
 * key and path. Bech32 is case-insensitive (BIP-173) and devices may pad the string,
 * so both sides are trimmed and lowercased before comparing.
 */
export function matchesDeviceAddress(expected: string, shownOnDevice: string): boolean {
	const normalize = (value: string) => value.trim().toLowerCase()
	const left = normalize(expected)
	return left.length > 0 && left === normalize(shownOnDevice)
}

/** Truncated Admin ID for chips and dense rows (`bc1qm0fnq…r72dphza`). */
export function truncateAdminId(value: string, prefix = 10, suffix = 8): string {
	if (value.length <= prefix + suffix + 1) return value
	return `${value.slice(0, prefix)}…${value.slice(-suffix)}`
}
