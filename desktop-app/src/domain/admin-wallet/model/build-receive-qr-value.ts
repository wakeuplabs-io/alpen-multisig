/**
 * Map a receive address to the exact string encoded in the QR code and copied
 * when the user clicks the QR or the address text (PRD §4.3.4.1 / §4.3.4.1.1).
 *
 * The payload is the **bare address** — deliberately NOT a `bitcoin:` BIP-21
 * URI — so that "clicking the QR copies the address" yields exactly the address
 * the user sees. Surrounding whitespace is trimmed; empty input yields an empty
 * string so the caller can guard rendering.
 */
export function buildReceiveQrValue(address: string): string {
	return address.trim()
}
