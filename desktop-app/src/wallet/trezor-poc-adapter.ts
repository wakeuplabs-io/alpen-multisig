import { tauriCall } from '@/api/tauri-bridge'
import type { HwAddressEntry, SignSighashResult, WalletAccountInfo, WalletAdapter } from './types'

/** Product default path. Must match Rust `DEFAULT_PATH`. */
const DEFAULT_DERIVATION_PATH = "m/86'/0'/73'/0/0"

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

export function createTrezorPocAdapter(): WalletAdapter {
	let derivationPath = DEFAULT_DERIVATION_PATH
	let publicKeyHex: string | null = null

	return {
		vendor: 'trezor',
		supportsSighashSigning: true,

		async connect(): Promise<WalletAccountInfo> {
			const result = await tauriCall<HwWalletInfo>('get_trezor_info', { derivationPath })
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
		setDerivationPath(nextPath: string): void {
			derivationPath = nextPath
		},

		async signSighash(sighashHex: string): Promise<SignSighashResult> {
			if (!publicKeyHex) throw new Error('Connect the Trezor first.')
			const result = await tauriCall<SignatureResult>('sign_with_trezor', { sighashHex, derivationPath })
			if (!result.ok) throw new Error(result.error)
			return {
				publicKeyHex: result.data.publicKeyHex,
				signatureHex: result.data.signatureHex,
				signatureFormat: 'bitcoin-message',
			}
		},

		async listAddresses(count = 20): Promise<HwAddressEntry[]> {
			const result = await tauriCall<HwAddressEntry[]>('list_hw_addresses', { count })
			if (!result.ok) throw new Error(result.error)
			return result.data
		},
	}
}
