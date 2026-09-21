import type { HwAddressEntry, WalletAccountInfo } from '@/wallet/types'

export type HwWalletPhase = 'connect' | 'selected'
export type ConnectViewState = 'idle' | 'loading' | 'success'

export type HwWalletConnectState = {
	phase: HwWalletPhase
	loading: boolean
	account: WalletAccountInfo | null
	selectedEntry: HwAddressEntry | null
	connectViewState: ConnectViewState
	/** The device is showing a pairing code; the connect screen asks for it. */
	pairingCodeRequested: boolean
	isVerifyingAddress: boolean
	verifyMessage: string | null
	error: string | null
}
