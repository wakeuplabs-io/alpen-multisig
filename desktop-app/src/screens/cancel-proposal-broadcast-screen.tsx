import { Navigate, useNavigate, useParams } from 'react-router-dom'
import { ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { LogOutMutedIcon, ShieldPurpleIcon } from '@/assets/icons'
import { BroadcastDetailsCard } from '@/domain/broadcast-proposal/components/broadcast-details-card'
import { BroadcastPhaseProgress } from '@/domain/broadcast-proposal/components/broadcast-phase-progress'
import { useCancelBroadcast } from '@/domain/cancel-proposal/hooks/use-cancel-broadcast'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { authorityLabelForRole } from '@/lib/authority-label'

export function CancelProposalBroadcastScreen() {
	const navigate = useNavigate()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, selectedRole, sessionTimeLabel, disconnectSession } = useSession()

	const authorityLabel = authorityLabelForRole(selectedRole)

	const { isResolvingCancel, cancelResolveError, phase, bundle, result, proposal, error, prepare, broadcast } =
		useCancelBroadcast(ORCHESTRATOR_BASE_URL, actionId ?? '')

	async function handleBack() {
		await disconnectSession()
	}

	if (wallet === null) return <Navigate to="/" replace />
	if (actionId === undefined) return <Navigate to="/proposals" replace />

	const isLoading = isResolvingCancel || phase === 'idle' || phase === 'preparing'
	const showDetails =
		bundle !== null && (phase === 'confirming' || phase === 'awaiting-device' || phase === 'broadcasting')
	const showProgress = phase === 'broadcasting' || phase === 'done' || phase === 'error'

	return (
		<ScreenShell
			headerContent={
				<>
					<span className="inline-flex items-center gap-1.5 rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-2.5 py-1.25 text-[12px] font-medium text-[#7c6fcd]">
						<ShieldPurpleIcon width={12} height={12} className="block shrink-0" />
						{authorityLabel}
					</span>
					<span className="inline-flex items-center gap-2 rounded-full border border-[#e5e7eb] bg-[#f8f8fb] px-3 py-1.25 text-[12px]">
						<span className="font-mono text-[11px] font-medium text-[#111827]">Session · {sessionTimeLabel}</span>
					</span>
					<button
						type="button"
						className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-[12px] font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleBack()}
					>
						<LogOutMutedIcon width={12} height={12} className="block shrink-0" />
						Disconnect
					</button>
				</>
			}
		>
			<div className="mx-auto w-full max-w-190">
				<button
					type="button"
					className="inline-flex items-center gap-1.5 text-sm text-[#6b7280] transition hover:text-[#111827]"
					onClick={() => navigate(`/proposals/${actionId}/cancel`)}
				>
					← Back to cancel proposal
				</button>

				<h1 className="m-0 mt-3 font-['BIZ_UDPMincho'] text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Broadcast cancel tx
				</h1>
				<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
					Cancel signatures have reached quorum. Review the commit details, then broadcast to remove the queued update.
				</p>

				<div className="mt-6 space-y-4">
					{isLoading && (
						<div className="animate-pulse space-y-3 rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
							<div className="h-7 w-48 rounded-lg bg-[#f3f4f6]" />
							<div className="h-4 w-32 rounded-md bg-[#f3f4f6]" />
							<div className="mt-4 h-1.5 w-full rounded-full bg-[#f3f4f6]" />
							<div className="mt-6 h-10 w-full rounded-xl bg-[#f3f4f6]" />
						</div>
					)}

					{cancelResolveError !== null && (
						<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
							<p className="m-0 text-sm text-[#991b1b]">{cancelResolveError.message}</p>
						</div>
					)}

					{showDetails && (
						<BroadcastDetailsCard
							bundle={bundle}
							proposal={proposal}
							onBroadcast={() => void broadcast()}
							isBroadcasting={phase === 'broadcasting' || phase === 'awaiting-device'}
						/>
					)}

					{showProgress && (
						<BroadcastPhaseProgress
							phase={phase}
							proposalStatus={proposal?.status ?? result?.proposalStatus}
							commitTxid={result?.commitTxid}
							revealTxid={result?.revealTxid}
							error={error}
						/>
					)}

					{phase === 'done' && (
						<div className="rounded-xl border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3">
							<p className="m-0 text-sm font-medium text-[#065f46]">
								Cancel transaction confirmed on-chain. The queued update has been removed.
							</p>
							<button
								type="button"
								className="mt-3 inline-flex items-center rounded-md border border-[#065f46] bg-white px-3 py-1.5 text-xs font-medium text-[#065f46] transition hover:bg-[#f0fdf4]"
								onClick={() => navigate('/proposals')}
							>
								Back to proposals
							</button>
						</div>
					)}

					{phase === 'error' && (
						<button
							type="button"
							onClick={() => void prepare()}
							className="inline-flex items-center rounded-xl border border-[#111827] bg-[#111827] px-4 py-2 text-sm font-medium text-white transition hover:bg-black"
						>
							Retry
						</button>
					)}
				</div>
			</div>
		</ScreenShell>
	)
}
