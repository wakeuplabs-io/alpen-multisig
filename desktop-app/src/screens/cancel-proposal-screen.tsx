import { Navigate, useLocation, useNavigate, useParams } from 'react-router-dom'
import { getOrchestratorBaseUrl } from '@/api/orchestrator-auth'
import { ShieldAccentIcon } from '@/assets/icons'
import { ActivationCountdown } from '@/domain/cancel-proposal/components/activation-countdown'
import { CancelDetailsCard } from '@/domain/cancel-proposal/components/cancel-details-card'
import { CancelTargetSummary } from '@/domain/cancel-proposal/components/cancel-target-summary'
import { useDecodedProposal } from '@/domain/proposal-detail/hooks/use-decoded-proposal'
import { useProposalDetail } from '@/domain/proposal-detail/hooks/use-proposal-detail'
import { useBlockHeight } from '@/hooks/use-block-height'
import { useSession } from '@/hooks/use-session'
import { Breadcrumbs } from '@/components/breadcrumbs'
import { DisconnectButton } from '@/components/disconnect-button'
import { ScreenShell } from '@/screens/screen-shell'
import { authorityLabelForRole } from '@/lib/authority-label'
import { deviceCopy } from '@/lib/device-copy'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'

const CANCELABLE_AUTHORITIES = ['alpen_admin', 'strata_admin']

type LocationState = { signerPubkey?: string | null }

export function CancelProposalScreen() {
	const navigate = useNavigate()
	const location = useLocation()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, adapter, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession } = useSession()
	const signerPubkey: string | null = (location.state as LocationState)?.signerPubkey ?? null
	const authorityLabel = authorityLabelForRole(selectedRole)
	const signerLabel = deviceCopy(adapter.vendor).label
	const panel = useWalletPanelData()

	const { proposal, isLoading, error, reload } = useProposalDetail(getOrchestratorBaseUrl(), actionId ?? '')
	const decodedData = useDecodedProposal(proposal)
	const currentBlockHeight = useBlockHeight()

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
						adminId={wallet.addressSample}
					/>
					<DisconnectButton onClick={() => void handleBack()} />
				</>
			}
		>
			<div className="mx-auto w-full max-w-190">
				<Breadcrumbs />

				<h1 className="m-0 mt-3 font-display text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Cancel proposal
				</h1>
				<p className="m-0 mt-1 text-body-sm text-[#6b7280]">
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
						<div className="rounded-xl border border-danger-border bg-danger-surface px-4 py-3">
							<p className="m-0 text-body text-danger-deep">{error}</p>
							<button
								type="button"
								className="mt-2 rounded-md border border-danger-deep bg-white px-3 py-1 text-label font-medium text-danger-deep transition hover:bg-danger-surface"
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
								<div className="rounded-xl border border-danger-border bg-danger-surface px-4 py-3">
									<p className="m-0 text-body font-medium text-danger-deep">
										This proposal has already been enacted. Cancellation is no longer possible.
									</p>
								</div>
							)}

							{/* Activation countdown */}
							{proposal.activationHeight !== null && proposal.status === 'approved' && (
								<div className="rounded-xl border border-accent-border bg-highlight-surface px-4 py-3">
									<ActivationCountdown
										activationHeight={proposal.activationHeight}
										currentHeight={currentBlockHeight}
									/>
								</div>
							)}

							{/* Target proposal summary */}
							{proposal.status === 'approved' && <CancelTargetSummary proposal={proposal} decodedData={decodedData} />}

							{/* Cancel details or prompt to initiate */}
							{proposal.status === 'approved' &&
								(proposal.cancelProposal !== null ? (
									<CancelDetailsCard
										cancelProposal={proposal.cancelProposal}
										signerPubkey={signerPubkey}
										walletVendor={adapter.vendor}
										onSign={() => navigate(`/proposals/${actionId}/cancel/sign`)}
										onBroadcast={() => navigate(`/proposals/${actionId}/cancel/broadcast`)}
									/>
								) : (
									<div className="rounded-xl border border-[#e5e7eb] bg-white px-6 py-5 shadow-sm">
										<p className="m-0 text-body-sm text-[#6b7280]">
											No cancel proposal initiated yet. Sign with your {signerLabel} to start collecting cancel
											signatures.
										</p>
										<button
											type="button"
											className="mt-4 w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black"
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
