import type { ActionType } from '@/api/proposals'
import type { PastedSignature } from '@/domain/proposal-detail/model/pasted-signature'

export type ManualStep = 'import' | 'sign-collect' | 'broadcast'

export type ManualSignature = {
	signerPubkey: string
	signatureHex: string
	source: 'local' | 'pasted'
}

export type ManualImportForm = {
	actionHex: string
	seqNo: string
	authority: string
}

export type ManualImportErrors = Partial<Record<keyof ManualImportForm, string>>

export type ManualImportData = {
	actionHex: string
	seqNo: number
	authority: string
	sighashHex: string
	/** Resolved by Rust's decoder at import time, never guessed from the hex. */
	actionType: ActionType
}

export type ManualBundleJson = {
	actionHex: string
	seqNo: number
	authority: string
	signatures: Array<{ signerPubkey: string; signatureHex: string }>
}

export type { PastedSignature }
