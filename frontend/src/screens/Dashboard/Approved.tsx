import { useProposals } from '@/hooks/useProposals'
import type { Proposal } from '@/types'

type Props = {
  sessionToken: string
}

function ProposalRow({ proposal }: { proposal: Proposal }) {
  return (
    <li>
      <span>{proposal.actionId}</span>
      <span>seq: {proposal.seqNo}</span>
      <span>approved · awaiting enactment</span>
    </li>
  )
}

export default function Approved({ sessionToken }: Props) {
  const { state } = useProposals(sessionToken, 'approved')

  if (state.status === 'loading') return <p>Loading…</p>
  if (state.status === 'error') return <p>Error: {state.message}</p>
  if (state.status !== 'loaded') return null

  return (
    <div>
      <h2>Approved Proposals</h2>
      {state.proposals.length === 0 ? (
        <p>No approved proposals.</p>
      ) : (
        <ul>
          {state.proposals.map((p) => (
            <ProposalRow key={p.actionId} proposal={p} />
          ))}
        </ul>
      )}
    </div>
  )
}
