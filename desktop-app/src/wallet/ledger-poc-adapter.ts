import { tauriCall } from '@/api/tauri-bridge'
import type { SignSighashResult, WalletAccountInfo, WalletAdapter } from './types'

/** BIP-84 first receive address — must match the Rust DEFAULT_PATH. */
const DEFAULT_DERIVATION_PATH = "86'/0'/73'/0/0"

type HwWalletInfo = {
	deviceLabel: string
	derivationPath: string
	addressSample?: string
	xpubOrFingerprint?: string
	keyLabel?: string
}

type SignatureResult = {
	publicKeyHex: string
	signatureHex: string
}

export function createLedgerPocAdapter(): WalletAdapter {
	let derivationPath = DEFAULT_DERIVATION_PATH
	let publicKeyHex: string | null = null

	return {
		vendor: 'ledger',
		supportsSighashSigning: true,

		async connect(): Promise<WalletAccountInfo> {
			const result = await tauriCall<HwWalletInfo>('get_ledger_info', { derivationPath })
			if (!result.ok) throw new Error(result.error)
			const info = result.data
			derivationPath = info.derivationPath
			publicKeyHex = info.xpubOrFingerprint ?? null
			return {
				deviceLabel: info.deviceLabel,
				derivationPath: info.derivationPath,
				addressSample: info.addressSample,
				xpubOrFingerprint: info.xpubOrFingerprint,
				keyLabel: info.keyLabel,
			}
		},

		async disconnect(): Promise<void> {
			publicKeyHex = null
		},

		async signSighash(sighashHex: string): Promise<SignSighashResult> {
			if (!publicKeyHex) throw new Error('Connect the Ledger first.')
			const result = await tauriCall<SignatureResult>('sign_with_ledger', { sighashHex, derivationPath })
			if (!result.ok) throw new Error(result.error)
			return {
				publicKeyHex: result.data.publicKeyHex,
				signatureHex: result.data.signatureHex,
				signatureFormat: 'bitcoin-message',
			}
		},
	}
}
