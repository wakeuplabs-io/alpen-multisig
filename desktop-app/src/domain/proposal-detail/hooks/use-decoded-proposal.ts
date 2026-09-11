import { useEffect, useState } from 'react'
import type { Proposal } from '@/api/proposals'
import { getMultisigConfig } from '@/api/asm-state'
import { decodeActionHex } from '@/api/signing'

export type SignerRow = {
	pubkey: string
	inBefore: boolean
	inAfter: boolean
	isAdded: boolean
	isRemoved: boolean
}

export type SignerSetChange = {
	rows: SignerRow[]
	thresholdBefore: number | null
	thresholdAfter: number
}

export type DecodedProposalData = {
	signerSetChange: SignerSetChange | null
	allSigners: string[]
	isLoading: boolean
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

		void Promise.all([decodeActionHex(proposal.actionHex), getMultisigConfig(proposal.authority)]).then(
			([actionRes, configRes]) => {
				if (cancelled) return
				setIsLoading(false)

				if (configRes.ok) {
					setAllSigners(configRes.data.signers)
				}

				if (actionRes.ok && actionRes.data.kind === 'multisig_update' && configRes.ok) {
					const decoded = actionRes.data
					const config = configRes.data
					const isEnacted = proposal.status === 'enacted'

					const addSet = new Set(decoded.addKeys.map((k) => k.toLowerCase()))
					const removeSet = new Set(decoded.removeKeys.map((k) => k.toLowerCase()))

					let beforeSigners: string[]
					let afterSigners: string[]

					if (isEnacted) {
						afterSigners = config.signers
						beforeSigners = [...config.signers.filter((k) => !addSet.has(k.toLowerCase())), ...decoded.removeKeys]
					} else {
						beforeSigners = config.signers
						afterSigners = [...config.signers.filter((k) => !removeSet.has(k.toLowerCase())), ...decoded.addKeys]
					}

					const rowMap = new Map<string, SignerRow>()
					for (const k of beforeSigners) {
						rowMap.set(k.toLowerCase(), { pubkey: k, inBefore: true, inAfter: false, isAdded: false, isRemoved: false })
					}
					for (const k of afterSigners) {
						const lower = k.toLowerCase()
						const existing = rowMap.get(lower)
						if (existing) {
							existing.inAfter = true
						} else {
							rowMap.set(lower, { pubkey: k, inBefore: false, inAfter: true, isAdded: true, isRemoved: false })
						}
					}
					for (const row of rowMap.values()) {
						if (row.inBefore && !row.inAfter) row.isRemoved = true
						if (!row.inBefore && row.inAfter) row.isAdded = true
					}

					const thresholdBefore = isEnacted ? null : config.threshold
					const thresholdAfter = isEnacted ? config.threshold : decoded.newThreshold

					setSignerSetChange({
						rows: Array.from(rowMap.values()),
						thresholdBefore,
						thresholdAfter,
					})
				}
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
