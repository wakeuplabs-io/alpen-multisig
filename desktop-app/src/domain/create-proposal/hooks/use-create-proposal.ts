import { useEffect, useState } from 'react'
import { getCurrentOperators, getCurrentVk, getMultisigConfig } from '@/api/asm-state'
import type { CurrentVk } from '@/api/asm-state'
import {
	buildAdminMultisigUpdateHex,
	buildDefcon1ActionHex,
	buildDefcon3ActionHex,
	buildOperatorSetUpdateHex,
	buildSequencerKeyUpdateHex,
	buildVkUpdateHex,
	type BuildActionHexResponse,
} from '@/api/action-builder'
import { authorityFromRole, orchestratorAuthGetSession, getOrchestratorBaseUrl } from '@/api/orchestrator-auth'
import { createProposal, getNextSeqNo, type Proposal } from '@/api/proposals'
import { computeSighash } from '@/api/signing'
import { useSession } from '@/hooks/use-session'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { VK_PREDICATE_TYPE_IDS, type CreateProposalFormValues } from '../model/create-proposal.schema'
import type { MultisigConfigSnapshot, ProposalPreview } from '../model/create-proposal.types'
import type { ApiResult } from '@/types'

export const SESSION_EXPIRED_REAUTH_MESSAGE = 'Session expired. Re-authenticate to continue.'

/** Every builder answers the same shape; a failure here must reach the signer, never a blank hex. */
function unwrapActionHex(result: ApiResult<BuildActionHexResponse>): string {
	if (!result.ok) throw new Error(result.error)
	return result.data.actionHex
}

export function isSessionExpiredReauthError(error: unknown): boolean {
	return String(error).includes(SESSION_EXPIRED_REAUTH_MESSAGE)
}

function normalizePubKeyHex(value: string): string {
	const trimmed = value.trim()
	const no0x = trimmed.startsWith('0x') || trimmed.startsWith('0X') ? trimmed.slice(2) : trimmed
	return no0x.toLowerCase()
}

export type UseCreateProposalReturn = {
	multisigConfig: MultisigConfigSnapshot | null
	multisigConfigVersion: number
	isLoadingConfig: boolean
	nextSeqNo: number | null
	isLoadingSeqNo: boolean
	currentVk: CurrentVk | null
	isLoadingCurrentVk: boolean
	currentOperators: string[]
	isLoadingOperators: boolean
	isSubmitting: boolean
	error: string | null
	createdProposal: Proposal | null
	computeProposalPreview: (data: CreateProposalFormValues) => Promise<ProposalPreview | null>
	submitCreateProposal: (data: CreateProposalFormValues) => Promise<void>
}

export function useCreateProposal(): UseCreateProposalReturn {
	const { adapter } = useWalletSession()
	const { selectedRole } = useSession()

	const [multisigConfig, setMultisigConfig] = useState<MultisigConfigSnapshot | null>(null)
	const [multisigConfigVersion, setMultisigConfigVersion] = useState(0)
	const [isLoadingConfig, setIsLoadingConfig] = useState(true)
	const [nextSeqNo, setNextSeqNo] = useState<number | null>(null)
	const [isLoadingSeqNo, setIsLoadingSeqNo] = useState(true)
	const [currentVk, setCurrentVk] = useState<CurrentVk | null>(null)
	const [isLoadingCurrentVk, setIsLoadingCurrentVk] = useState(true)
	const [currentOperators, setCurrentOperators] = useState<string[]>([])
	const [isLoadingOperators, setIsLoadingOperators] = useState(true)
	const [isSubmitting, setIsSubmitting] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const [createdProposal, setCreatedProposal] = useState<Proposal | null>(null)

	async function assertValidSessionForProposalCreation() {
		const sessionResult = await orchestratorAuthGetSession()
		if (!sessionResult.ok) throw new Error(sessionResult.error)
		if (sessionResult.data === null) {
			throw new Error(SESSION_EXPIRED_REAUTH_MESSAGE)
		}
	}

	async function buildActionHex(formData: CreateProposalFormValues): Promise<string> {
		// Exhaustive on purpose. This used to be an `if` chain whose final `else` built a VK update,
		// so a missing arm did not fail to compile and did not fail loudly — it made the signer sign
		// a vk_update sighash under another action's form. The `never` below is that tripwire.
		switch (formData.actionType) {
			case 'signer_update': {
				const threshold = Number(formData.threshold)
				if (!Number.isInteger(threshold) || threshold < 1 || threshold > 255) {
					throw new Error('Threshold must be an integer between 1 and 255')
				}
				return unwrapActionHex(
					await buildAdminMultisigUpdateHex({
						role: authorityFromRole(selectedRole) as 'strata_admin' | 'sequencer_manager' | 'alpen_admin',
						addKeys: formData.keysToAdd.map((row) => normalizePubKeyHex(row.value)).filter((k) => k.length > 0),
						removeKeys: formData.keysToRemove.map((row) => normalizePubKeyHex(row.value)).filter((k) => k.length > 0),
						newThreshold: threshold,
					}),
				)
			}
			case 'sequencer_key_update':
				return unwrapActionHex(
					await buildSequencerKeyUpdateHex({ newPubKey: normalizePubKeyHex(formData.newSequencerKeyHex) }),
				)
			case 'defcon_1':
				return unwrapActionHex(await buildDefcon1ActionHex())
			case 'defcon_3':
				return unwrapActionHex(await buildDefcon3ActionHex())
			case 'operator_set_update':
				return unwrapActionHex(
					await buildOperatorSetUpdateHex({
						addOperatorKeys: formData.operatorsToAdd.map((r) => r.value.trim()).filter((k) => k.length > 0),
						removeOperatorIndices: formData.operatorIndicesToRemove
							.map((r) => r.value.trim())
							.filter((v) => v.length > 0)
							.map(Number),
					}),
				)
			case 'vk_update':
				return unwrapActionHex(
					await buildVkUpdateHex({
						authority: authorityFromRole(selectedRole),
						typeId: VK_PREDICATE_TYPE_IDS[formData.vkTypeId],
						conditionHex: formData.newVkHex.trim(),
					}),
				)
			default: {
				const unhandled: never = formData.actionType
				throw new Error(`No action builder for ${String(unhandled)}`)
			}
		}
	}

	useEffect(() => {
		let cancelled = false
		setIsLoadingConfig(true)
		getMultisigConfig(authorityFromRole(selectedRole)).then((result) => {
			if (cancelled) return
			setIsLoadingConfig(false)
			if (!result.ok) return
			setMultisigConfig({
				signers: result.data.signers,
				threshold: result.data.threshold,
			})
			setMultisigConfigVersion((v) => v + 1)
		})
		return () => {
			cancelled = true
		}
	}, [selectedRole])

	useEffect(() => {
		let cancelled = false
		setIsLoadingSeqNo(true)
		getNextSeqNo({ baseUrl: getOrchestratorBaseUrl() }).then((result) => {
			if (cancelled) return
			setIsLoadingSeqNo(false)
			if (result.ok) setNextSeqNo(result.data)
		})
		return () => {
			cancelled = true
		}
	}, [])

	useEffect(() => {
		let cancelled = false
		// The current-VK RPC only exposes the OL STF predicate (Strata Admin's VK).
		// For Alpen Admin (EE STF VK), no on-chain lookup is available yet, so the
		// form falls back to its "could not load" state.
		if (authorityFromRole(selectedRole) !== 'strata_admin') {
			setIsLoadingCurrentVk(false)
			setCurrentVk(null)
			return
		}
		setIsLoadingCurrentVk(true)
		getCurrentVk().then((result) => {
			if (cancelled) return
			setIsLoadingCurrentVk(false)
			if (result.ok) setCurrentVk(result.data)
		})
		return () => {
			cancelled = true
		}
	}, [selectedRole])

	useEffect(() => {
		let cancelled = false
		setIsLoadingOperators(true)
		getCurrentOperators().then((result) => {
			if (cancelled) return
			setIsLoadingOperators(false)
			if (result.ok) setCurrentOperators(result.data)
		})
		return () => {
			cancelled = true
		}
	}, [])

	async function submitCreateProposal(formData: CreateProposalFormValues) {
		setError(null)
		setIsSubmitting(true)
		try {
			const seqNo = Number(formData.seqNo.trim())
			if (!Number.isInteger(seqNo) || seqNo < 0) {
				throw new Error('Sequence number must be a valid non-negative integer')
			}

			await assertValidSessionForProposalCreation()

			const actionHex = await buildActionHex(formData)

			const sighashResult = await computeSighash(seqNo, actionHex)
			if (!sighashResult.ok) throw new Error(sighashResult.error)

			const sig = await adapter.signSighash(sighashResult.data.sighashHex, { seqno: seqNo, actionHex })

			const createResult = await createProposal({
				baseUrl: getOrchestratorBaseUrl(),
				seqNo,
				actionHex,
				signerPubkey: sig.publicKeyHex,
				signatureHex: sig.signatureHex,
				title: formData.title.trim() || undefined,
			})
			if (!createResult.ok) throw new Error(createResult.error)
			setCreatedProposal(createResult.data)
		} catch (err) {
			if (isSessionExpiredReauthError(err)) {
				throw err
			}
			setError(String(err))
		} finally {
			setIsSubmitting(false)
		}
	}

	async function computeProposalPreview(formData: CreateProposalFormValues): Promise<ProposalPreview | null> {
		setError(null)
		try {
			const seqNo = Number(formData.seqNo.trim())
			if (!Number.isInteger(seqNo) || seqNo < 0) {
				throw new Error('Sequence number must be a valid non-negative integer')
			}
			await assertValidSessionForProposalCreation()
			const actionHex = await buildActionHex(formData)
			// The sighash is not shown and not carried: signing recomputes it. This call stays as a
			// pre-flight check, so a draft that cannot be signed fails here instead of after the
			// signer has already reviewed it and reached for their device.
			const sighashResult = await computeSighash(seqNo, actionHex)
			if (!sighashResult.ok) throw new Error(sighashResult.error)
			return { seqNo, actionHex }
		} catch (err) {
			if (isSessionExpiredReauthError(err)) {
				throw err
			}
			setError(String(err))
			return null
		}
	}

	return {
		multisigConfig,
		multisigConfigVersion,
		isLoadingConfig,
		nextSeqNo,
		isLoadingSeqNo,
		currentVk,
		isLoadingCurrentVk,
		currentOperators,
		isLoadingOperators,
		isSubmitting,
		error,
		createdProposal,
		computeProposalPreview,
		submitCreateProposal,
	}
}
