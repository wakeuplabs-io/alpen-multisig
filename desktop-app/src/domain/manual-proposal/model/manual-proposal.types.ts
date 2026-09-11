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
}

export type ManualBundleJson = {
	actionHex: string
	seqNo: number
	authority: string
	signatures: Array<{ signerPubkey: string; signatureHex: string }>
}

export type { PastedSignature }
