import { useEffect, useState } from 'react'
import { getCurrentOperators, getCurrentVk, getMultisigConfig } from '@/api/asm-state'
import type { CurrentVk } from '@/api/asm-state'
import { buildAdminMultisigUpdateHex, buildOperatorSetUpdateHex, buildVkUpdateHex } from '@/api/action-builder'
import { authorityFromRole, orchestratorAuthGetSession, ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { createProposal, getNextSeqNo, type Proposal } from '@/api/proposals'
import { computeSighash } from '@/api/signing'
import { useSession } from '@/hooks/use-session'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { VK_PREDICATE_TYPE_IDS, type CreateProposalFormValues } from '../model/create-proposal.schema'
import type { MultisigConfigSnapshot } from '../model/create-proposal.types'

export const SESSION_EXPIRED_REAUTH_MESSAGE = 'Session expired. Re-authenticate to continue.'

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
	computeProposalPreview: (data: CreateProposalFormValues) => Promise<string | null>
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
		if (formData.actionType === 'signer_update') {
			const threshold = Number(formData.threshold)
			if (!Number.isInteger(threshold) || threshold < 1 || threshold > 255) {
				throw new Error('Threshold must be an integer between 1 and 255')
			}
			const hexResult = await buildAdminMultisigUpdateHex({
				role: authorityFromRole(selectedRole) as 'strata_admin' | 'sequencer_manager',
				addKeys: formData.keysToAdd.map((row) => normalizePubKeyHex(row.value)).filter((k) => k.length > 0),
				removeKeys: formData.keysToRemove.map((row) => normalizePubKeyHex(row.value)).filter((k) => k.length > 0),
				newThreshold: threshold,
			})
			if (!hexResult.ok) throw new Error(hexResult.error)
			return hexResult.data.actionHex
		}
		if (formData.actionType === 'operator_set_update') {
			const hexResult = await buildOperatorSetUpdateHex({
				addOperatorKeys: formData.operatorsToAdd.map((r) => r.value.trim()).filter((k) => k.length > 0),
				removeOperatorIndices: formData.operatorIndicesToRemove
					.map((r) => r.value.trim())
					.filter((v) => v.length > 0)
					.map(Number),
			})
			if (!hexResult.ok) throw new Error(hexResult.error)
			return hexResult.data.actionHex
		}
		const typeId = VK_PREDICATE_TYPE_IDS[formData.vkTypeId]
		const hexResult = await buildVkUpdateHex({
			authority: authorityFromRole(selectedRole),
			typeId,
			conditionHex: formData.newVkHex.trim(),
		})
		if (!hexResult.ok) throw new Error(hexResult.error)
		return hexResult.data.actionHex
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
		getNextSeqNo({ baseUrl: ORCHESTRATOR_BASE_URL }).then((result) => {
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
		setIsLoadingCurrentVk(true)
		getCurrentVk().then((result) => {
			if (cancelled) return
			setIsLoadingCurrentVk(false)
			if (result.ok) setCurrentVk(result.data)
		})
		return () => {
			cancelled = true
		}
	}, [])

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
				baseUrl: ORCHESTRATOR_BASE_URL,
				seqNo,
				actionHex,
				signerPubkey: sig.publicKeyHex,
				signatureHex: sig.signatureHex,
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

	async function computeProposalPreview(formData: CreateProposalFormValues): Promise<string | null> {
		setError(null)
		try {
			const seqNo = Number(formData.seqNo.trim())
			if (!Number.isInteger(seqNo) || seqNo < 0) {
				throw new Error('Sequence number must be a valid non-negative integer')
			}
			await assertValidSessionForProposalCreation()
			const actionHex = await buildActionHex(formData)
			const sighashResult = await computeSighash(seqNo, actionHex)
			if (!sighashResult.ok) throw new Error(sighashResult.error)
			return sighashResult.data.sighashHex
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
