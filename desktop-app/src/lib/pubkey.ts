/** Compressed pubkey hex: 33 bytes, `02`/`03` prefix, optional `0x`. */
export const compressedPubKeyHexPattern = /^(?:0x)?(?:02|03)[0-9a-fA-F]{64}$/

/** Truncated pubkey for dense rows (`02a1b2c3d4e5…9f8e7d6c`). */
export function truncatePubkey(pubkey: string): string {
	return `${pubkey.slice(0, 12)}…${pubkey.slice(-8)}`
}
