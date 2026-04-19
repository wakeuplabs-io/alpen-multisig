import { useCallback, useMemo, useState, type ReactNode } from 'react'
import { WalletSessionContext } from '@/contexts/wallet-session-context'
import type { Authority } from '@/types'
import { pocWalletAdapter } from '@/wallet/default-poc-adapter'
import type { WalletAccountInfo } from '@/wallet/types'

export function WalletSessionProvider({ children }: { children: ReactNode }) {
	const [wallet, setWallet] = useState<WalletAccountInfo | null>(null)
	const [selectedAuthority, setSelectedAuthority] = useState<Authority | null>(null)
	const setConnectedWallet = useCallback((info: WalletAccountInfo | null) => {
		setWallet(info)
		if (info === null) {
			setSelectedAuthority(null)
		}
	}, [])
	const clearSession = useCallback(() => {
		setWallet(null)
		setSelectedAuthority(null)
		void pocWalletAdapter.disconnect()
	}, [])
	const value = useMemo(
		() => ({
			wallet,
			setConnectedWallet,
			selectedAuthority,
			setSelectedAuthority,
			clearSession,
			adapter: pocWalletAdapter,
		}),
		[wallet, setConnectedWallet, selectedAuthority, setSelectedAuthority, clearSession],
	)
	return <WalletSessionContext.Provider value={value}>{children}</WalletSessionContext.Provider>
}
