import { useCallback, useMemo, useState, type ReactNode } from 'react'
import { WalletSessionContext } from '@/contexts/wallet-session-context'
import { pocWalletAdapter } from '@/wallet/default-poc-adapter'
import type { WalletAccountInfo } from '@/wallet/types'

export function WalletSessionProvider({ children }: { children: ReactNode }) {
	const [wallet, setWallet] = useState<WalletAccountInfo | null>(null)
	const setConnectedWallet = useCallback((info: WalletAccountInfo | null) => {
		setWallet(info)
	}, [])
	const clearSession = useCallback(() => {
		setWallet(null)
		void pocWalletAdapter.disconnect()
	}, [])
	const value = useMemo(
		() => ({
			wallet,
			setConnectedWallet,
			clearSession,
			adapter: pocWalletAdapter,
		}),
		[wallet, setConnectedWallet, clearSession],
	)
	return <WalletSessionContext.Provider value={value}>{children}</WalletSessionContext.Provider>
}
