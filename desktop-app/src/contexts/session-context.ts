import { createContext } from 'react'
import type { AuthRole, AuthSession } from '@/api/authentication'
import type { WalletAccountInfo, WalletAdapter, WalletAdapterOptions, WalletVendor } from '@/wallet/types'

export type SigningStepInfo = {
	challengeMessage: string
	step: number
	totalSteps: number
}

export type SessionContextValue = {
	remainingMs: number
	sessionTimeLabel: string
	sessionWarning: boolean
	connectOrchestratorSession: () => Promise<void>
	connectOnChainSession: () => Promise<void>
	disconnectSession: () => Promise<void>
	ensureOrchestratorSession: () => Promise<void>
	session: AuthSession | null
	isAuthenticated: boolean
	isOrchestratorSessionActive: boolean
	isLoading: boolean
	selectedRole: AuthRole
	setSelectedRole: (role: AuthRole) => void
	wallet: WalletAccountInfo | null
	setConnectedWallet: (info: WalletAccountInfo | null) => void
	adapter: WalletAdapter
	selectAdapter: (vendor: WalletVendor, options?: WalletAdapterOptions) => void
	clearSession: () => void
	signingStep: SigningStepInfo | null
}

export const SessionContext = createContext<SessionContextValue | null>(null)
