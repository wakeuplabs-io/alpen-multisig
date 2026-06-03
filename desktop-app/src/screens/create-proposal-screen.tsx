import { Navigate, useNavigate } from 'react-router-dom'
import { CreateProposalForm } from '@/domain/create-proposal/components/create-proposal-form'
import { useCreateProposal } from '@/domain/create-proposal/hooks/use-create-proposal'
import { useSession } from '@/hooks/use-session'
import { AuthRole } from '@/types/auth-role'
import { ScreenShell } from '@/screens/screen-shell'
import { SessionChip } from '@/components/session-chip'
import { LogOutMutedIcon, ShieldPurpleIcon } from '@/assets/icons'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { useAdminWalletCapability } from '@/domain/admin-wallet/hooks/use-admin-wallet-capability'
import { WalletPanel } from '@/domain/admin-wallet/components/wallet-panel'
import { WalletPanelHeader } from '@/domain/admin-wallet/components/wallet-panel-header'
import { WalletPanelContent } from '@/domain/admin-wallet/components/wallet-panel-content'

export function CreateProposalScreen() {
	const navigate = useNavigate()
	const { wallet, selectedRole, sessionTimeLabel, sessionWarning, disconnectSession, connectSession } = useSession()
	const createProposal = useCreateProposal()
	const panel = useWalletPanelData()
	const { canSign } = useAdminWalletCapability()

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	const authorityLabel =
		selectedRole === AuthRole.StrataAdministrator ? 'Alpen Administrator' : 'Alpen Sequencer Manager'

	const signerLabel = wallet.addressSample
		? `${wallet.addressSample.slice(0, 10)}…${wallet.addressSample.slice(-8)}`
		: 'Unknown'

	async function handleDisconnect() {
		await disconnectSession()
	}

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
						onClick={() => void handleDisconnect()}
					>
						<LogOutMutedIcon width={12} height={12} className="block shrink-0" />
						Disconnect
					</button>
				</>
			}
		>
			<div className="mx-auto flex w-full max-w-[760px] flex-col gap-5">
				<CreateProposalForm
					authorityLabel={authorityLabel}
					multisigConfig={createProposal.multisigConfig}
					multisigConfigVersion={createProposal.multisigConfigVersion}
					isLoadingConfig={createProposal.isLoadingConfig}
					nextSeqNo={createProposal.nextSeqNo}
					isLoadingSeqNo={createProposal.isLoadingSeqNo}
					currentVk={createProposal.currentVk}
					isLoadingCurrentVk={createProposal.isLoadingCurrentVk}
					isSubmitting={createProposal.isSubmitting}
					error={createProposal.error}
					createdProposal={createProposal.createdProposal}
					onCancel={() => navigate('/proposals')}
					onPreviewValid={(data) => createProposal.computeProposalPreview(data)}
					onSubmitValid={(data) => createProposal.submitCreateProposal(data)}
					onReauthenticate={connectSession}
				/>
			</div>

			<WalletPanel isOpen={panel.isOpen} onClose={panel.close} panelId="wallet-slide-dialog">
				<WalletPanelHeader
					onClose={panel.close}
					subtitle={`Session · ${sessionTimeLabel} · ${signerLabel}`}
					isWatchOnly={!canSign}
				/>
				<WalletPanelContent
					disabledError={panel.disabledError}
					confirmedBalanceSats={panel.confirmedBalanceSats}
					unconfirmedBalanceSats={panel.unconfirmedBalanceSats}
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
