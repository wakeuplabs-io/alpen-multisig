import { Navigate, useNavigate, useParams } from 'react-router-dom'
import { getOrchestratorBaseUrl } from '@/api/orchestrator-auth'
import { LogOutMutedIcon, ShieldAccentIcon } from '@/assets/icons'
import { BroadcastDetailsCard } from '@/domain/broadcast-proposal/components/broadcast-details-card'
import { BroadcastPhaseProgress } from '@/domain/broadcast-proposal/components/broadcast-phase-progress'
import { BroadcastStepper } from '@/domain/broadcast-proposal/components/broadcast-stepper'
import { useCancelBroadcast } from '@/domain/cancel-proposal/hooks/use-cancel-broadcast'
import { useFeePresets } from '@/domain/fee-selection/hooks/use-fee-presets'
import { FeeRateSelector } from '@/domain/fee-selection/components/fee-rate-selector'
import { useSession } from '@/hooks/use-session'
import { Breadcrumbs } from '@/components/breadcrumbs'
import { ScreenShell } from '@/screens/screen-shell'
import { authorityLabelForRole } from '@/lib/authority-label'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'

export function CancelProposalBroadcastScreen() {
	const navigate = useNavigate()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession } = useSession()

	const authorityLabel = authorityLabelForRole(selectedRole)
	const panel = useWalletPanelData()

	// `null` until presets load: prepare/broadcast stay blocked so we never fall back to a silent default rate.
	const feeState = useFeePresets()
	const feeRateSatPerKvb = feeState.status === 'ready' ? feeState.satPerKvb : null

	const {
		isResolvingCancel,
		targetQueued,
		targetQueuedError,
		cancelResolveError,
		phase,
		bundle,
		result,
		proposal,
		error,
		prepare,
		broadcast,
	} = useCancelBroadcast(getOrchestratorBaseUrl(), actionId ?? '', feeRateSatPerKvb)

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
						addressSample={wallet.addressSample}
					/>
					<button
						type="button"
						className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-label font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleBack()}
					>
						<LogOutMutedIcon width={12} height={12} className="block shrink-0" />
						Disconnect
					</button>
				</>
			}
		>
			<div className="mx-auto w-full max-w-190">
				<Breadcrumbs />

				<h1 className="m-0 mt-3 font-display text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Send cancel tx
				</h1>
				<p className="m-0 mt-1 text-body-sm text-[#6b7280]">
					Cancel signatures have reached quorum. Review the commit details, then send to remove the queued update.
				</p>

				<div className="mt-5 rounded-xl border border-[#e5e7eb] bg-white px-6 py-4 shadow-sm">
					<BroadcastStepper phase={phase} />
				</div>

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
							<p className="m-0 text-body text-[#991b1b]">{cancelResolveError.message}</p>
						</div>
					)}

					{targetQueuedError !== null && (
						<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
							<p className="m-0 text-body text-[#991b1b]">
								Could not verify the target action's queue status: {targetQueuedError}
							</p>
						</div>
					)}

					{showDetails && (
						<BroadcastDetailsCard
							bundle={bundle}
							proposal={proposal}
							onBroadcast={() => void broadcast()}
							isBroadcasting={phase === 'broadcasting' || phase === 'awaiting-device'}
							targetQueued={targetQueued}
							feeSelector={
								feeState.status === 'ready' ? (
									<FeeRateSelector
										presets={feeState.presets}
										selection={feeState.selection}
										onSelectPreset={feeState.setPreset}
										onSetCustomRate={feeState.setCustomRate}
									/>
								) : undefined
							}
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
							<p className="m-0 text-body font-medium text-[#065f46]">
								Cancel transaction confirmed on-chain. The queued update has been removed.
							</p>
							<button
								type="button"
								className="mt-3 inline-flex items-center rounded-md border border-[#065f46] bg-white px-3 py-1.5 text-label font-medium text-[#065f46] transition hover:bg-[#f0fdf4]"
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
							className="inline-flex items-center rounded-xl border border-[#111827] bg-[#111827] px-4 py-2 text-body font-medium text-white transition hover:bg-black"
						>
							Retry
						</button>
					)}
				</div>
			</div>
		</ScreenShell>
	)
}
