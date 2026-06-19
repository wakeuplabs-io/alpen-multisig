import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import {
	authorityFromRole,
	orchestratorAuthComplete,
	orchestratorAuthGetSession,
	orchestratorAuthLogout,
	orchestratorAuthStart,
	getOrchestratorBaseUrl,
} from '@/api/orchestrator-auth'
import { walletSessionInit, walletSessionInitWatchOnly } from '@/api/admin-wallet'
import { SessionContext, type SigningStepInfo } from '@/contexts/session-context'
import { initAdminWalletForAdapter } from '@/contexts/session-provider-vendor-branch'
import { useAuthSession } from '@/hooks/use-auth-session'
import { useWalletSession } from '@/hooks/use-wallet-session'

export function SessionProvider({ children }: { children: ReactNode }) {
	const { session, isAuthenticated, isLoading, selectedRole, setSelectedRole, authenticate, logout } = useAuthSession()
	const { wallet, setConnectedWallet, clearSession, adapter, selectAdapter } = useWalletSession()

	const [signingStep, setSigningStep] = useState<SigningStepInfo | null>(null)
	const [isOrchestratorSessionActive, setIsOrchestratorSessionActive] = useState(false)
	const [orchestratorExpiresAtMs, setOrchestratorExpiresAtMs] = useState<number | null>(null)

	/** Wall clock for session countdown; updated every second. */
	const [nowMs, setNowMs] = useState(() => Date.now())
	useEffect(() => {
		const id = setInterval(() => setNowMs(Date.now()), 1_000)
		return () => clearInterval(id)
	}, [])
	useEffect(() => {
		setNowMs(Date.now())
	}, [session?.expiresAtUnixMs, orchestratorExpiresAtMs])

	const activeExpiresAtMs = orchestratorExpiresAtMs ?? session?.expiresAtUnixMs ?? null
	const remainingMs = Math.max(0, (activeExpiresAtMs ?? 0) - nowMs)
	const min = Math.floor(remainingMs / 60_000)
	const sec = Math.floor((remainingMs % 60_000) / 1_000)
	const sessionTimeLabel = activeExpiresAtMs
		? `${String(min).padStart(2, '0')}:${String(sec).padStart(2, '0')}`
		: '--:--'
	const sessionWarning = activeExpiresAtMs !== null && min < 5

	const ensureOrchestratorSession = useCallback(async () => {
		const expectedAuthority = authorityFromRole(selectedRole)
		const currentSession = await orchestratorAuthGetSession()
		if (!currentSession.ok) {
			throw new Error(currentSession.error)
		}
		if (currentSession.data !== null) {
			if (currentSession.data.authority === expectedAuthority) {
				return
			}
			await orchestratorAuthLogout(getOrchestratorBaseUrl())
		}

		const challengeResult = await orchestratorAuthStart({
			baseUrl: getOrchestratorBaseUrl(),
			authority: authorityFromRole(selectedRole),
		})
		if (!challengeResult.ok) {
			throw new Error(challengeResult.error)
		}

		const signature = await adapter.signSighash(challengeResult.data.challengeMessage)
		const completeResult = await orchestratorAuthComplete({
			baseUrl: getOrchestratorBaseUrl(),
			challengeId: challengeResult.data.challengeId,
			signerPubkey: signature.publicKeyHex,
			signatureHex: signature.signatureHex,
			signatureFormat: signature.signatureFormat,
		})
		if (!completeResult.ok) {
			throw new Error(completeResult.error)
		}
	}, [adapter, selectedRole])

	/** Normal flow: admin wallet init + orchestrator auth (1 hardware-wallet signature). */
	const connectOrchestratorSession = useCallback(async () => {
		try {
			const adminWalletInit = await initAdminWalletForAdapter(adapter, walletSessionInitWatchOnly, walletSessionInit)
			if (!adminWalletInit.ok) {
				throw new Error(
					adminWalletInit.error ?? 'Admin Wallet session could not be started — reconnect your wallet and try again',
				)
			}
			const challengeResult = await orchestratorAuthStart({
				baseUrl: getOrchestratorBaseUrl(),
				authority: authorityFromRole(selectedRole),
			})
			if (!challengeResult.ok) {
				throw new Error(challengeResult.error)
			}
			setSigningStep({
				challengeMessage: challengeResult.data.challengeMessage,
				step: 1,
				totalSteps: 1,
			})
			const signature = await adapter.signSighash(challengeResult.data.challengeMessage)
			const completeResult = await orchestratorAuthComplete({
				baseUrl: getOrchestratorBaseUrl(),
				challengeId: challengeResult.data.challengeId,
				signerPubkey: signature.publicKeyHex,
				signatureHex: signature.signatureHex,
				signatureFormat: signature.signatureFormat,
			})
			if (!completeResult.ok) {
				throw new Error(completeResult.error)
			}
			setIsOrchestratorSessionActive(true)
			setOrchestratorExpiresAtMs(completeResult.data.expiresAtUnixMs)
		} finally {
			setSigningStep(null)
		}
	}, [adapter, selectedRole])

	/** Manual/offline flow: on-chain membership auth + admin wallet init (1 hardware-wallet signature). */
	const connectOnChainSession = useCallback(async () => {
		try {
			await authenticate((challengeMessage: string) => {
				setSigningStep({ challengeMessage, step: 1, totalSteps: 1 })
				return adapter.signSighash(challengeMessage)
			})
			const adminWalletInit = await initAdminWalletForAdapter(adapter, walletSessionInitWatchOnly, walletSessionInit)
			if (!adminWalletInit.ok) {
				throw new Error(
					adminWalletInit.error ?? 'Admin Wallet session could not be started — reconnect your wallet and try again',
				)
			}
		} finally {
			setSigningStep(null)
		}
	}, [adapter, authenticate])

	const disconnectSession = useCallback(async () => {
		setIsOrchestratorSessionActive(false)
		setOrchestratorExpiresAtMs(null)
		void (await orchestratorAuthLogout(getOrchestratorBaseUrl()))
		await logout()
		clearSession()
	}, [clearSession, logout])

	const value = useMemo(
		() => ({
			remainingMs,
			sessionTimeLabel,
			sessionWarning,
			connectOrchestratorSession,
			connectOnChainSession,
			disconnectSession,
			ensureOrchestratorSession,
			session,
			isAuthenticated,
			isOrchestratorSessionActive,
			isLoading,
			selectedRole,
			setSelectedRole,
			wallet,
			setConnectedWallet,
			adapter,
			selectAdapter,
			clearSession,
			signingStep,
		}),
		[
			remainingMs,
			sessionTimeLabel,
			sessionWarning,
			connectOrchestratorSession,
			connectOnChainSession,
			disconnectSession,
			ensureOrchestratorSession,
			session,
			isAuthenticated,
			isOrchestratorSessionActive,
			isLoading,
			selectedRole,
			setSelectedRole,
			wallet,
			setConnectedWallet,
			adapter,
			selectAdapter,
			clearSession,
			signingStep,
		],
	)

	return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>
}
