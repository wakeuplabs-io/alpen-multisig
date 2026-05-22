import type { HwAddressEntry, WalletAccountInfo } from '@/wallet/types'

export type HwWalletPhase = 'connect' | 'picking' | 'selected'
export type ConnectViewState = 'idle' | 'loading' | 'success'

export type HwWalletConnectState = {
	phase: HwWalletPhase
	loading: boolean
	account: WalletAccountInfo | null
	addresses: HwAddressEntry[]
	selectedIndex: number | null
	selectedEntry: HwAddressEntry | null
	connectViewState: ConnectViewState
	isVerifyingAddress: boolean
	verifyMessage: string | null
	error: string | null
}
