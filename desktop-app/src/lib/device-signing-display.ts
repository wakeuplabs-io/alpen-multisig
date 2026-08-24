import type { WalletVendor } from '@/wallet/types'

/**
 * What the connected signer actually shows on screen for a message signature, so the UI
 * can present the same value for the signer to compare — instead of the BIP-137 sighash,
 * which no device displays.
 *
 * - `hash-and-text`: a Ledger renders either the message text or `sha256(message)` as its
 *   "Message hash", and **which one is not predictable from here**: it turns on the model, the
 *   Bitcoin app version and the message itself. The login challenge and the certificate are
 *   printable ASCII and render as text (measured, `issues/evidence/G10-B0-CHALLENGE-MEASUREMENT.md`);
 *   the governance message is built with embedded newlines by the ASM subprotocol crate and renders
 *   as a hash on a Nano S+ with app 2.4.2, while the client measured full text for it on a Nano X
 *   (#420). Hence both values, always, with the signer comparing whichever appears — the resolution
 *   #420 settled on.
 * - `text`: Trezor renders the canonical message text.
 * - `none`: software signer (mnemonic/mock) — there is no device screen to compare.
 */
export type DeviceSigningDisplay =
	| { kind: 'hash-and-text'; deviceLabel: string; hash: string; text: string }
	| { kind: 'text'; deviceLabel: string; value: string }
	| { kind: 'none' }

export function deviceSigningDisplay(
	vendor: WalletVendor,
	input: { message: string | null; messageHash: string | null },
): DeviceSigningDisplay {
	if (vendor === 'ledger') {
		// Both values are required: a partial prompt would push the signer to compare against
		// whichever one happens to have resolved, which may not be the one on their screen.
		// The hash is upper-cased because the device prints it with "%02X" — the signer is
		// comparing two strings character by character, so ours must look like the device's.
		return input.message && input.messageHash
			? { kind: 'hash-and-text', deviceLabel: 'Ledger', hash: input.messageHash.toUpperCase(), text: input.message }
			: { kind: 'none' }
	}
	if (vendor === 'trezor') {
		return input.message ? { kind: 'text', deviceLabel: 'Trezor', value: input.message } : { kind: 'none' }
	}
	return { kind: 'none' }
}
