import { tauriCall } from '@/api/tauri-bridge'
import type { SignSighashResult, WalletAccountInfo, WalletAdapter } from './types'

export type MnemonicAdapterOptions = {
	mnemonic: string
	derivationPath?: string
}

export function createMnemonicPocAdapter(opts: MnemonicAdapterOptions): WalletAdapter {
	let publicKeyHex: string | null = null
	const derivationPath = opts.derivationPath ?? "m/86'/0'/0'/0/0"

	return {
		vendor: 'mnemonic',
		supportsSighashSigning: true,

		async connect(): Promise<WalletAccountInfo> {
			const result = await tauriCall<string>('load_mnemonic_wallet', {
				mnemonic: opts.mnemonic,
				derivationPath,
			})
			if (!result.ok) {
				throw new Error(result.error)
			}
			publicKeyHex = result.data
			return {
				deviceLabel: 'Mnemonic Wallet (BIP39)',
				derivationPath,
				xpubOrFingerprint: publicKeyHex,
				keyLabel: 'Public key',
			}
		},

		async disconnect(): Promise<void> {
			publicKeyHex = null
			await tauriCall('unload_mnemonic_wallet')
		},

		async signSighash(sighashHex: string): Promise<SignSighashResult> {
			if (!publicKeyHex) {
				throw new Error('Connect the mnemonic wallet first.')
			}
			const result = await tauriCall<{ public_key_hex: string; signature_hex: string }>('sign_with_mnemonic_wallet', {
				sighashHex,
			})
			if (!result.ok) {
				throw new Error(result.error)
			}
			return {
				publicKeyHex: result.data.public_key_hex,
				signatureHex: result.data.signature_hex,
				signatureFormat: 'raw-ecdsa',
			}
		},
	}
}
