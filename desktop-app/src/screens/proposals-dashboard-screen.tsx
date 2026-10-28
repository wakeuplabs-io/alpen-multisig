import { useCallback, useEffect, useMemo, useState } from 'react'
import { DisconnectButton } from '@/components/disconnect-button'
import { Navigate, useNavigate } from 'react-router-dom'
import { orchestratorAuthGetSession, getOrchestratorBaseUrl } from '@/api/orchestrator-auth'
import { listProposals, type Proposal } from '@/api/proposals'
import { ShieldAccentIcon } from '@/assets/icons'
import { SafeHarborNote } from '@/components/safe-harbor-note'
import { ProposalsDashboard } from '@/domain/proposals-dashboard/components/proposals-dashboard'
import { useActivationRefresh } from '@/hooks/use-activation-refresh'
import { useBlockHeight } from '@/hooks/use-block-height'
import { useSafeHarborActivated } from '@/hooks/use-safe-harbor-status'
import { useSession } from '@/hooks/use-session'
import { authorityLabelForRole } from '@/lib/authority-label'
import { createProposalReadGate } from '@/lib/proposal-read-gate'
import { isActivationReached } from '@/lib/proposal-status'
import { COUNCIL_DASHBOARD_SAFE_HARBOR_NOTE } from '@/lib/defcon-copy'
import { AuthRole } from '@/types/auth-role'
import { ScreenShell } from '@/screens/screen-shell'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'
import { useSessionAdminId } from '@/domain/admin-wallet/hooks/use-session-admin-id'

export function ProposalsDashboardScreen() {
	const navigate = useNavigate()
	const { wallet, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession, ensureOrchestratorSession } =
		useSession()
	const [proposals, setProposals] = useState<Proposal[]>([])
	const [isLoading, setIsLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [signerPubkey, setSignerPubkey] = useState<string | null>(null)

	const panel = useWalletPanelData()
	const adminId = useSessionAdminId(wallet)
	const currentBlockHeight = useBlockHeight()

	const authorityLabel = authorityLabelForRole(selectedRole)

	// Read by both authorities that hold a lever answering this bridge-wide state, and by no
	// other session. The council's Defcon levers set the flag; the administrator's safe harbor
	// rotation is decided by it, since a rotation submitted after activation is accepted on chain
	// and applied nowhere. Gating it on the council alone judged every rotation against a false flag.
	const isCouncil = selectedRole === AuthRole.StrataSecurityCouncil
	const isStrataAdmin = selectedRole === AuthRole.StrataAdministrator
	const safeHarborActivated = useSafeHarborActivated(isCouncil || isStrataAdmin)

	async function handleDisconnect() {
		await disconnectSession()
	}

	// The visible load and the silent refresh both write `proposals`: a slower, older response must
	// not land over a newer one.
	const [readGate] = useState(createProposalReadGate)

	const loadProposals = useCallback(async () => {
		const ticket = readGate.beginLoad()
		setIsLoading(true)
		setError(null)
		try {
			await ensureOrchestratorSession()
			const currentSession = await orchestratorAuthGetSession()
			if (!currentSession.ok) {
				throw new Error(currentSession.error)
			}
			const response = await listProposals({ baseUrl: getOrchestratorBaseUrl() })
			if (!response.ok) {
				throw new Error(response.error)
			}
			if (readGate.isLatest(ticket)) {
				setSignerPubkey(currentSession.data?.signerPubkey ?? null)
				setProposals(response.data)
			}
		} catch (loadError) {
			if (readGate.isLatest(ticket)) setError(String(loadError))
		} finally {
			readGate.settle(ticket)
			if (readGate.isLatest(ticket)) setIsLoading(false)
		}
	}, [readGate, ensureOrchestratorSession])

	useEffect(() => {
		void loadProposals()
	}, [loadProposals, selectedRole])

	// The tip is polled and the list is not: once a proposal's activation block is reached, read the
	// list again until the backend reports it enacted (#602). Silent — the dashboard swaps the list
	// for a spinner while `isLoading`, and a failed read keeps what it shows.
	const refreshProposals = useCallback(() => {
		const ticket = readGate.beginRefresh()
		if (ticket === null) return
		void listProposals({ baseUrl: getOrchestratorBaseUrl() }).then((response) => {
			if (readGate.isLatest(ticket) && response.ok) setProposals(response.data)
		})
	}, [readGate])
	const hasActivationReached = useMemo(
		() => proposals.some((proposal) => isActivationReached(proposal, currentBlockHeight)),
		[proposals, currentBlockHeight],
	)
	useActivationRefresh(hasActivationReached, refreshProposals)

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
	// Superseded sits with expired: both ran out of a window rather than failing at anything.
	const expiredOrSkipped = useMemo(
		() => proposals.filter((proposal) => proposal.status === 'expired' || proposal.status === 'superseded'),
		[proposals],
	)

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	return (
		<ScreenShell
			authorityBadge={
				<span className="inline-flex items-center gap-1.5 rounded-md border border-accent-border bg-bg-surface px-2.5 py-1.25 text-label font-medium text-accent-hover">
					<ShieldAccentIcon width={12} height={12} className="block shrink-0" />
					{authorityLabel}
				</span>
			}
			headerContent={
				<>
					<WalletSessionControl
						panel={panel}
						sessionTimeLabel={sessionTimeLabel}
						sessionWarning={sessionWarning}
						adminId={adminId}
					/>

					<DisconnectButton onClick={() => void handleDisconnect()} />
				</>
			}
		>
			<ProposalsDashboard
				authorityLabel={authorityLabel}
				currentBlockHeight={currentBlockHeight}
				safeHarborActivated={safeHarborActivated}
				notice={
					isCouncil && safeHarborActivated ? (
						<SafeHarborNote>{COUNCIL_DASHBOARD_SAFE_HARBOR_NOTE}</SafeHarborNote>
					) : null
				}
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
