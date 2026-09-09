import { useEffect, useState } from 'react'
import type { ApiResult } from '@/types'
import type { Proposal } from '@/api/proposals'
import { getMultisigConfig, getSafeHarbourStatus, type MultisigConfig } from '@/api/asm-state'
import { decodeActionHex, type DecodedAction } from '@/api/signing'
import {
	buildSafeHarbourChange,
	type SafeHarbourChange,
} from '@/domain/safe-harbour-change/model/build-safe-harbour-change'
import { multisigUpdateTargetAuthority } from '@/lib/multisig-update-target'
import {
	buildSignerSetChange,
	type SignerRow,
	type SignerSetChange,
} from '@/domain/signer-set-change/model/build-signer-set-change'

export type { SignerRow, SignerSetChange }
export type { SafeHarbourChange }

export type DecodedProposalData = {
	signerSetChange: SignerSetChange | null
	/**
	 * Where the bridge sweeps to, and where this proposal would send it instead. Null for every
	 * action that is not a safe harbour rotation, and also when the installed destination could not
	 * be read — see `buildSafeHarbourChange`.
	 */
	safeHarbourChange: SafeHarbourChange | null
	allSigners: string[]
	isLoading: boolean
}

type KeyedDecodedProposalData = DecodedProposalData & {
	proposalKey: string | null
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
	const proposalKey = proposal === null ? null : `${proposal.actionId}:${proposal.status}`
	const [decodedData, setDecodedData] = useState<KeyedDecodedProposalData>({
		proposalKey: null,
		signerSetChange: null,
		safeHarbourChange: null,
		allSigners: [],
		isLoading: false,
	})

	useEffect(() => {
		if (proposal === null) {
			setDecodedData({
				proposalKey: null,
				signerSetChange: null,
				safeHarbourChange: null,
				allSigners: [],
				isLoading: false,
			})
			return
		}

		let cancelled = false
		setDecodedData({
			proposalKey,
			signerSetChange: null,
			safeHarbourChange: null,
			allSigners: [],
			isLoading: true,
		})

		// `allSigners` is the pending-signer roster `ApprovalsList` derives its rows from — it must
		// always read the proposal's own authority, never the target of the action it decodes to
		// (§4.9). This call is unchanged by the retarget below: same request, same timing, so the
		// approval surface carries zero regression risk from this commit.
		void Promise.all([decodeActionHex(proposal.actionHex), getMultisigConfig(proposal.authority)]).then(
			([actionRes, ownConfigRes]) => {
				if (cancelled) return

				const allSigners = ownConfigRes.ok ? ownConfigRes.data.signers : []

				const target = actionRes.ok ? multisigUpdateTargetAuthority(actionRes.data) : null

				if (!actionRes.ok) {
					setDecodedData({
						proposalKey,
						signerSetChange: null,
						safeHarbourChange: null,
						allSigners,
						isLoading: false,
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
					void getSafeHarbourStatus().then((harbourRes) => {
						if (cancelled) return
						setDecodedData({
							proposalKey,
							signerSetChange: null,
							safeHarbourChange: buildSafeHarbourChange({
								installed: harbourRes.ok
									? { address: harbourRes.data.address, addressHex: harbourRes.data.addressHex }
									: null,
								proposed,
								isEnacted: proposal.status === 'enacted',
							}),
							allSigners,
							isLoading: false,
						})
					})
					return
				}

				// A decoded view must not outlive the action it decoded: a successful decode of an
				// action that carries no signer-set change (a Defcon lever, a VK update) blanks the
				// table, or `deriveProposalTitle` would go on titling a Defcon 1 "Add 2 signers".
				if (target === null) {
					setDecodedData({
						proposalKey,
						signerSetChange: null,
						safeHarbourChange: null,
						allSigners,
						isLoading: false,
					})
					return
				}

				const action = actionRes.data

				if (target === proposal.authority) {
					// No retarget: the config already read above for `allSigners` is also the target's.
					setDecodedData({
						proposalKey,
						signerSetChange: buildTableOrNull(action, ownConfigRes, proposal),
						safeHarbourChange: null,
						allSigners,
						isLoading: false,
					})
					return
				}

				// Retarget (§4.9): the decoded action modifies an authority other than the proposal's
				// own (a council rotation, authored and persisted under `strata_admin`). The target is
				// only known after the decode, so this second read is issued here, conditionally, and
				// re-checks `cancelled` on its own — `allSigners` above is untouched by it.
				void getMultisigConfig(target).then((targetConfigRes) => {
					if (cancelled) return
					setDecodedData({
						proposalKey,
						signerSetChange: buildTableOrNull(action, targetConfigRes, proposal),
						safeHarbourChange: null,
						allSigners,
						isLoading: false,
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
		return { signerSetChange: null, safeHarbourChange: null, allSigners: [], isLoading: proposal !== null }
	}
	return decodedData
}
