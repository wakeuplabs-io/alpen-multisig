import type { WalletVendor } from '@/wallet/types'

/**
 * What the connected signer actually shows on screen for a message signature, so the UI
 * can present the same value for the signer to compare — instead of the BIP-137 sighash,
 * which no device displays.
 *
 * - `hash-and-text`: Ledger renders the message text for every message the app asks it to sign —
 *   all of them are printable ASCII, which is what its Bitcoin app needs to show text (measured on
 *   app 2.4.2, `issues/evidence/G10-B0-CHALLENGE-MEASUREMENT.md`). Older models and app versions
 *   can still fall back to `sha256(message)` as a "Message hash", and the app cannot tell which in
 *   advance, so both values are offered and the signer compares whichever one appears.
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
