import { useState } from 'react'
import type { Session } from '@/types'
import Pending from './Pending'
import Approved from './Approved'
import Past from './Past'

type Tab = 'pending' | 'approved' | 'past'

type Props = {
  session: Session
}

export default function Dashboard({ session }: Props) {
  const [activeTab, setActiveTab] = useState<Tab>('pending')

  return (
    <div>
      <h1>Multisig Dashboard</h1>
      <p>Authority: {session.authority} · Signer: {session.signerPubkey}</p>

      <nav>
        {(['pending', 'approved', 'past'] as Tab[]).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            aria-current={activeTab === tab ? 'page' : undefined}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </nav>

      {activeTab === 'pending' && <Pending sessionToken={session.sessionId} />}
      {activeTab === 'approved' && <Approved sessionToken={session.sessionId} />}
      {activeTab === 'past' && <Past sessionToken={session.sessionId} />}
    </div>
  )
}
