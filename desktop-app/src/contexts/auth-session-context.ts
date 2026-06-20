import { createContext } from 'react'
import type { AuthRole, AuthSession } from '@/api/authentication'
import type { SignSighashResult } from '@/wallet/types'

export type AuthSessionValue = {
	session: AuthSession | null
	isAuthenticated: boolean
	isLoading: boolean
	selectedRole: AuthRole
	setSelectedRole: (role: AuthRole) => void
	refreshSession: () => Promise<void>
	authenticate: (signChallenge: (challengeMessage: string) => Promise<SignSighashResult>) => Promise<void>
	logout: () => Promise<void>
}

export const AuthSessionContext = createContext<AuthSessionValue | null>(null)
