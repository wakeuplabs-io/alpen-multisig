import { useCallback, useEffect, useMemo, useState } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import { ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
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

export function ProposalsDashboardScreen() {
	const navigate = useNavigate()
	const {
		wallet,
		selectedRole,
		sessionTimeLabel,
		sessionWarning,
		disconnectSession,
		ensureOrchestratorSession,
	} = useSession()
	const [proposals, setProposals] = useState<Proposal[]>([])
	const [isLoading, setIsLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)

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
	const signerLabel = wallet.addressSample
		? `${wallet.addressSample.slice(0, 10)}…${wallet.addressSample.slice(-8)}`
		: 'Unknown'

	return (
		<ScreenShell
			headerContent={
				<>
					<span className="inline-flex items-center gap-1.5 rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-2.5 py-1.25 text-[12px] font-medium text-[#7c6fcd]">
						<ShieldPurpleIcon width={12} height={12} className="block shrink-0" />
						{authorityLabel}
					</span>

					<SessionChip
						timeLabel={sessionTimeLabel}
						signerLabel={signerLabel}
						warning={sessionWarning}
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
			<span
				className="h-3 w-px"
				style={{ background: warning ? '#fde68a' : '#e5e7eb' }}
				aria-hidden="true"
			/>
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
