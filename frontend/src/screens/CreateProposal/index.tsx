import type { Session } from '@/types'

type Props = {
  session: Session
  onCreated: (actionId: string) => void
  onCancel: () => void
}

export default function CreateProposal({ session, onCreated, onCancel }: Props) {
  // TODO: build proposal form driven by action type
  // Action types are defined by SPS-50/51/65 per authority
  return (
    <div>
      <h1>Create Proposal</h1>
      <p>Authority: {session.authority}</p>
      <button onClick={onCancel}>Cancel</button>
    </div>
  )
}
