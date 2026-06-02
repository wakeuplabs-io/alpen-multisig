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
 * Returns `ok: false` when session binding fails — callers must not proceed to broadcast.
 */
export async function initAdminWalletForAdapter(
	adapter: WalletAdapter,
	walletSessionInitWatchOnly: (input: {
		xpub: string
		masterFingerprint?: number
		deviceType?: string
	}) => Promise<ApiResult<null>>,
	walletSessionInit: (input: { mnemonic: string }) => Promise<ApiResult<null>>,
): Promise<ApiResult<null>> {
	console.log('[admin-wallet] initAdminWalletForAdapter called, vendor:', adapter.vendor)

	if (adapter.vendor === 'mock') {
		return { ok: true, data: null }
	}

	if (adapter.vendor === 'mnemonic') {
		const mnemonicAdapter = adapter as unknown as MnemonicAdapter
		const result = await walletSessionInit({ mnemonic: mnemonicAdapter.getMnemonic() })
		console.log('[admin-wallet] mnemonic session init result:', result)
		if (!result.ok) {
			console.warn('[admin-wallet] session init failed:', result.error)
		}
		return result
	}

	if (adapter.vendor === 'trezor' || adapter.vendor === 'ledger') {
		console.log(
			'[admin-wallet] HW path: has getAccountXpub:',
			!!adapter.getAccountXpub,
			'has getMasterFingerprint:',
			!!adapter.getMasterFingerprint,
		)
		if (adapter.getAccountXpub && adapter.getMasterFingerprint) {
			try {
				console.log(
					'[admin-wallet] fetching xpub and fingerprint (sequential — Ledger/Speculos is not concurrent-safe)...',
				)
				const xpub = await adapter.getAccountXpub()
				const masterFingerprint = await adapter.getMasterFingerprint()
				console.log('[admin-wallet] xpub:', xpub, 'fingerprint:', masterFingerprint)
				const result = await walletSessionInitWatchOnly(
					masterFingerprint > 0 ? { xpub, masterFingerprint, deviceType: adapter.vendor } : { xpub },
				)
				console.log('[admin-wallet] watch-only session init result:', result)
				if (!result.ok) {
					console.warn('[admin-wallet] watch-only session init failed:', result.error)
				}
				return result
			} catch (err) {
				const message = err instanceof Error ? err.message : String(err)
				console.error('[admin-wallet] failed to fetch account xpub or fingerprint from device:', err)
				return { ok: false, error: message }
			}
		}
		return {
			ok: false,
			error: 'Hardware wallet adapter is missing account xpub or fingerprint helpers',
		}
	}

	return { ok: true, data: null }
}
