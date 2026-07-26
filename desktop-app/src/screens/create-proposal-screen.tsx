import { Navigate, useNavigate } from 'react-router-dom'
import { CreateProposalForm } from '@/domain/create-proposal/components/create-proposal-form'
import { useCreateProposal } from '@/domain/create-proposal/hooks/use-create-proposal'
import { useSession } from '@/hooks/use-session'
import { authorityFromRole } from '@/api/orchestrator-auth'
import { authorityLabelForRole } from '@/lib/authority-label'
import { Breadcrumbs } from '@/components/breadcrumbs'
import { ScreenShell } from '@/screens/screen-shell'
import { LogOutMutedIcon, ShieldAccentIcon } from '@/assets/icons'
import { useWalletPanelData } from '@/domain/admin-wallet/hooks/use-wallet-panel-data'
import { WalletSessionControl } from '@/domain/admin-wallet/components/wallet-session-control'

export function CreateProposalScreen() {
	const navigate = useNavigate()
	const {
		wallet,
		adapter,
		selectedRole,
		sessionTimeLabel,
		sessionWarning,
		disconnectSession,
		connectOrchestratorSession,
	} = useSession()
	const createProposal = useCreateProposal()
	const panel = useWalletPanelData()

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	const authorityLabel = authorityLabelForRole(selectedRole)
	const authority = authorityFromRole(selectedRole)

	async function handleDisconnect() {
		await disconnectSession()
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
						adminId={wallet.publicKeyHex}
					/>
					<button
						type="button"
						className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-label font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleDisconnect()}
					>
						<LogOutMutedIcon width={12} height={12} className="block shrink-0" />
						Disconnect
					</button>
				</>
			}
		>
			<div className="mx-auto flex w-full max-w-[760px] flex-col gap-5">
				<Breadcrumbs />
				<CreateProposalForm
					authorityLabel={authorityLabel}
					authority={authority}
					walletVendor={adapter.vendor}
					multisigConfig={createProposal.multisigConfig}
					multisigConfigVersion={createProposal.multisigConfigVersion}
					isLoadingConfig={createProposal.isLoadingConfig}
					nextSeqNo={createProposal.nextSeqNo}
					isLoadingSeqNo={createProposal.isLoadingSeqNo}
					currentVk={createProposal.currentVk}
					isLoadingCurrentVk={createProposal.isLoadingCurrentVk}
					currentOperators={createProposal.currentOperators}
					isLoadingOperators={createProposal.isLoadingOperators}
					isSubmitting={createProposal.isSubmitting}
					error={createProposal.error}
					createdProposal={createProposal.createdProposal}
					onCancel={() => navigate('/proposals')}
					onPreviewValid={(data) => createProposal.computeProposalPreview(data)}
					onSubmitValid={(data) => createProposal.submitCreateProposal(data)}
					onReauthenticate={connectOrchestratorSession}
				/>
			</div>
		</ScreenShell>
	)
}
