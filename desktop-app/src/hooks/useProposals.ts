import { useCallback, useEffect, useState } from 'react'
import { listProposals } from '@/api/proposals'
import type { Proposal, ProposalStatus } from '@/types'

type ProposalsState =
	| { status: 'idle' }
	| { status: 'loading' }
	| { status: 'loaded'; proposals: Proposal[] }
	| { status: 'error'; message: string }

export function useProposals(filter?: ProposalStatus) {
	const [state, setState] = useState<ProposalsState>({ status: 'idle' })

	const load = useCallback(async () => {
		setState({ status: 'loading' })
		const result = await listProposals(filter)
		if (result.ok) {
			setState({ status: 'loaded', proposals: result.data.proposals })
		} else {
			setState({ status: 'error', message: result.error })
		}
	}, [filter])

	useEffect(() => {
		void load()
	}, [load])

	return { state, reload: load }
}
