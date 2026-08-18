/**
 * Admin ID presentation rules — shared across the connect flow and the Admin Wallet.
 *
 * PRD 06 §3.b.ii.2: the Admin ID **is** a P2WPKH bitcoin address derived at
 * `m/84'/0'/73'/0/0`. It authenticates the signer and signs admin subprotocol
 * messages — it must never receive funds and must never sign a bitcoin transaction.
 *
 * Between PR #444 and PRD 06 the app rendered the compressed public key here instead.
 * That is why a raw key is explicitly *not* a displayable Admin ID: the reversion is
 * undone the moment a surface can pass one again and still render.
 *
 * The copy literals live here as the single audited source (mirrors the §4.3.5
 * send-copy pattern); `architecture.test.ts` Rule 9 pins them.
 */

/**
 * P2WPKH shape: a known HRP, the `1` separator, the witness-v0 character `q`, then the
 * 32 data characters of a 20-byte program plus a 6-character checksum. `b`, `i`, `o` and
 * `1` are outside the bech32 charset; case-insensitive per BIP-173.
 *
 * Pinned to **P2WPKH specifically**, not "any segwit address". A guard looser than the
 * contract lets an unrelated output render as a copyable identity — the mock adapter
 * ships a hardcoded taproot address that has nothing to do with the key it signs with.
 *
 * Still a **display guard, not a consensus validator**: the checksum is not verified,
 * because the string comes from the device and the authority on its correctness is the
 * device itself, via verify-on-device. What this stops is the UI labelling a placeholder,
 * a public key or an unrelated address as the signer's identity.
 */
const P2WPKH_ADDRESS_PATTERN = /^(?:bc|tb|bcrt)1q[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{38}$/i

export const ADMIN_ID_LABEL = 'Admin ID'

export const ADMIN_ID_SAFETY_CAPTION = 'For authentication only — never send funds to this address.'

/** Shown wherever an Admin ID is expected but none is displayable. */
export const ADMIN_ID_UNKNOWN = 'Unknown'

/**
 * True when `value` is a real, copyable Admin ID. Narrows the type, so callers that
 * guard on it can render the value without asserting it is present.
 */
export function isDisplayableAdminId(value: string | undefined): value is string {
	if (!value) return false
	return P2WPKH_ADDRESS_PATTERN.test(value.trim())
}

/**
 * The Admin ID as a surface should print it, or `'Unknown'`.
 *
 * One rule, because four surfaces used to answer this question separately and one of them
 * answered it from the public key — so a screen's header chip and the panel that chip
 * opens showed the same identity in two incompatible shapes.
 */
export function adminIdText(value: string | undefined): string {
	return isDisplayableAdminId(value) ? value : ADMIN_ID_UNKNOWN
}

/** The same rule for chips and dense rows, truncated. */
export function adminIdChipLabel(value: string | undefined): string {
	return isDisplayableAdminId(value) ? truncateAdminId(value) : ADMIN_ID_UNKNOWN
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
