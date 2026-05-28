import { useCallback, useEffect, useMemo, useState } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import { orchestratorAuthGetSession, ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { listProposals, type Proposal } from '@/api/proposals'
import {
	ClockSessionDefaultIcon,
	ClockSessionWarningIcon,
	LogOutMutedIcon,
	LogOutRedIcon,
	ShieldPurpleIcon,
	UsbSessionDefaultIcon,
	UsbSessionWarningIcon,
} from '@/assets/icons'
import { AuthRole } from '@/types'
import { ProposalsDashboard } from '@/domain/proposals-dashboard/components/proposals-dashboard'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { useWalletPanelState } from '@/domain/admin-wallet/hooks/use-wallet-panel-state'
import { useAdminWalletBalance } from '@/domain/admin-wallet/hooks/use-admin-wallet-balance'
import { useAdminWalletAddresses } from '@/domain/admin-wallet/hooks/use-admin-wallet-addresses'
import { useAdminWalletSync } from '@/domain/admin-wallet/hooks/use-admin-wallet-sync'
import { useAddressesWithBalance } from '@/domain/admin-wallet/hooks/use-addresses-with-balance'
import { WalletPanel } from '@/domain/admin-wallet/components/wallet-panel'
import { WalletPanelHeader } from '@/domain/admin-wallet/components/wallet-panel-header'
import { WalletPanelTrigger } from '@/domain/admin-wallet/components/wallet-panel-trigger'
import { WalletPanelContent } from '@/domain/admin-wallet/components/wallet-panel-content'

export function ProposalsDashboardScreen() {
	const navigate = useNavigate()
	const { wallet, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession, ensureOrchestratorSession } =
		useSession()
	const [proposals, setProposals] = useState<Proposal[]>([])
	const [isLoading, setIsLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [signerPubkey, setSignerPubkey] = useState<string | null>(null)

	const { isOpen, expandedSection, open, close, setExpandedSection } = useWalletPanelState()
	const balanceHook = useAdminWalletBalance()
	const addressesHook = useAdminWalletAddresses('External', 0, 20)
	const syncHook = useAdminWalletSync()
	const addressesWithBalanceHook = useAddressesWithBalance()

	const walletDisabledError =
		balanceHook.error?.type === 'Disabled' || balanceHook.error?.type === 'RegtestGuardViolation'
			? balanceHook.error
			: addressesHook.error?.type === 'Disabled' || addressesHook.error?.type === 'RegtestGuardViolation'
				? addressesHook.error
				: null

	const receiveAddress = addressesHook.data?.find((a) => !a.isUsed)?.address ?? null

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
	const signerLabel = wallet.addressSample
		? `${wallet.addressSample.slice(0, 10)}…${wallet.addressSample.slice(-8)}`
		: 'Unknown'

	return (
		<ScreenShell
			headerContent={
				<>
					<WalletPanelTrigger isOpen={isOpen} onToggle={() => (isOpen ? close() : open())} />

					<span className="inline-flex items-center gap-1.5 rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-2.5 py-1.25 text-[12px] font-medium text-[#7c6fcd]">
						<ShieldPurpleIcon width={12} height={12} className="block shrink-0" />
						{authorityLabel}
					</span>

					<SessionChip timeLabel={sessionTimeLabel} signerLabel={signerLabel} warning={sessionWarning} />

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

			<WalletPanel isOpen={isOpen} onClose={close} panelId="wallet-slide-dialog">
				<WalletPanelHeader onClose={close} />
				<WalletPanelContent
					disabledError={walletDisabledError}
					balanceSats={balanceHook.data?.confirmedSats ?? 0}
					isBalanceLoading={balanceHook.isLoading}
					receiveAddress={receiveAddress}
					isAddressesLoading={addressesHook.isLoading}
					addressRows={addressesWithBalanceHook.data}
					addressRowsLoading={addressesWithBalanceHook.isLoading}
					addressRowsError={addressesWithBalanceHook.error}
					expandedSection={expandedSection}
					onToggleAddresses={() => setExpandedSection(expandedSection === 'addresses' ? null : 'addresses')}
					syncStatus={syncHook.syncStatus}
					isSyncRefreshing={syncHook.isLoading}
					syncError={syncHook.error}
					onRefreshSync={() => void syncHook.triggerSync()}
				/>
			</WalletPanel>
		</ScreenShell>
	)
}

function hasReachedQuorum(proposal: Proposal): boolean {
	return proposal.status === 'pending' && proposal.signatures.length >= proposal.requiredSignatures
}

function SessionChip({
	timeLabel,
	signerLabel,
	warning,
}: {
	timeLabel: string
	signerLabel: string
	warning: boolean
}) {
	return (
		<span
			className="inline-flex items-center gap-2 rounded-full border px-3 py-1.25 text-[12px] whitespace-nowrap flex-none transition"
			style={
				warning
					? {
							background: '#fffbeb',
							borderColor: '#fde68a',
							color: '#d97706',
						}
					: {
							background: '#f8f8fb',
							borderColor: '#e5e7eb',
							color: '#111827',
						}
			}
		>
			{warning ? (
				<ClockSessionWarningIcon width={12} height={12} className="block shrink-0" />
			) : (
				<ClockSessionDefaultIcon width={12} height={12} className="block shrink-0" />
			)}
			<span className="font-mono text-[11px] font-medium">Session · {timeLabel}</span>
			<span className="h-3 w-px" style={{ background: warning ? '#fde68a' : '#e5e7eb' }} aria-hidden="true" />
			{warning ? (
				<UsbSessionWarningIcon width={12} height={12} className="block shrink-0" />
			) : (
				<UsbSessionDefaultIcon width={12} height={12} className="block shrink-0" />
			)}
			<span className="font-mono text-[11px]" style={{ color: warning ? '#d97706' : '#6b7280' }}>
				{signerLabel}
			</span>
		</span>
	)
}
