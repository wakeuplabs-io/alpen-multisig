import { createContext } from 'react'
import type { WalletAccountInfo, WalletAdapter, WalletAdapterOptions, WalletVendor } from '@/wallet/types'

export type WalletSessionValue = {
	wallet: WalletAccountInfo | null
	setConnectedWallet: (info: WalletAccountInfo | null) => void
	clearSession: () => void
	adapter: WalletAdapter
	selectAdapter: (vendor: WalletVendor, options?: WalletAdapterOptions) => void
}

export const WalletSessionContext = createContext<WalletSessionValue | null>(null)
