import { useAdminIds } from '@/hooks/use-admin-ids'
import { normalizePubkey } from '@/lib/pubkey'
import type { WalletAccountInfo } from '@/wallet/types'

/**
 * The session's own Admin ID in its two renderings. Both carry the same witness program; only the HRP differs.
 *
 * A device names the address under the coin type it derives with, not under the chain: a Trezor shows `bc1q…` on
 * every network, and the mnemonic signer derives on mainnet. Every other signer the app lists is on the chain's HRP,
 * and the add-signer field only accepts that one, so an Admin ID a signer copies has to be on it too.
 */
export type SessionAdminId = {
	/** On the chain's network: what the app shows and copies. `undefined` while it resolves. */
	display: string | undefined
	/** As the device renders it: on a hardware session, what Verify compares and what the certificate signs. */
	device: string | undefined
}

/**
 * Picks the chain rendering for `publicKeyHex` out of the resolved Admin IDs. Without a key there is nothing to
 * resolve, and a key that has no Admin ID falls back to the device's own rendering rather than to nothing.
 */
export function sessionAdminId(
	device: string | undefined,
	publicKeyHex: string | undefined,
	resolved: ReadonlyMap<string, string>,
): SessionAdminId {
	if (!publicKeyHex) return { display: device ?? '', device }
	const chain = resolved.get(normalizePubkey(publicKeyHex))
	if (chain === undefined) return { display: undefined, device }
	return { display: chain === '' ? (device ?? '') : chain, device }
}

export function useSessionAdminId(
	account: Pick<WalletAccountInfo, 'addressSample' | 'publicKeyHex'> | null | undefined,
): SessionAdminId {
	const publicKeyHex = account?.publicKeyHex
	const resolved = useAdminIds(publicKeyHex ? [publicKeyHex] : [])
	return sessionAdminId(account?.addressSample, publicKeyHex, resolved)
}
