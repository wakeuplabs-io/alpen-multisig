import { Navigate, useNavigate } from 'react-router-dom'
import { CreateProposalForm } from '@/domain/create-proposal/components/create-proposal-form'
import { useCreateProposal } from '@/domain/create-proposal/hooks/use-create-proposal'
import { useSession } from '@/hooks/use-session'
import { AuthRole } from '@/types/auth-role'
import { ScreenShell } from '@/screens/screen-shell'

export function CreateProposalScreen() {
	const navigate = useNavigate()
	const { wallet, selectedRole, session, sessionTimeLabel, disconnectSession, connectSession } = useSession()
	const createProposal = useCreateProposal()

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	const authorityLabel =
		selectedRole === AuthRole.StrataAdministrator ? 'Alpen Administrator' : 'Alpen Sequencer Manager'

	const signerLabel = wallet.addressSample
		? `${wallet.addressSample.slice(0, 6)}...${wallet.addressSample.slice(-6)}`
		: 'Unknown'

	const sessionLabel = session === null ? 'Session' : `Session · ${sessionTimeLabel}`

	async function handleDisconnect() {
		await disconnectSession()
	}

	return (
		<ScreenShell
			headerContent={
				<>
					<span className="inline-flex items-center rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-2.5 py-1 text-xs font-medium text-[#5b44c9]">
						{authorityLabel}
					</span>
					<span className="inline-flex items-center rounded-full border border-[#e5e7eb] bg-[#f8f8fb] px-3 py-1.25 text-xs text-[#111827]">
						{sessionLabel}
					</span>
					<span className="inline-flex items-center rounded-full border border-[#e5e7eb] bg-[#f8f8fb] px-3 py-1.25 font-mono text-[11px] text-[#6b7280]">
						{signerLabel}
					</span>
					<button
						type="button"
						className="inline-flex items-center rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-xs font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleDisconnect()}
					>
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
		</ScreenShell>
	)
}
