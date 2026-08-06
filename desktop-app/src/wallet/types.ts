export type WalletVendor = 'mock' | 'ledger' | 'trezor' | 'mnemonic'

export type WalletAdapterOptions = {
	/** Required when vendor is 'mnemonic'. */
	mnemonic?: string
	/**
	 * BIP39 passphrase for the software wallet only. Hardware vendors never take one:
	 * a Trezor passphrase is entered on the device keypad, never on this machine (#448).
	 */
	passphrase?: string
	derivationPath?: string
}

export type WalletAccountInfo = {
	deviceLabel: string
	derivationPath: string
	addressSample?: string
	/** Full compressed pubkey hex — required for ASM membership checks on HW connect. */
	publicKeyHex?: string
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

/**
 * Which wallet behind a Trezor seed to open. One seed backs the standard wallet plus a
 * distinct wallet per passphrase, and the choice is made per connection — the device does
 * not remember one. 'hidden' hands entry to the device keypad; neither value carries a
 * secret from this machine.
 */
export type WalletKind = 'standard' | 'hidden'

export type WalletAdapter = {
	readonly vendor: WalletVendor
	readonly supportsSighashSigning: boolean
	connect(kind?: WalletKind): Promise<WalletAccountInfo>
	disconnect(): Promise<void>
	signSighash(sighashHex: string, context?: SigningContext): Promise<SignSighashResult>
	getAccountXpub?(): Promise<string>
	getMasterFingerprint?(): Promise<number>
}
