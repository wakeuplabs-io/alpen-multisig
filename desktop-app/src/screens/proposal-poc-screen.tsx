import { Navigate, useNavigate } from 'react-router-dom'
import { CreateProposalForm } from '@/domain/create-proposal/components/create-proposal-form'
import { useCreateProposal } from '@/domain/create-proposal/hooks/use-create-proposal'
import { useSession } from '@/hooks/use-session'
import { AuthRole } from '@/types/auth-role'
import { ScreenShell } from '@/screens/screen-shell'

export function ProposalPocScreen() {
  const navigate = useNavigate()
  const { wallet, selectedRole, session, sessionTimeLabel, disconnectSession } = useSession()
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
      <button
        type="button"
        className="inline-flex items-center gap-1.5 text-sm text-[#6b7280] hover:text-[#111827]"
        onClick={() => navigate('/proposals')}
      >
        ← Back to proposals
      </button>

      <CreateProposalForm
        authorityLabel={authorityLabel}
        multisigConfig={createProposal.multisigConfig}
        multisigConfigVersion={createProposal.multisigConfigVersion}
        isLoadingConfig={createProposal.isLoadingConfig}
        isSubmitting={createProposal.isSubmitting}
        error={createProposal.error}
        createdProposal={createProposal.createdProposal}
        onCancel={() => navigate('/proposals')}
        onSubmitValid={(data) => createProposal.submitCreateProposal(data)}
      />
    </ScreenShell>
  )
}
