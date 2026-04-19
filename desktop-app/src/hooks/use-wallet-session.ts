import { useContext } from 'react'
import { WalletSessionContext } from '@/contexts/wallet-session-context'

export function useWalletSession() {
	const ctx = useContext(WalletSessionContext)
	if (ctx === null) {
		throw new Error('useWalletSession must be used within WalletSessionProvider')
	}
	return ctx
}
