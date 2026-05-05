import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import {
	authorityFromRole,
	orchestratorAuthComplete,
	orchestratorAuthGetSession,
	orchestratorAuthLogout,
	orchestratorAuthStart,
	ORCHESTRATOR_BASE_URL,
} from '@/api/orchestrator-auth'
import { SessionContext } from '@/contexts/session-context'
import { useAuthSession } from '@/hooks/use-auth-session'
import { useWalletSession } from '@/hooks/use-wallet-session'

export function SessionProvider({ children }: { children: ReactNode }) {
	const { session, isAuthenticated, isLoading, selectedRole, setSelectedRole, authenticate, logout } = useAuthSession()
	const { wallet, setConnectedWallet, clearSession, adapter } = useWalletSession()

	/** Wall clock for session countdown; updated every second (effects may call Date.now). */
	const [nowMs, setNowMs] = useState(() => Date.now())
	useEffect(() => {
		const id = setInterval(() => setNowMs(Date.now()), 1_000)
		return () => clearInterval(id)
	}, [])
	useEffect(() => {
		setNowMs(Date.now())
	}, [session?.expiresAtUnixMs])

	const remainingMs = Math.max(0, (session?.expiresAtUnixMs ?? 0) - nowMs)
	const min = Math.floor(remainingMs / 60_000)
	const sec = Math.floor((remainingMs % 60_000) / 1_000)
	const sessionTimeLabel = session ? `${String(min).padStart(2, '0')}:${String(sec).padStart(2, '0')}` : '--:--'
	const sessionWarning = session !== null && min < 5

	const ensureOrchestratorSession = useCallback(async () => {
		const currentSession = await orchestratorAuthGetSession()
		if (!currentSession.ok) {
			throw new Error(currentSession.error)
		}
		if (currentSession.data !== null) {
			return
		}

		const challengeResult = await orchestratorAuthStart({
			baseUrl: ORCHESTRATOR_BASE_URL,
			authority: authorityFromRole(selectedRole),
		})
		if (!challengeResult.ok) {
			throw new Error(challengeResult.error)
		}

		const signature = await adapter.signSighash(challengeResult.data.challengeHex)
		const completeResult = await orchestratorAuthComplete({
			baseUrl: ORCHESTRATOR_BASE_URL,
			challengeId: challengeResult.data.challengeId,
			signerPubkey: signature.publicKeyHex,
			signatureHex: signature.signatureHex,
			signatureFormat: signature.signatureFormat,
		})
		if (!completeResult.ok) {
			throw new Error(completeResult.error)
		}
	}, [adapter, selectedRole])

	const connectSession = useCallback(async () => {
		await authenticate((challengeHex: string) => adapter.signSighash(challengeHex))
		const challengeResult = await orchestratorAuthStart({
			baseUrl: ORCHESTRATOR_BASE_URL,
			authority: authorityFromRole(selectedRole),
		})
		if (!challengeResult.ok) {
			throw new Error(challengeResult.error)
		}
		const signature = await adapter.signSighash(challengeResult.data.challengeHex)
		const completeResult = await orchestratorAuthComplete({
			baseUrl: ORCHESTRATOR_BASE_URL,
			challengeId: challengeResult.data.challengeId,
			signerPubkey: signature.publicKeyHex,
			signatureHex: signature.signatureHex,
			signatureFormat: signature.signatureFormat,
		})
		if (!completeResult.ok) {
			throw new Error(completeResult.error)
		}
	}, [adapter, authenticate, selectedRole])

	const disconnectSession = useCallback(async () => {
		void (await orchestratorAuthLogout(ORCHESTRATOR_BASE_URL))
		await logout()
		clearSession()
	}, [clearSession, logout])

	const value = useMemo(
		() => ({
			remainingMs,
			sessionTimeLabel,
			sessionWarning,
			connectSession,
			disconnectSession,
			ensureOrchestratorSession,
			session,
			isAuthenticated,
			isLoading,
			selectedRole,
			setSelectedRole,
			wallet,
			setConnectedWallet,
			adapter,
			clearSession,
		}),
		[
			remainingMs,
			sessionTimeLabel,
			sessionWarning,
			connectSession,
			disconnectSession,
			ensureOrchestratorSession,
			session,
			isAuthenticated,
			isLoading,
			selectedRole,
			setSelectedRole,
			wallet,
			setConnectedWallet,
			adapter,
			clearSession,
		],
	)

	return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>
}
