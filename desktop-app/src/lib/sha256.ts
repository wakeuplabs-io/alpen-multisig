/**
 * Lowercase hex SHA-256 of a UTF-8 string, via Web Crypto.
 *
 * This is the "Message hash" a Ledger Bitcoin app shows for `sign_message`: a single
 * SHA-256 of the raw message bytes (no `"Bitcoin Signed Message:"` magic prefix). It is
 * NOT the BIP-137 sighash that is actually signed/verified — it is only the on-screen
 * content identifier the signer compares against the device.
 */
export async function sha256Hex(text: string): Promise<string> {
	const bytes = new TextEncoder().encode(text)
	const digest = await crypto.subtle.digest('SHA-256', bytes)
	return Array.from(new Uint8Array(digest))
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('')
}
