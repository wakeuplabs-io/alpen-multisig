import { tauriCall } from '@/api/tauri-bridge'
import type { HwAddressEntry, SignSighashResult, WalletAccountInfo, WalletAdapter } from './types'

export type MnemonicAdapterOptions = {
	mnemonic: string
	passphrase?: string
	derivationPath?: string
}

export function createMnemonicPocAdapter(opts: MnemonicAdapterOptions): WalletAdapter {
	let publicKeyHex: string | null = null
	let derivationPath = opts.derivationPath ?? "m/84'/0'/73'/0/0"
	let selectedAddress: string | null = null

	return {
		vendor: 'mnemonic',
		supportsSighashSigning: true,

		async connect(): Promise<WalletAccountInfo> {
			const addressesResult = await tauriCall<HwAddressEntry[]>('list_mnemonic_addresses', {
				mnemonic: opts.mnemonic,
				passphrase: opts.passphrase,
				count: 20,
			})
			if (!addressesResult.ok) {
				throw new Error(addressesResult.error)
			}

			const firstEntry = addressesResult.data[0]
			if (!firstEntry) {
				throw new Error('No derived addresses available for this mnemonic.')
			}

			derivationPath = firstEntry.derivationPath
			publicKeyHex = firstEntry.publicKeyHex
			selectedAddress = firstEntry.address

			return {
				deviceLabel: 'Mnemonic Wallet (BIP39)',
				derivationPath,
				addressSample: selectedAddress,
				xpubOrFingerprint: publicKeyHex,
				keyLabel: 'Public key',
			}
		},

		async disconnect(): Promise<void> {
			publicKeyHex = null
			selectedAddress = null
		},

		setDerivationPath(nextPath: string): void {
			derivationPath = nextPath
		},

		async signSighash(sighashHex: string): Promise<SignSighashResult> {
			if (!publicKeyHex) {
				throw new Error('Connect the mnemonic wallet first.')
			}
			const result = await tauriCall<{ publicKeyHex: string; signatureHex: string }>('sign_with_mnemonic_path', {
				mnemonic: opts.mnemonic,
				passphrase: opts.passphrase,
				derivationPath,
				sighashHex,
			})
			if (!result.ok) {
				throw new Error(result.error)
			}
			return {
				publicKeyHex: result.data.publicKeyHex,
				signatureHex: result.data.signatureHex,
				signatureFormat: 'raw-ecdsa',
			}
		},

		async listAddresses(count = 20): Promise<HwAddressEntry[]> {
			const result = await tauriCall<HwAddressEntry[]>('list_mnemonic_addresses', {
				mnemonic: opts.mnemonic,
				passphrase: opts.passphrase,
				count,
			})
			if (!result.ok) {
				throw new Error(result.error)
			}
			return result.data
		},
	}
}
