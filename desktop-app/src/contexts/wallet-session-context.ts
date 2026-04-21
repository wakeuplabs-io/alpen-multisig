import { createContext } from 'react'
import type { WalletAccountInfo, WalletAdapter } from '@/wallet/types'

export type WalletSessionValue = {
	wallet: WalletAccountInfo | null
	setConnectedWallet: (info: WalletAccountInfo | null) => void
	clearSession: () => void
	adapter: WalletAdapter
}

export const WalletSessionContext = createContext<WalletSessionValue | null>(null)
