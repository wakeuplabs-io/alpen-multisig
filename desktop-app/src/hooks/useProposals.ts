import { useCallback, useEffect, useState } from 'react'
import { listProposals } from '@/api/proposals'
import type { Proposal, ProposalStatus } from '@/types'

type ProposalsState =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'loaded'; proposals: Proposal[] }
  | { status: 'error'; message: string }

export function useProposals(token: string | null, filter?: ProposalStatus) {
  const [state, setState] = useState<ProposalsState>({ status: 'idle' })

  const load = useCallback(async () => {
    if (!token) return
    setState({ status: 'loading' })
    const result = await listProposals(token, filter)
    if (result.ok) {
      setState({ status: 'loaded', proposals: result.data.proposals })
    } else {
      setState({ status: 'error', message: result.error })
    }
  }, [token, filter])

  useEffect(() => {
    load()
  }, [load])

  return { state, reload: load }
}
