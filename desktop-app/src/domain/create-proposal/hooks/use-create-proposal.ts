import { useEffect, useState } from 'react'
import { getMultisigConfig } from '@/api/asm-state'
import { buildAdminMultisigUpdateHex } from '@/api/action-builder'
import { authorityFromRole, ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import { createProposal, type Proposal } from '@/api/proposals'
import { computeSighash } from '@/api/signing'
import { useSession } from '@/hooks/use-session'
import { useWalletSession } from '@/hooks/use-wallet-session'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import type { MultisigConfigSnapshot } from '../model/create-proposal.types'

export type UseCreateProposalReturn = {
  multisigConfig: MultisigConfigSnapshot | null
  multisigConfigVersion: number
  isLoadingConfig: boolean
  isSubmitting: boolean
  error: string | null
  createdProposal: Proposal | null
  submitCreateProposal: (data: CreateProposalFormValues) => Promise<void>
}

export function useCreateProposal(): UseCreateProposalReturn {
  const { adapter } = useWalletSession()
  const { selectedRole, ensureOrchestratorSession } = useSession()

  const [multisigConfig, setMultisigConfig] = useState<MultisigConfigSnapshot | null>(null)
  const [multisigConfigVersion, setMultisigConfigVersion] = useState(0)
  const [isLoadingConfig, setIsLoadingConfig] = useState(true)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createdProposal, setCreatedProposal] = useState<Proposal | null>(null)

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

  async function submitCreateProposal(formData: CreateProposalFormValues) {
    setError(null)
    setIsSubmitting(true)
    try {
      const seqNo = Number(formData.seqNo.trim())
      if (!Number.isInteger(seqNo) || seqNo < 0) {
        throw new Error('Sequence number must be a valid non-negative integer')
      }

      await ensureOrchestratorSession()

      let actionHex: string
      if (formData.actionType === 'signer_update') {
        const threshold = Number(formData.threshold)
        if (!Number.isInteger(threshold) || threshold < 1 || threshold > 255) {
          throw new Error('Threshold must be an integer between 1 and 255')
        }
        const hexResult = await buildAdminMultisigUpdateHex({
          role: 'strata_admin',
          addKeys: formData.keysToAdd.map((row) => row.value.trim()).filter((k) => k.length > 0),
          removeKeys: formData.keysToRemove.map((row) => row.value.trim()).filter((k) => k.length > 0),
          newThreshold: threshold,
        })
        if (!hexResult.ok) throw new Error(hexResult.error)
        actionHex = hexResult.data.actionHex
      } else {
        if (formData.newVkHex.trim().length === 0) {
          throw new Error('New verification key is required')
        }
        throw new Error('Verification key update is not yet supported by the backend')
      }

      const sighashResult = await computeSighash(seqNo, actionHex)
      if (!sighashResult.ok) throw new Error(sighashResult.error)

      const sig = await adapter.signSighash(sighashResult.data.sighashHex)

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
      setError(String(err))
    } finally {
      setIsSubmitting(false)
    }
  }

  return {
    multisigConfig,
    multisigConfigVersion,
    isLoadingConfig,
    isSubmitting,
    error,
    createdProposal,
    submitCreateProposal,
  }
}
