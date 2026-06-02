export type WalletVendor = 'mock' | 'ledger' | 'trezor' | 'mnemonic'

export type WalletAdapterOptions = {
	/** Required when vendor is 'mnemonic'. */
	mnemonic?: string
	passphrase?: string
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

export type SignatureFormat = 'raw-ecdsa' | 'bitcoin-message' | 'p2wpkh-tx-binding'

export type SignSighashResult = {
	publicKeyHex: string
	signatureHex: string
	/** Indicates how the signature was produced so the verifier picks the right hash. */
	signatureFormat: SignatureFormat
}

export type HwAddressEntry = {
	index: number
	derivationPath: string
	address: string
	publicKeyHex: string
}

export type SigningContext = {
	seqno: number
	actionHex: string
}

export type WalletAdapter = {
	readonly vendor: WalletVendor
	readonly supportsSighashSigning: boolean
	connect(): Promise<WalletAccountInfo>
	disconnect(): Promise<void>
	signSighash(sighashHex: string, context?: SigningContext): Promise<SignSighashResult>
	getAccountXpub?(): Promise<string>
	getMasterFingerprint?(): Promise<number>
}
