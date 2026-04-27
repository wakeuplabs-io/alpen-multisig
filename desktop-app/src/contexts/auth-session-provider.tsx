import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import {
	authComplete,
	authGetSession,
	authLogout,
	authStartChallenge,
	type AuthRole,
	type AuthSession,
} from '@/api/authentication'
import { AuthSessionContext } from '@/contexts/auth-session-context'
import type { SignSighashResult } from '@/wallet/types'

export function AuthSessionProvider({ children }: { children: ReactNode }) {
	const [session, setSession] = useState<AuthSession | null>(null)
	const [isLoading, setIsLoading] = useState(true)
	const [selectedRole, setSelectedRole] = useState<AuthRole>('strata_administrator')

	const refreshSession = useCallback(async () => {
		const result = await authGetSession()
		if (!result.ok) {
			setSession(null)
			return
		}
		setSession(result.data.authenticated ? result.data.session : null)
	}, [])

	useEffect(() => {
		void (async () => {
			setIsLoading(true)
			await refreshSession()
			setIsLoading(false)
		})()
	}, [refreshSession])

	const authenticate = useCallback(
		async (signChallenge: (challengeHex: string) => Promise<SignSighashResult>) => {
			const challengeResult = await authStartChallenge({ role: selectedRole })
			if (!challengeResult.ok) {
				throw new Error(challengeResult.error)
			}
			const signature = await signChallenge(challengeResult.data.challengeHex)
			const completeResult = await authComplete({
				challengeId: challengeResult.data.challengeId,
				signerPubkeyHex: signature.publicKeyHex,
				signatureHex: signature.signatureHex,
				signatureFormat: signature.signatureFormat,
			})
			if (!completeResult.ok) {
				throw new Error(completeResult.error)
			}
			setSession(completeResult.data)
		},
		[selectedRole],
	)

	const logout = useCallback(async () => {
		await authLogout()
		setSession(null)
	}, [])

	const value = useMemo(
		() => ({
			session,
			isAuthenticated: session !== null,
			isLoading,
			selectedRole,
			setSelectedRole,
			refreshSession,
			authenticate,
			logout,
		}),
		[session, isLoading, selectedRole, refreshSession, authenticate, logout],
	)

	return <AuthSessionContext.Provider value={value}>{children}</AuthSessionContext.Provider>
}
