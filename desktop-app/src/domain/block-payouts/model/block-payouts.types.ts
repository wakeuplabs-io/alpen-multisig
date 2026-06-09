export type BlockPayoutInput = {
	outpoint: string
	amount: number
	claimId: string
	isConflicting: boolean
}

export type PendingBlockPayoutTx = {
	id: string
	inputs: BlockPayoutInput[]
	signaturesReceived: number
	signaturesRequired: number
	signedByCurrentUser: boolean
	expiresAt: Date
	createdAt: Date
	rawTx: string
	signatures: string[]
}

export type PastBlockPayoutTx = {
	id: string
	inputs: BlockPayoutInput[]
	status: 'unconfirmed' | 'confirmed'
	broadcastAt: Date
	blockTimestamp?: Date
	rawTx: string
}

export type CreateBlockPayoutDraft = {
	inputs: BlockPayoutInput[]
	feeRateSatPerVb: number
}

export type FalseClaimReport = {
	claimId: string
	outpoint: string
	amount: number
	proof: string
}

export type ParsedReport = { valid: true; report: FalseClaimReport } | { valid: false; reason: string; raw: string }

export type ActiveModal = { kind: 'sign'; txId: string } | { kind: 'paste'; txId: string } | { kind: 'create' } | null
