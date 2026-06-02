import { useEffect } from 'react'
import { Navigate, useNavigate, useParams } from 'react-router-dom'
import { ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import type { AdminWalletError } from '@/api/admin-wallet'
import { LogOutMutedIcon, ShieldPurpleIcon } from '@/assets/icons'
import { SessionChip } from '@/components/session-chip'
import { BroadcastDetailsCard } from '@/domain/broadcast-proposal/components/broadcast-details-card'
import { BroadcastFundingSignerBanner } from '@/domain/broadcast-proposal/components/broadcast-funding-signer-banner'
import { BroadcastPhaseProgress } from '@/domain/broadcast-proposal/components/broadcast-phase-progress'
import { useBroadcastProposal } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import type { SignerKind } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import { useAdminWalletInfo } from '@/domain/broadcast-proposal/hooks/use-admin-wallet-info'
import { useAdminWalletUtxos, useAdminWalletSync } from '@/domain/admin-wallet/hooks'
import { useAdminWalletCapability } from '@/domain/admin-wallet/hooks/use-admin-wallet-capability'
import { useWalletPanelState } from '@/domain/admin-wallet/hooks/use-wallet-panel-state'
import { useAdminWalletBalance } from '@/domain/admin-wallet/hooks/use-admin-wallet-balance'
import { useAdminWalletReceiveAddress } from '@/domain/admin-wallet/hooks/use-admin-wallet-receive-address'
import { useAddressesWithBalance } from '@/domain/admin-wallet/hooks/use-addresses-with-balance'
import { WalletPanel } from '@/domain/admin-wallet/components/wallet-panel'
import { WalletPanelHeader } from '@/domain/admin-wallet/components/wallet-panel-header'
import { WalletPanelContent } from '@/domain/admin-wallet/components/wallet-panel-content'
import { useEnsureAdminWalletSession } from '@/domain/admin-wallet/hooks/use-ensure-admin-wallet-session'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { authorityLabelForRole } from '@/lib/authority-label'

type WalletPanelData = {
	isOpen: boolean
	open: () => void
	close: () => void
	balanceSats: number
	isBalanceLoading: boolean
	receiveAddress: string | null
	isAddressesLoading: boolean
	addressRows: ReturnType<typeof useAddressesWithBalance>['data']
	addressRowsLoading: boolean
	addressRowsError: ReturnType<typeof useAddressesWithBalance>['error']
	expandedSection: ReturnType<typeof useWalletPanelState>['expandedSection']
	syncStatus: ReturnType<typeof useAdminWalletSync>['syncStatus']
	isSyncRefreshing: boolean
	syncError: ReturnType<typeof useAdminWalletSync>['error'] | null
	onToggleAddresses: () => void
	onRefreshSync: () => Promise<void>
	disabledError: AdminWalletError | null
}

function useWalletPanelData(isAdminWalletMode: boolean): WalletPanelData {
	const { isOpen, expandedSection, open, close, setExpandedSection } = useWalletPanelState()
	const balanceHook = useAdminWalletBalance()
	const receiveAddressHook = useAdminWalletReceiveAddress()
	const syncHook = useAdminWalletSync()
	const addressesWithBalanceHook = useAddressesWithBalance()

	const walletDisabledError =
		balanceHook.error?.type === 'Disabled' || balanceHook.error?.type === 'RegtestGuardViolation'
			? balanceHook.error
			: receiveAddressHook.error?.type === 'Disabled' || receiveAddressHook.error?.type === 'RegtestGuardViolation'
				? receiveAddressHook.error
				: null

	const receiveAddress = receiveAddressHook.address

	return {
		isOpen,
		open,
		close,
		balanceSats: balanceHook.data?.confirmedSats ?? 0,
		isBalanceLoading: balanceHook.isLoading,
		receiveAddress,
		isAddressesLoading: receiveAddressHook.isLoading,
		addressRows: addressesWithBalanceHook.data,
		addressRowsLoading: addressesWithBalanceHook.isLoading,
		addressRowsError: addressesWithBalanceHook.error,
		expandedSection,
		syncStatus: syncHook.syncStatus,
		isSyncRefreshing: syncHook.isLoading,
		syncError: syncHook.error,
		onToggleAddresses: () => setExpandedSection(expandedSection === 'addresses' ? null : 'addresses'),
		onRefreshSync: async () => {
			await syncHook.triggerSync()
			balanceHook.refresh()
			receiveAddressHook.refresh()
			addressesWithBalanceHook.refresh()
		},
		disabledError: isAdminWalletMode ? walletDisabledError : null,
	}
}

export function BroadcastProposalScreen() {
	const navigate = useNavigate()
	const { actionId } = useParams<{ actionId: string }>()
	const { wallet, adapter, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession } = useSession()
	useEnsureAdminWalletSession(adapter)

	const authorityLabel = authorityLabelForRole(selectedRole)

	const { adminWalletInfo } = useAdminWalletInfo()
	const { canSign, signerKind: rawSignerKind, canSignReason } = useAdminWalletCapability()
	const isAdminWalletMode = adminWalletInfo != null
	const signerKind: SignerKind = rawSignerKind === 'hardware' ? 'hardware' : 'mnemonic'

	const { data: utxos, refresh: refreshUtxos } = useAdminWalletUtxos()
	const { syncStatus, triggerSync } = useAdminWalletSync()

	const panel = useWalletPanelData(isAdminWalletMode)

	// Trigger sync on mount when in admin_wallet mode; refresh UTXOs once sync resolves
	useEffect(() => {
		if (isAdminWalletMode) {
			void triggerSync().then(() => refreshUtxos())
		}
	}, [isAdminWalletMode, triggerSync, refreshUtxos])

	const { phase, bundle, result, proposal, error, prepare, broadcast } = useBroadcastProposal(
		ORCHESTRATOR_BASE_URL,
		actionId ?? '',
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

	const signerLabel = wallet.addressSample
		? `${wallet.addressSample.slice(0, 10)}…${wallet.addressSample.slice(-8)}`
		: 'Unknown'

	const isLoading = phase === 'idle' || phase === 'preparing'
	const showDetails =
		bundle !== null && (phase === 'confirming' || phase === 'awaiting-device' || phase === 'broadcasting')
	const showProgress = phase === 'awaiting-device' || phase === 'broadcasting' || phase === 'done' || phase === 'error'

	const utxoCount = isAdminWalletMode && utxos != null ? utxos.length : undefined
	const lastSyncedAt = isAdminWalletMode ? (syncStatus?.lastSyncedAt ?? null) : undefined
	const syncError = isAdminWalletMode
		? syncStatus?.lastError != null
			? { type: 'SyncIncomplete' as const, message: syncStatus.lastError.message }
			: null
		: undefined

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
						onActivate={() => (panel.isOpen ? panel.close() : panel.open())}
						isActive={panel.isOpen}
						panelId="wallet-slide-dialog"
					/>
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
					onClick={() => navigate('/proposals')}
				>
					← Back to proposals
				</button>

				<h1 className="m-0 mt-3 font-['BIZ_UDPMincho'] text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Broadcast proposal
				</h1>
				<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
					Quorum has been reached. Review the commit details, then broadcast via the commit/reveal flow.
				</p>

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
							utxoCount={utxoCount}
							lastSyncedAt={lastSyncedAt}
							syncError={syncError}
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
							<p className="m-0 text-sm font-medium text-[#065f46]">
								{proposal?.status === 'enacted' || result?.proposalStatus === 'enacted'
									? 'Proposal enacted on-chain.'
									: 'Reveal confirmed on-chain. Proposal stays approved until ASM enactment; refresh the dashboard after the confirmation delay.'}{' '}
								({proposal?.broadcastStatus ?? result?.broadcastStatus ?? '—'}).
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
						<div>
							<button
								type="button"
								data-testid="e2e-broadcast-prepare"
								disabled={!canSign}
								onClick={() => void prepare()}
								className="inline-flex items-center rounded-xl border border-[#111827] bg-[#111827] px-4 py-2 text-sm font-medium text-white transition hover:bg-black disabled:cursor-not-allowed disabled:opacity-60"
							>
								Retry
							</button>
							{!canSign && (
								<p className="mt-2 text-[12px] text-[#6b7280]">{canSignReason ?? 'Hardware wallet required to sign'}</p>
							)}
						</div>
					)}
				</div>
			</div>

			<WalletPanel isOpen={panel.isOpen} onClose={panel.close} panelId="wallet-slide-dialog">
				<WalletPanelHeader onClose={panel.close} title={`Session · ${sessionTimeLabel}`} subtitle={signerLabel} />
				<WalletPanelContent
					disabledError={panel.disabledError}
					balanceSats={panel.balanceSats}
					isBalanceLoading={panel.isBalanceLoading}
					receiveAddress={panel.receiveAddress}
					isAddressesLoading={panel.isAddressesLoading}
					addressRows={panel.addressRows}
					addressRowsLoading={panel.addressRowsLoading}
					addressRowsError={panel.addressRowsError}
					expandedSection={panel.expandedSection}
					onToggleAddresses={panel.onToggleAddresses}
					syncStatus={panel.syncStatus}
					isSyncRefreshing={panel.isSyncRefreshing}
					syncError={panel.syncError}
					onRefreshSync={panel.onRefreshSync}
				/>
			</WalletPanel>
		</ScreenShell>
	)
}
