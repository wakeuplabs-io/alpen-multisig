import { useEffect, useState } from 'react'
import { getMultisigConfig } from '@/api/asm-state'
import type { MultisigTargetAuthority } from '@/api/action-builder'
import type { MultisigConfigSnapshot } from '../model/create-proposal.types'

type LoadedConfig = { authority: MultisigTargetAuthority; config: MultisigConfigSnapshot }

export type UseMultisigConfigReturn = {
	multisigConfig: MultisigConfigSnapshot | null
	multisigConfigVersion: number
	isLoadingConfig: boolean
}

/**
 * The multisig config read, keyed by whichever authority the caller names — the form's *target*
 * (Constraint 2), not necessarily the session. Lifted out of `useCreateProposal` so it can be
 * called with `multisigTargetAuthority(actionType, authority)` instead of the session alone.
 *
 * Freshness is derived, never stored as its own flag. `useEffect` runs after render, so in the
 * render right after the target changes, a hook that only stored `config` would still hold the
 * previous target's signers while `isLoadingConfig` was still `false` — one frame where the form
 * validates against the wrong signer set with both buttons enabled. Storing which authority the
 * held config belongs to and comparing it against the requested authority closes that frame:
 * `multisigConfig` reads `null` and `isLoadingConfig` reads `true` until a config for the current
 * target has actually landed.
 */
export function useMultisigConfig(authority: MultisigTargetAuthority): UseMultisigConfigReturn {
	const [loaded, setLoaded] = useState<LoadedConfig | null>(null)
	const [version, setVersion] = useState(0)
	const [inFlight, setInFlight] = useState(true)

	useEffect(() => {
		let cancelled = false
		setInFlight(true)
		getMultisigConfig(authority).then((result) => {
			if (cancelled) return
			setInFlight(false)
			if (!result.ok) return
			setLoaded({ authority, config: { signers: result.data.signers, threshold: result.data.threshold } })
			setVersion((v) => v + 1)
		})
		// Cancellation only stops a stale response from landing — it must not lower `isLoadingConfig`,
		// or it would re-open exactly the lying frame the freshness check above closes.
		return () => {
			cancelled = true
		}
	}, [authority])

	const isFresh = loaded !== null && loaded.authority === authority
	return {
		multisigConfig: isFresh ? loaded.config : null,
		multisigConfigVersion: version,
		isLoadingConfig: inFlight || !isFresh,
	}
}
