import { useEffect, useState } from 'react'
import type { ApiResult } from '@/types'
import type { Proposal } from '@/api/proposals'
import { getMultisigConfig, type MultisigConfig } from '@/api/asm-state'
import { decodeActionHex, type DecodedAction } from '@/api/signing'
import { multisigUpdateTargetAuthority } from '@/lib/multisig-update-target'
import {
	buildSignerSetChange,
	type SignerRow,
	type SignerSetChange,
} from '@/domain/proposal-detail/model/build-signer-set-change'

export type { SignerRow, SignerSetChange }

export type DecodedProposalData = {
	signerSetChange: SignerSetChange | null
	allSigners: string[]
	isLoading: boolean
}

/**
 * Builds the Before/After table from a decoded action and the config it should read against, or
 * suppresses it (returns `null`) rather than guess. Suppression is Phase 1 behaviour, preserved
 * on purpose (§4.9): a failed config read for the *target* falls back here too, never to
 * rendering against some other config.
 */
function buildTableOrNull(
	action: DecodedAction,
	configRes: ApiResult<MultisigConfig>,
	proposal: Proposal,
): SignerSetChange | null {
	if (!configRes.ok || action.kind !== 'multisig_update') return null

	return buildSignerSetChange({
		signers: configRes.data.signers,
		threshold: configRes.data.threshold,
		addKeys: action.addKeys,
		removeKeys: action.removeKeys,
		newThreshold: action.newThreshold,
		isEnacted: proposal.status === 'enacted',
	})
}

export function useDecodedProposal(proposal: Proposal | null): DecodedProposalData {
	const [signerSetChange, setSignerSetChange] = useState<SignerSetChange | null>(null)
	const [allSigners, setAllSigners] = useState<string[]>([])
	const [isLoading, setIsLoading] = useState(false)

	useEffect(() => {
		if (proposal === null) {
			setSignerSetChange(null)
			setAllSigners([])
			return
		}

		let cancelled = false
		setIsLoading(true)

		// `allSigners` is the pending-signer roster `ApprovalsList` derives its rows from — it must
		// always read the proposal's own authority, never the target of the action it decodes to
		// (§4.9). This call is unchanged by the retarget below: same request, same timing, so the
		// approval surface carries zero regression risk from this commit.
		void Promise.all([decodeActionHex(proposal.actionHex), getMultisigConfig(proposal.authority)]).then(
			([actionRes, ownConfigRes]) => {
				if (cancelled) return

				if (ownConfigRes.ok) {
					setAllSigners(ownConfigRes.data.signers)
				}

				const target = actionRes.ok ? multisigUpdateTargetAuthority(actionRes.data) : null

				// A decoded view must not outlive the action it decoded. Narrowed to a successful
				// decode with a target on purpose: a failed decode says nothing about whether this
				// proposal has a signer-set change, and blanking the table on a transient RPC error
				// would lose information rather than correct it — see `buildTableOrNull` for the
				// matching narrowing on the config side.
				if (!actionRes.ok || target === null) {
					setIsLoading(false)
					setSignerSetChange(null)
					return
				}

				const action = actionRes.data

				if (target === proposal.authority) {
					// No retarget: the config already read above for `allSigners` is also the target's.
					setIsLoading(false)
					setSignerSetChange(buildTableOrNull(action, ownConfigRes, proposal))
					return
				}

				// Retarget (§4.9): the decoded action modifies an authority other than the proposal's
				// own (a council rotation, authored and persisted under `strata_admin`). The target is
				// only known after the decode, so this second read is issued here, conditionally, and
				// re-checks `cancelled` on its own — `allSigners` above is untouched by it.
				void getMultisigConfig(target).then((targetConfigRes) => {
					if (cancelled) return
					setIsLoading(false)
					setSignerSetChange(buildTableOrNull(action, targetConfigRes, proposal))
				})
			},
		)

		return () => {
			cancelled = true
		}
		// Re-run when proposal identity or status changes
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [proposal?.actionId, proposal?.status])

	return { signerSetChange, allSigners, isLoading }
}
