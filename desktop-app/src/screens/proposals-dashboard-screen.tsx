import { useCallback, useEffect, useMemo, useState } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import { orchestratorAuthGetSession, ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { listProposals, type Proposal } from '@/api/proposals'
import { LogOutMutedIcon, LogOutRedIcon, ShieldPurpleIcon } from '@/assets/icons'
import { AuthRole } from '@/types'
import { ProposalsDashboard } from '@/domain/proposals-dashboard/components/proposals-dashboard'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'

export function ProposalsDashboardScreen() {
	const navigate = useNavigate()
	const { wallet, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession, ensureOrchestratorSession } =
		useSession()
	const [proposals, setProposals] = useState<Proposal[]>([])
	const [isLoading, setIsLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [signerPubkey, setSignerPubkey] = useState<string | null>(null)

	const panel = useWalletPanelData()

	const authorityLabel =
		selectedRole === AuthRole.StrataAdministrator ? 'Strata Administrator' : 'Strata Sequencer Manager'

	async function handleDisconnect() {
		await disconnectSession()
	}

	const loadProposals = useCallback(async () => {
		setIsLoading(true)
		setError(null)
		try {
			await ensureOrchestratorSession()
			const currentSession = await orchestratorAuthGetSession()
			if (!currentSession.ok) {
				throw new Error(currentSession.error)
			}
			setSignerPubkey(currentSession.data?.signerPubkey ?? null)
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

	const quorumReached = useMemo(
		() => proposals.filter((proposal) => proposal.status === 'approved' || hasReachedQuorum(proposal)),
		[proposals],
	)
	const pending = useMemo(
		() => proposals.filter((proposal) => proposal.status === 'pending' && !hasReachedQuorum(proposal)),
		[proposals],
	)
	const executedOrCanceled = useMemo(
		() => proposals.filter((proposal) => proposal.status === 'enacted' || proposal.status === 'canceled'),
		[proposals],
	)
	const expiredOrSkipped = useMemo(() => proposals.filter((proposal) => proposal.status === 'expired'), [proposals])

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	return (
		<ScreenShell
			headerContent={
				<>
					<span className="inline-flex items-center gap-1.5 rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-2.5 py-1.25 text-[12px] font-medium text-[#7c6fcd]">
						<ShieldPurpleIcon width={12} height={12} className="block shrink-0" />
						{authorityLabel}
					</span>

					<WalletSessionControl
						panel={panel}
						sessionTimeLabel={sessionTimeLabel}
						sessionWarning={sessionWarning}
						addressSample={wallet.addressSample}
					/>

					<button
						type="button"
						className="group/disconnect inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-[12px] font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleDisconnect()}
					>
						<span className="relative inline-flex h-3 w-3 shrink-0">
							<LogOutMutedIcon
								width={12}
								height={12}
								className="absolute left-0 top-0 transition-opacity group-hover/disconnect:opacity-0"
							/>
							<LogOutRedIcon
								width={12}
								height={12}
								className="absolute left-0 top-0 opacity-0 transition-opacity group-hover/disconnect:opacity-100"
							/>
						</span>
						Disconnect
					</button>
				</>
			}
		>
			<ProposalsDashboard
				authorityLabel={authorityLabel}
				signerPubkey={signerPubkey}
				quorumReached={quorumReached}
				pending={pending}
				executedOrCanceled={executedOrCanceled}
				expiredOrSkipped={expiredOrSkipped}
				isLoading={isLoading}
				error={error}
				onRetry={() => void loadProposals()}
				onRefresh={() => void loadProposals()}
				onCreateProposal={() => {
					navigate('/proposals/create')
				}}
				onViewProposal={(actionId) => {
					const p = proposals.find((pr) => pr.actionId === actionId)
					if (p?.kind === 'cancel' && p.targetActionId !== null) {
						navigate(`/proposals/${p.targetActionId}/cancel`, { state: { signerPubkey } })
					} else {
						navigate(`/proposals/${actionId}`, { state: { signerPubkey } })
					}
				}}
				onSignProposal={(actionId) => {
					navigate(`/proposals/${actionId}/sign`)
				}}
				onBroadcastProposal={(actionId) => {
					navigate(`/proposals/${actionId}/broadcast`)
				}}
				onCancelProposal={(actionId) => {
					navigate(`/proposals/${actionId}/cancel`, { state: { signerPubkey } })
				}}
			/>
		</ScreenShell>
	)
}

function hasReachedQuorum(proposal: Proposal): boolean {
	return proposal.status === 'pending' && proposal.signatures.length >= proposal.requiredSignatures
}
