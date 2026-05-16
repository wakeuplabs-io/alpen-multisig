import { Navigate, useNavigate, useParams } from 'react-router-dom'
import { ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { LogOutMutedIcon, ShieldPurpleIcon } from '@/assets/icons'
import { BroadcastDetailsCard } from '@/domain/broadcast-proposal/components/broadcast-details-card'
import { BroadcastPhaseProgress } from '@/domain/broadcast-proposal/components/broadcast-phase-progress'
import { useBroadcastProposal } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { AuthRole } from '@/types/auth-role'

export function BroadcastProposalScreen() {
	const navigate = useNavigate()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, selectedRole, sessionTimeLabel, disconnectSession } = useSession()

	const authorityLabel =
		selectedRole === AuthRole.StrataAdministrator ? 'Alpen Administrator' : 'Alpen Sequencer Manager'

	const { phase, bundle, result, error, prepare, broadcast } = useBroadcastProposal(
		ORCHESTRATOR_BASE_URL,
		actionId ?? '',
	)

	async function handleBack() {
		await disconnectSession()
	}

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	if (actionId === undefined) {
		return <Navigate to="/proposals" replace />
	}

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
			<div className="mx-auto w-full max-w-[760px]">
				<button
					type="button"
					className="inline-flex items-center gap-1.5 text-sm text-[#6b7280] transition hover:text-[#111827]"
					onClick={() => navigate('/proposals')}
				>
					← Back
				</button>

				<h1 className="m-0 mt-3 font-['BIZ_UDPMincho'] text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Broadcast proposal
				</h1>
				<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
					Review the commit details, then broadcast to Bitcoin via the commit/reveal flow.
				</p>

				<div className="mt-6 space-y-4">
					{(phase === 'idle' || phase === 'preparing') && (
						<div className="rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
							<p className="m-0 text-sm text-[#6b7280]">Preparing broadcast artifacts from the orchestrator…</p>
							<button
								type="button"
								data-testid="e2e-broadcast-prepare"
								disabled={phase === 'preparing'}
								onClick={() => void prepare()}
								className="mt-4 inline-flex items-center rounded-xl border border-[#111827] bg-[#111827] px-4 py-2 text-sm font-medium text-white transition hover:bg-black disabled:cursor-not-allowed disabled:opacity-60"
							>
								{phase === 'preparing' ? 'Preparing…' : 'Prepare broadcast'}
							</button>
						</div>
					)}

					{bundle !== null && phase === 'confirming' && (
						<BroadcastDetailsCard bundle={bundle} onBroadcast={() => void broadcast()} isBroadcasting={false} />
					)}

					{phase === 'broadcasting' && bundle !== null && (
						<BroadcastDetailsCard bundle={bundle} onBroadcast={() => void broadcast()} isBroadcasting />
					)}

					{(phase === 'broadcasting' || phase === 'done' || phase === 'error') && (
						<BroadcastPhaseProgress
							phase={phase}
							commitTxid={result?.commitTxid}
							revealTxid={result?.revealTxid}
							error={error}
						/>
					)}

					{phase === 'done' && (
						<div
							className="rounded-xl border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3"
							data-testid="e2e-broadcast-done-banner"
						>
							<p className="m-0 text-sm font-medium text-[#065f46]">Proposal enacted onchain.</p>
							<button
								type="button"
								className="mt-3 inline-flex items-center rounded-md border border-[#065f46] bg-white px-3 py-1.5 text-xs font-medium text-[#065f46] transition hover:bg-[#f0fdf4]"
								onClick={() => navigate('/proposals')}
							>
								Back to proposals
							</button>
						</div>
					)}
				</div>
			</div>
		</ScreenShell>
	)
}
