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
	console.log('[admin-wallet] initAdminWalletForAdapter called, vendor:', adapter.vendor)

	if (adapter.vendor === 'mnemonic') {
		const mnemonicAdapter = adapter as unknown as MnemonicAdapter
		const result = await walletSessionInit({ mnemonic: mnemonicAdapter.getMnemonic() })
		console.log('[admin-wallet] mnemonic session init result:', result)
		if (!result.ok) {
			console.warn('[admin-wallet] session init failed:', result.error)
		}
	} else if (adapter.vendor === 'trezor' || adapter.vendor === 'ledger') {
		console.log('[admin-wallet] HW path: has getAccountXpub:', !!adapter.getAccountXpub, 'has getMasterFingerprint:', !!adapter.getMasterFingerprint)
		if (adapter.getAccountXpub && adapter.getMasterFingerprint) {
			try {
				console.log('[admin-wallet] fetching xpub and fingerprint...')
				const [xpub, masterFingerprint] = await Promise.all([adapter.getAccountXpub(), adapter.getMasterFingerprint()])
				console.log('[admin-wallet] xpub:', xpub, 'fingerprint:', masterFingerprint)
				const result = await walletSessionInitWatchOnly({
					xpub,
					masterFingerprint,
					deviceType: adapter.vendor,
				})
				console.log('[admin-wallet] watch-only session init result:', result)
				if (!result.ok) {
					console.warn('[admin-wallet] watch-only session init failed:', result.error)
				}
			} catch (err) {
				console.error('[admin-wallet] failed to fetch account xpub or fingerprint from device:', err)
			}
		} else {
			console.warn('[admin-wallet] HW adapter missing getAccountXpub or getMasterFingerprint')
		}
	}
}
