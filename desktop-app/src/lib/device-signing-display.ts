import type { WalletVendor } from '@/wallet/types'

/**
 * What the connected signer actually shows on screen for a message signature, so the UI
 * can present the same value for the signer to compare — instead of the BIP-137 sighash,
 * which no device displays.
 *
 * - `hash`: Ledger renders `sha256(message)` as its "Message hash".
 * - `text`: Trezor renders the canonical message text.
 * - `none`: software signer (mnemonic/mock) — there is no device screen to compare.
 */
export type DeviceSigningDisplay =
	| { kind: 'hash'; deviceLabel: string; value: string }
	| { kind: 'text'; deviceLabel: string; value: string }
	| { kind: 'none' }

export function deviceSigningDisplay(
	vendor: WalletVendor,
	input: { message: string | null; messageHash: string | null },
): DeviceSigningDisplay {
	if (vendor === 'ledger') {
		return input.messageHash ? { kind: 'hash', deviceLabel: 'Ledger', value: input.messageHash } : { kind: 'none' }
	}
	if (vendor === 'trezor') {
		return input.message ? { kind: 'text', deviceLabel: 'Trezor', value: input.message } : { kind: 'none' }
	}
	return { kind: 'none' }
}
