import { useCallback, useMemo, useState, type ReactNode } from 'react'
import { WalletSessionContext } from '@/contexts/wallet-session-context'
import { createWalletAdapter } from '@/wallet/create-poc-wallet-adapter'
import { pocWalletAdapter } from '@/wallet/default-poc-adapter'
import type { WalletAccountInfo, WalletAdapter, WalletAdapterOptions, WalletVendor } from '@/wallet/types'

export function WalletSessionProvider({ children }: { children: ReactNode }) {
	const [wallet, setWallet] = useState<WalletAccountInfo | null>(null)
	const [adapter, setAdapter] = useState<WalletAdapter>(pocWalletAdapter)

	const setConnectedWallet = useCallback((info: WalletAccountInfo | null) => {
		setWallet(info)
	}, [])

	const selectAdapter = useCallback(
		(vendor: WalletVendor, options: WalletAdapterOptions = {}) => {
			void adapter.disconnect()
			setWallet(null)
			setAdapter(createWalletAdapter(vendor, options))
		},
		[adapter],
	)

	const clearSession = useCallback(() => {
		setWallet(null)
		void adapter.disconnect()
	}, [adapter])

	const value = useMemo(
		() => ({
			wallet,
			setConnectedWallet,
			clearSession,
			adapter,
			selectAdapter,
		}),
		[wallet, setConnectedWallet, clearSession, adapter, selectAdapter],
	)
	return <WalletSessionContext.Provider value={value}>{children}</WalletSessionContext.Provider>
}
