import { useEffect, useState } from 'react'
import { getMultisigConfig } from '@/api/asm-state'
import type { MultisigTargetAuthority } from '@/api/action-builder'
import type { MultisigConfigSnapshot } from '../model/create-proposal.types'

type ConfigLoadState =
	| { authority: MultisigTargetAuthority; status: 'loading' }
	| { authority: MultisigTargetAuthority; status: 'unavailable' }
	| { authority: MultisigTargetAuthority; status: 'loaded'; config: MultisigConfigSnapshot }

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
	const [loadState, setLoadState] = useState<ConfigLoadState>(() => ({ authority, status: 'loading' }))
	const [version, setVersion] = useState(0)

	useEffect(() => {
		let cancelled = false
		setLoadState({ authority, status: 'loading' })
		getMultisigConfig(authority).then((result) => {
			if (cancelled) return
			if (!result.ok) {
				setLoadState({ authority, status: 'unavailable' })
				return
			}
			setLoadState({
				authority,
				status: 'loaded',
				config: { signers: result.data.signers, threshold: result.data.threshold },
			})
			setVersion((v) => v + 1)
		})
		return () => {
			cancelled = true
		}
	}, [authority])

	const isCurrent = loadState.authority === authority
	const isLoaded = isCurrent && loadState.status === 'loaded'
	return {
		multisigConfig: isLoaded ? loadState.config : null,
		multisigConfigVersion: version,
		isLoadingConfig: !isCurrent || loadState.status === 'loading',
	}
}
