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

/** The HRP of a P2WPKH Admin ID (`bc`, `tb`, `bcrt`), lowercased; null when it is not one. */
export function adminIdHrp(value: string): string | null {
	const trimmed = value.trim()
	if (!P2WPKH_ADDRESS_PATTERN.test(trimmed)) return null
	return trimmed.slice(0, trimmed.lastIndexOf('1')).toLowerCase()
}

/** Canonical form for comparing Admin IDs: bech32 is case-insensitive (BIP-173). */
export function normalizeAdminId(value: string): string {
	return value.trim().toLowerCase()
}

/** Whether `adminId` is one of `adminIds`, compared in canonical form. An empty ID is in no set. */
export function includesAdminId(adminIds: readonly string[], adminId: string): boolean {
	const target = normalizeAdminId(adminId)
	return target !== '' && adminIds.some((id) => normalizeAdminId(id) === target)
}

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

export type AdminIdVariant = 'truncated' | 'full'

/**
 * The text a signer row prints: its Admin ID, `…` while it resolves, `Unknown` when there is none. Never the public
 * key: a signer is identified by its Admin ID wherever the app lists one.
 *
 * The session's own Admin ID goes through the same rule. Surfaces that each answered this separately are how a
 * screen's header chip once showed the public key while the panel it opens showed the address.
 */
export function adminIdDisplay(adminId: string | undefined, variant: AdminIdVariant): string {
	if (adminId === undefined) return '…'
	if (!isDisplayableAdminId(adminId)) return ADMIN_ID_UNKNOWN
	return variant === 'full' ? adminId : truncateAdminId(adminId)
}
