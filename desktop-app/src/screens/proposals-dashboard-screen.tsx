import { useCallback, useEffect, useMemo, useState } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import {
	authorityFromRole,
	orchestratorAuthComplete,
	orchestratorAuthGetSession,
	orchestratorAuthStart,
	ORCHESTRATOR_BASE_URL,
} from '@/api/orchestrator-auth'
import { listProposals, type Proposal } from '@/api/proposals'
import { AuthRole } from '@/types'
import { ProposalsDashboard } from '@/domain/proposals-dashboard/components/proposals-dashboard'
import { useAuthSession } from '@/hooks/use-auth-session'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { ScreenShell } from '@/screens/screen-shell'

export function ProposalsDashboardScreen() {
	const navigate = useNavigate()
	const { wallet, adapter, clearSession } = useWalletSession()
	const { logout, selectedRole, session } = useAuthSession()
	const [proposals, setProposals] = useState<Proposal[]>([])
	const [isLoading, setIsLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)

	const authorityLabel =
		selectedRole === AuthRole.StrataAdministrator ? 'Strata Administrator' : 'Strata Sequencer Manager'
	const sessionLabel = formatSessionLabel(session?.expiresAtUnixMs ?? null)

	async function handleDisconnect() {
		await logout()
		clearSession()
	}

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

	const loadProposals = useCallback(async () => {
		setIsLoading(true)
		setError(null)
		try {
			await ensureOrchestratorSession()
			const response = await listProposals({ baseUrl: ORCHESTRATOR_BASE_URL })
			if (!response.ok) {
				throw new Error(response.error)
			}
			setProposals(response.data)
		} catch (loadError) {
			setError(String(loadError))
		} finally {
			setIsLoading(false)
		}
	}, [ensureOrchestratorSession])

	useEffect(() => {
		void loadProposals()
	}, [loadProposals, selectedRole])

	const quorumReached = useMemo(() => proposals.filter((proposal) => proposal.status === 'approved'), [proposals])
	const pending = useMemo(() => proposals.filter((proposal) => proposal.status === 'pending'), [proposals])
	const executedOrCanceled = useMemo(
		() => proposals.filter((proposal) => proposal.status === 'enacted' || proposal.status === 'canceled'),
		[proposals],
	)
	const expiredOrSkipped = useMemo(() => proposals.filter((proposal) => proposal.status === 'expired'), [proposals])

	if (wallet === null) {
		return <Navigate to="/" replace />
	}
	const signerLabel = wallet.addressSample ? `${wallet.addressSample.slice(0, 6)}...${wallet.addressSample.slice(-6)}` : 'Unknown'

	return (
		<ScreenShell
			headerContent={
				<>
					<span className="inline-flex items-center rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-[10px] py-1 text-xs font-medium text-[#5b44c9]">
						{authorityLabel}
					</span>
					<span className="inline-flex items-center rounded-full border border-[#e5e7eb] bg-[#f8f8fb] px-3 py-[5px] text-xs text-[#111827]">
						{sessionLabel}
					</span>
					<span className="inline-flex items-center rounded-full border border-[#e5e7eb] bg-[#f8f8fb] px-3 py-[5px] font-mono text-[11px] text-[#6b7280]">
						{signerLabel}
					</span>
					<button
						type="button"
						className="inline-flex items-center rounded-lg border border-[#e5e7eb] bg-white px-[10px] py-[5px] text-xs font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleDisconnect()}
					>
						Disconnect
					</button>
				</>
			}
		>
			<ProposalsDashboard
				authorityLabel={authorityLabel}
				quorumReached={quorumReached}
				pending={pending}
				executedOrCanceled={executedOrCanceled}
				expiredOrSkipped={expiredOrSkipped}
				isLoading={isLoading}
				error={error}
				onRetry={() => void loadProposals()}
				onCreateProposal={() => {
					navigate('/proposals/create')
				}}
			/>
		</ScreenShell>
	)
}

function formatSessionLabel(expiresAtUnixMs: number | null): string {
	if (expiresAtUnixMs === null) {
		return 'Session'
	}

	const remainingMs = Math.max(0, expiresAtUnixMs - Date.now())
	const minutes = Math.floor(remainingMs / 60_000)
	const seconds = Math.floor((remainingMs % 60_000) / 1000)
	return `Session · ${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
}
