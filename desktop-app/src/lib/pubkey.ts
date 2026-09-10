/** Compressed pubkey hex: 33 bytes, `02`/`03` prefix, optional `0x`. */
export const compressedPubKeyHexPattern = /^(?:0x)?(?:02|03)[0-9a-fA-F]{64}$/

/** Canonical form used when comparing signer pubkeys from forms, payloads, and ASM state. */
export function normalizePubkey(pubkey: string): string {
	const trimmed = pubkey.trim()
	const withoutPrefix = trimmed.startsWith('0x') || trimmed.startsWith('0X') ? trimmed.slice(2) : trimmed
	return withoutPrefix.toLowerCase()
}

/** Truncated pubkey for dense rows (`02a1b2c3d4e5…9f8e7d6c`). */
export function truncatePubkey(pubkey: string): string {
	return `${pubkey.slice(0, 12)}…${pubkey.slice(-8)}`
}
