import { useEffect, useState } from 'react'
import { walletSessionInit, walletSessionInitWatchOnly } from '@/api/admin-wallet'
import { initAdminWalletForAdapter } from '@/contexts/session-provider-vendor-branch'
import type { WalletAdapter } from '@/wallet/types'

type UseEnsureAdminWalletSessionReturn = {
	/** True once the session init attempt has resolved (success or failure). */
	sessionReady: boolean
}

/** Re-binds the Tauri Admin Wallet session when the UI wallet is connected but the Rust slot was cleared (e.g. app restart). */
export function useEnsureAdminWalletSession(adapter: WalletAdapter): UseEnsureAdminWalletSessionReturn {
	const [sessionReady, setSessionReady] = useState(false)

	useEffect(() => {
		void initAdminWalletForAdapter(adapter, walletSessionInitWatchOnly, walletSessionInit).then((result) => {
			if (!result.ok) {
				console.warn('[admin-wallet] ensure session on mount failed:', result.error)
			}
			setSessionReady(true)
		})
	}, [adapter])

	return { sessionReady }
}
