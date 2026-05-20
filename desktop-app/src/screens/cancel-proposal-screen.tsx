import { Navigate, useLocation, useNavigate, useParams } from 'react-router-dom'
import { ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { LogOutMutedIcon, ShieldPurpleIcon } from '@/assets/icons'
import { ActivationCountdown } from '@/domain/cancel-proposal/components/activation-countdown'
import { CancelDetailsCard } from '@/domain/cancel-proposal/components/cancel-details-card'
import { useProposalDetail } from '@/domain/proposal-detail/hooks/use-proposal-detail'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { authorityLabelForRole } from '@/lib/authority-label'

const CANCELABLE_AUTHORITIES = ['alpen_admin', 'strata_admin']

type LocationState = { signerPubkey?: string | null }

export function CancelProposalScreen() {
	const navigate = useNavigate()
	const location = useLocation()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, selectedRole, sessionTimeLabel, disconnectSession } = useSession()
	const signerPubkey: string | null = (location.state as LocationState)?.signerPubkey ?? null
	const authorityLabel = authorityLabelForRole(selectedRole)

	const { proposal, isLoading, error, reload } = useProposalDetail(ORCHESTRATOR_BASE_URL, actionId ?? '')

	async function handleBack() {
		await disconnectSession()
	}

	if (wallet === null) return <Navigate to="/" replace />
	if (actionId === undefined) return <Navigate to="/proposals" replace />

	// Guard: redirect if target is not cancellable
	if (proposal !== null && proposal.status !== 'approved') {
		return <Navigate to={`/proposals/${actionId}`} replace />
	}
	if (proposal !== null && !CANCELABLE_AUTHORITIES.includes(proposal.authority)) {
		return <Navigate to={`/proposals/${actionId}`} replace />
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
			<div className="mx-auto w-full max-w-190">
				<button
					type="button"
					className="inline-flex items-center gap-1.5 text-sm text-[#6b7280] transition hover:text-[#111827]"
					onClick={() => navigate(`/proposals/${actionId}`)}
				>
					← Back to proposal
				</button>

				<h1 className="m-0 mt-3 font-['BIZ_UDPMincho'] text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Cancel proposal
				</h1>
				<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
					Collect cancel signatures and broadcast to remove the queued update before it activates.
				</p>

				<div className="mt-6 space-y-4">
					{isLoading && (
						<div className="animate-pulse space-y-3 rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
							<div className="h-7 w-48 rounded-lg bg-[#f3f4f6]" />
							<div className="h-4 w-64 rounded-md bg-[#f3f4f6]" />
						</div>
					)}

					{error && (
						<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
							<p className="m-0 text-sm text-[#991b1b]">{error}</p>
							<button
								type="button"
								className="mt-2 rounded-md border border-[#991b1b] bg-white px-3 py-1 text-xs font-medium text-[#991b1b] transition hover:bg-[#fef2f2]"
								onClick={reload}
							>
								Retry
							</button>
						</div>
					)}

					{proposal !== null && (
						<>
							{/* Target already enacted — cancellation no longer possible */}
							{proposal.status === 'enacted' && (
								<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
									<p className="m-0 text-sm font-medium text-[#991b1b]">
										This proposal has already been enacted. Cancellation is no longer possible.
									</p>
								</div>
							)}

							{/* Activation countdown */}
							{proposal.activationHeight !== null && proposal.status === 'approved' && (
								<div className="rounded-xl border border-[#fde68a] bg-[#fffbeb] px-4 py-3">
									<ActivationCountdown activationHeight={proposal.activationHeight} />
								</div>
							)}

							{/* Cancel details or prompt to initiate */}
							{proposal.status === 'approved' &&
								(proposal.cancelProposal !== null ? (
									<CancelDetailsCard
										cancelProposal={proposal.cancelProposal}
										signerPubkey={signerPubkey}
										onSign={() => navigate(`/proposals/${actionId}/cancel/sign`)}
										onBroadcast={() => navigate(`/proposals/${actionId}/cancel/broadcast`)}
									/>
								) : (
									<div className="rounded-xl border border-[#e5e7eb] bg-white px-6 py-5 shadow-sm">
										<p className="m-0 text-[13px] text-[#6b7280]">
											No cancel proposal initiated yet. Sign on your hardware wallet to start collecting cancel
											signatures.
										</p>
										<button
											type="button"
											className="mt-4 w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-sm font-medium text-white transition hover:bg-black"
											onClick={() => navigate(`/proposals/${actionId}/cancel/sign`)}
										>
											Initiate cancel
										</button>
									</div>
								))}
						</>
					)}
				</div>
			</div>
		</ScreenShell>
	)
}
