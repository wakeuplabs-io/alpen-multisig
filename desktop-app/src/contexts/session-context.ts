import { createContext } from 'react'
import type { AuthRole, AuthSession } from '@/api/authentication'
import type { WalletAccountInfo, WalletAdapter, WalletAdapterOptions, WalletVendor } from '@/wallet/types'

export type SessionContextValue = {
	remainingMs: number
	sessionTimeLabel: string
	sessionWarning: boolean
	connectSession: () => Promise<void>
	disconnectSession: () => Promise<void>
	ensureOrchestratorSession: () => Promise<void>
	session: AuthSession | null
	isAuthenticated: boolean
	isLoading: boolean
	selectedRole: AuthRole
	setSelectedRole: (role: AuthRole) => void
	wallet: WalletAccountInfo | null
	setConnectedWallet: (info: WalletAccountInfo | null) => void
	adapter: WalletAdapter
	selectAdapter: (vendor: WalletVendor, options?: WalletAdapterOptions) => void
	clearSession: () => void
}

export const SessionContext = createContext<SessionContextValue | null>(null)
