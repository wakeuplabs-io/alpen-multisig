import type { WalletAdapter } from '@/wallet/types.ts'
import type { ApiResult } from '@/types'
import type { MnemonicAdapter } from '@/wallet/mnemonic-adapter.ts'

/**
 * Performs vendor-specific Admin Wallet session init after authentication.
 * - mnemonic: calls walletSessionInit with the mnemonic
 * - trezor/ledger: fetches xpub + masterFingerprint via getAccountXpub + getMasterFingerprint,
 *   calls walletSessionInitWatchOnly with deviceType
 * - mock: skips both inits
 *
 * Failures are non-fatal: callers should warn and continue.
 */
export async function initAdminWalletForAdapter(
	adapter: WalletAdapter,
	walletSessionInitWatchOnly: (input: {
		xpub: string
		masterFingerprint?: number
		deviceType?: string
	}) => Promise<ApiResult<null>>,
	walletSessionInit: (input: { mnemonic: string }) => Promise<ApiResult<null>>,
): Promise<void> {
	if (adapter.vendor === 'mnemonic') {
		const mnemonicAdapter = adapter as unknown as MnemonicAdapter
		const result = await walletSessionInit({ mnemonic: mnemonicAdapter.getMnemonic() })
		if (!result.ok) {
			console.warn('[admin-wallet] session init failed:', result.error)
		}
	} else if (adapter.vendor === 'trezor' || adapter.vendor === 'ledger') {
		if (adapter.getAccountXpub && adapter.getMasterFingerprint) {
			try {
				const [xpub, masterFingerprint] = await Promise.all([adapter.getAccountXpub(), adapter.getMasterFingerprint()])
				const result = await walletSessionInitWatchOnly({
					xpub,
					masterFingerprint,
					deviceType: adapter.vendor,
				})
				if (!result.ok) {
					console.warn('[admin-wallet] watch-only session init failed:', result.error)
				}
			} catch (err) {
				console.warn('[admin-wallet] failed to fetch account xpub or fingerprint from device:', err)
			}
		}
	}
}
