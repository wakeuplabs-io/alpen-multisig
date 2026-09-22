import { useEffect, useState } from 'react'
import type { ApiResult } from '@/types'
import type { Proposal } from '@/api/proposals'
import { getMultisigConfig, getSafeHarborStatus, type MultisigConfig } from '@/api/asm-state'
import { decodeActionHex, type DecodedAction } from '@/api/signing'
import {
	buildSafeHarborChange,
	type SafeHarborChange,
} from '@/domain/safe-harbor-change/model/build-safe-harbor-change'
import { multisigUpdateTargetAuthority } from '@/lib/multisig-update-target'
import {
	buildSignerSetChange,
	type SignerRow,
	type SignerSetChange,
} from '@/domain/signer-set-change/model/build-signer-set-change'

export type { SignerRow, SignerSetChange }
export type { SafeHarborChange }

export type DecodedProposalData = {
	signerSetChange: SignerSetChange | null
	/**
	 * Where the bridge sweeps to, and where this proposal would send it instead. Null for every
	 * action that is not a safe harbor rotation, and also when the installed destination could not
	 * be read — see `buildSafeHarborChange`.
	 */
	safeHarborChange: SafeHarborChange | null
	/**
	 * The bridge's live safe harbor flag, read only where it is already being read: the same
	 * `getSafeHarborStatus` call the rotation's change table comes from. `false` everywhere else,
	 * which no consumer can mistake for a claim, since every one of them also asks whether the
	 * action is a rotation.
	 */
	safeHarborActivated: boolean
	allSigners: string[]
	isLoading: boolean
}

const NOTHING_DECODED: DecodedProposalData = {
	signerSetChange: null,
	safeHarborChange: null,
	safeHarborActivated: false,
	allSigners: [],
	isLoading: false,
}

type KeyedDecodedProposalData = DecodedProposalData & {
	proposalKey: string | null
}

/**
 * Builds the Before/After table from a decoded action and the config it should read against, or
 * suppresses it (returns `null`) rather than guess: a failed config read for the *target* falls back here too, never to
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
	const proposalKey = proposal === null ? null : `${proposal.actionId}:${proposal.status}`
	const [decodedData, setDecodedData] = useState<KeyedDecodedProposalData>({
		...NOTHING_DECODED,
		proposalKey: null,
	})

	useEffect(() => {
		if (proposal === null) {
			setDecodedData({
				...NOTHING_DECODED,
				proposalKey: null,
			})
			return
		}

		let cancelled = false
		setDecodedData({
			...NOTHING_DECODED,
			proposalKey,
			isLoading: true,
		})

		// `allSigners` is the pending-signer roster `ApprovalsList` derives its rows from — it must
		// always read the proposal's own authority, never the target of the action it decodes to
		//. This call is unchanged by the retarget below: same request, same timing, so the
		// approval surface carries zero regression risk from this commit.
		void Promise.all([decodeActionHex(proposal.actionHex), getMultisigConfig(proposal.authority)]).then(
			([actionRes, ownConfigRes]) => {
				if (cancelled) return

				const allSigners = ownConfigRes.ok ? ownConfigRes.data.signers : []

				const target = actionRes.ok ? multisigUpdateTargetAuthority(actionRes.data) : null

				if (!actionRes.ok) {
					setDecodedData({
						...NOTHING_DECODED,
						proposalKey,
						allSigners,
					})
					return
				}

				// The destination change, issued only for the action that has one. A second read for
				// the same reason the retarget below is: the answer is only known after the decode,
				// so one `Promise.all` cannot cover both. It re-checks `cancelled` on its own, and
				// `allSigners` above is untouched by it.
				if (actionRes.data.kind === 'safe_harbour_address_update') {
					const proposed = {
						address: actionRes.data.address,
						addressHex: actionRes.data.addressHex,
					}
					void getSafeHarborStatus().then((harborRes) => {
						if (cancelled) return
						setDecodedData({
							...NOTHING_DECODED,
							proposalKey,
							safeHarborActivated: harborRes.ok && harborRes.data.activated,
							safeHarborChange: buildSafeHarborChange({
								installed: harborRes.ok
									? { address: harborRes.data.address, addressHex: harborRes.data.addressHex }
									: null,
								proposed,
								isEnacted: proposal.status === 'enacted',
							}),
							allSigners,
						})
					})
					return
				}

				// A decoded view must not outlive the action it decoded: a successful decode of an
				// action that carries no signer-set change (a Defcon lever, a VK update) blanks the
				// table, or `deriveProposalTitle` would go on titling a Defcon 1 "Add 2 signers".
				if (target === null) {
					setDecodedData({
						...NOTHING_DECODED,
						proposalKey,
						allSigners,
					})
					return
				}

				const action = actionRes.data

				if (target === proposal.authority) {
					// No retarget: the config already read above for `allSigners` is also the target's.
					setDecodedData({
						...NOTHING_DECODED,
						proposalKey,
						signerSetChange: buildTableOrNull(action, ownConfigRes, proposal),
						allSigners,
					})
					return
				}

				// Retarget: the decoded action modifies an authority other than the proposal's
				// own (a council rotation, authored and persisted under `strata_admin`). The target is
				// only known after the decode, so this second read is issued here, conditionally, and
				// re-checks `cancelled` on its own — `allSigners` above is untouched by it.
				void getMultisigConfig(target).then((targetConfigRes) => {
					if (cancelled) return
					setDecodedData({
						...NOTHING_DECODED,
						proposalKey,
						signerSetChange: buildTableOrNull(action, targetConfigRes, proposal),
						allSigners,
					})
				})
			},
		)

		return () => {
			cancelled = true
		}
		// Re-run when proposal identity or status changes
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [proposal?.actionId, proposal?.status])

	// Effects run after paint. Keying the state makes the render immediately following a proposal
	// change return an empty loading view instead of exposing the previous proposal's signer data.
	if (decodedData.proposalKey !== proposalKey) {
		return {
			...NOTHING_DECODED,
			isLoading: proposal !== null,
		}
	}
	return decodedData
}
