export type WalletVendor = 'mock' | 'ledger' | 'trezor' | 'mnemonic'

export type WalletAdapterOptions = {
	/** Required when vendor is 'mnemonic'. */
	mnemonic?: string
	derivationPath?: string
}

export type WalletAccountInfo = {
	deviceLabel: string
	derivationPath: string
	addressSample?: string
	xpubOrFingerprint?: string
	/** Display label for xpubOrFingerprint (e.g. 'Public key', 'xpub', 'Fingerprint') */
	keyLabel?: string
}

export type SignTestPayloadResult = {
	signatureHex: string
	note?: string
}

export type SignatureFormat = 'raw-ecdsa' | 'bitcoin-message' | 'p2wpkh-tx-binding'

export type SignSighashResult = {
	publicKeyHex: string
	signatureHex: string
	/** Indicates how the signature was produced so the verifier picks the right hash. */
	signatureFormat: SignatureFormat
}

export type WalletAdapter = {
	readonly vendor: WalletVendor
	readonly supportsSighashSigning: boolean
	connect(): Promise<WalletAccountInfo>
	disconnect(): Promise<void>
	signTestPayload(payloadUtf8: string): Promise<SignTestPayloadResult>
	signSighash(sighashHex: string): Promise<SignSighashResult>
}
