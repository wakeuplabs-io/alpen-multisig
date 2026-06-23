import { useEffect } from 'react'
import { Navigate, useNavigate, useParams } from 'react-router-dom'
import { getOrchestratorBaseUrl } from '@/api/orchestrator-auth'
import { LogOutMutedIcon, ShieldAccentIcon } from '@/assets/icons'
import { BroadcastDetailsCard } from '@/domain/broadcast-proposal/components/broadcast-details-card'
import { BroadcastFundingSignerBanner } from '@/domain/broadcast-proposal/components/broadcast-funding-signer-banner'
import { BroadcastPhaseProgress } from '@/domain/broadcast-proposal/components/broadcast-phase-progress'
import { BroadcastStepper } from '@/domain/broadcast-proposal/components/broadcast-stepper'
import { useBroadcastProposal } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import type { SignerKind } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import { useFeePresets } from '@/domain/fee-selection/hooks/use-fee-presets'
import { FeeRateSelector } from '@/domain/fee-selection/components/fee-rate-selector'
import { useAdminWalletInfo } from '@/domain/broadcast-proposal/hooks/use-admin-wallet-info'
import { useAdminWalletSync } from '@/domain/admin-wallet/hooks'
import { useAdminWalletCapability } from '@/domain/admin-wallet/hooks/use-admin-wallet-capability'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'
import { useEnsureAdminWalletSession } from '@/domain/admin-wallet/hooks/use-ensure-admin-wallet-session'
import { useSession } from '@/hooks/use-session'
import { Breadcrumbs } from '@/components/breadcrumbs'
import { ScreenShell } from '@/screens/screen-shell'
import { authorityLabelForRole } from '@/lib/authority-label'

export function BroadcastProposalScreen() {
	const navigate = useNavigate()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, adapter, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession } = useSession()
	const { sessionReady } = useEnsureAdminWalletSession(adapter)

	const authorityLabel = authorityLabelForRole(selectedRole)

	const { adminWalletInfo, refresh: refreshAdminWalletInfo } = useAdminWalletInfo(sessionReady)
	const { canSign, signerKind: rawSignerKind, canSignReason } = useAdminWalletCapability()
	const isAdminWalletMode = adminWalletInfo != null
	const signerKind: SignerKind = rawSignerKind === 'hardware' ? 'hardware' : 'mnemonic'

	const { syncStatus, triggerSync } = useAdminWalletSync()

	const panel = useWalletPanelData(isAdminWalletMode)

	// Trigger an Electrum sync on mount when in admin_wallet mode; re-read the funding info once
	// the sync resolves so the card shows the post-sync balance and receive address (the initial
	// fetch races the sync and would otherwise pin a stale 0-sats snapshot).
	useEffect(() => {
		if (isAdminWalletMode) {
			void triggerSync().then(() => refreshAdminWalletInfo())
		}
	}, [isAdminWalletMode, triggerSync, refreshAdminWalletInfo])

	// `null` until presets load: prepare/broadcast stay blocked so we never fall back to a silent default rate.
	const feeState = useFeePresets()
	const feeRateSatPerKvb = feeState.status === 'ready' ? feeState.satPerKvb : null

	const { phase, bundle, result, proposal, error, prepare, broadcast } = useBroadcastProposal(
		getOrchestratorBaseUrl(),
		actionId ?? '',
		feeRateSatPerKvb,
		signerKind,
		adapter,
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

	const isLoading = phase === 'idle' || phase === 'preparing'
	const showDetails =
		bundle !== null && (phase === 'confirming' || phase === 'awaiting-device' || phase === 'broadcasting')
	const showProgress =
		phase === 'awaiting-device' ||
		phase === 'broadcasting' ||
		phase === 'awaiting-confirmation' ||
		phase === 'done' ||
		phase === 'error'

	const lastSyncedAt = isAdminWalletMode ? (syncStatus?.lastSyncedAt ?? null) : undefined
	const syncError = isAdminWalletMode
		? syncStatus?.lastError != null
			? { type: 'SyncIncomplete' as const, message: syncStatus.lastError.message }
			: null
		: undefined

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
					Send proposal
				</h1>
				<p className="m-0 mt-1 text-body-sm text-[#6b7280]">
					Quorum has been reached. Review the commit details, then send via the commit/reveal flow.
				</p>

				<div className="mt-5 rounded-xl border border-[#e5e7eb] bg-white px-6 py-4 shadow-sm">
					<BroadcastStepper phase={phase} />
				</div>

				<div className="mt-6 space-y-4">
					<BroadcastFundingSignerBanner backendSignerKind={rawSignerKind} connectVendor={adapter.vendor} />

					{isLoading && (
						<div className="animate-pulse space-y-3 rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
							<div className="h-7 w-48 rounded-lg bg-[#f3f4f6]" />
							<div className="h-4 w-32 rounded-md bg-[#f3f4f6]" />
							<div className="mt-4 h-1.5 w-full rounded-full bg-[#f3f4f6]" />
							<div className="mt-6 h-10 w-full rounded-xl bg-[#f3f4f6]" />
						</div>
					)}

					{showDetails && (
						<BroadcastDetailsCard
							bundle={bundle}
							proposal={proposal}
							onBroadcast={() => void broadcast()}
							isBroadcasting={phase === 'broadcasting' || phase === 'awaiting-device'}
							canSign={canSign}
							canSignReason={canSignReason}
							phase={phase}
							adminWalletInfo={adminWalletInfo}
							lastSyncedAt={lastSyncedAt}
							syncError={syncError}
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
						<div
							className="rounded-xl border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3"
							data-testid="e2e-broadcast-done-banner"
						>
							<p className="m-0 text-body font-medium text-[#065f46]">
								{proposal?.status === 'enacted' || result?.proposalStatus === 'enacted'
									? 'Proposal enacted on-chain.'
									: 'Reveal confirmed on-chain. Proposal stays approved until ASM enactment; refresh the dashboard after the confirmation delay.'}{' '}
								({proposal?.broadcastStatus ?? result?.broadcastStatus ?? '—'}).
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
						<div>
							<button
								type="button"
								data-testid="e2e-broadcast-prepare"
								disabled={!canSign}
								onClick={() => void prepare()}
								className="inline-flex items-center rounded-xl border border-[#111827] bg-[#111827] px-4 py-2 text-body font-medium text-white transition hover:bg-black disabled:cursor-not-allowed disabled:opacity-60"
							>
								Retry
							</button>
							{!canSign && (
								<p className="mt-2 text-label text-[#6b7280]">{canSignReason ?? 'Hardware wallet required to sign'}</p>
							)}
						</div>
					)}
				</div>
			</div>
		</ScreenShell>
	)
}
