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
      <span>{proposal.status}</span>
      <span>{proposal.updatedAt}</span>
    </li>
  )
}

export default function Past({ sessionToken }: Props) {
  const { state } = useProposals(sessionToken)

  if (state.status === 'loading') return <p>Loading…</p>
  if (state.status === 'error') return <p>Error: {state.message}</p>
  if (state.status !== 'loaded') return null

  const past = state.proposals.filter((p) =>
    ['enacted', 'canceled', 'expired'].includes(p.status)
  )

  return (
    <div>
      <h2>Past Proposals</h2>
      {past.length === 0 ? (
        <p>No past proposals.</p>
      ) : (
        <ul>
          {past.map((p) => (
            <ProposalRow key={p.actionId} proposal={p} />
          ))}
        </ul>
      )}
    </div>
  )
}
