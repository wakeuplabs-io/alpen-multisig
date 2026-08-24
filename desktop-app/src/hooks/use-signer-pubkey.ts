import { useEffect, useState } from 'react'
import { orchestratorAuthGetSession } from '@/api/orchestrator-auth'

/**
 * The connected signer's pubkey, so a screen can tell the user which approval row is theirs.
 *
 * Screens pass it to each other through router state, which a page refresh throws away — and then
 * the YOU badge and the "you have signed" note silently disappear (#486). When the router state is
 * missing, fall back to the orchestrator session, which holds the same pubkey.
 */
export function useSignerPubkey(fromRouterState: string | null): string | null {
	const [fromSession, setFromSession] = useState<string | null>(null)

	useEffect(() => {
		if (fromRouterState !== null) return

		let cancelled = false
		void orchestratorAuthGetSession().then((res) => {
			if (cancelled) return
			setFromSession(res.ok ? (res.data?.signerPubkey ?? null) : null)
		})

		return () => {
			cancelled = true
		}
	}, [fromRouterState])

	return fromRouterState ?? fromSession
}
