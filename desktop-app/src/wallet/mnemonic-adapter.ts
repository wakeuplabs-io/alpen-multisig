import { tauriCall } from '@/api/tauri-bridge'
import type { HwAddressEntry, SignSighashResult, SigningContext, WalletAccountInfo, WalletAdapter } from './types'

type SignatureResult = {
	publicKeyHex: string
	signatureHex: string
}

export type MnemonicAdapterOptions = {
	mnemonic: string
	passphrase?: string
	derivationPath?: string
}

export type MnemonicAdapter = WalletAdapter & {
	getMnemonic(): string
}

export function createMnemonicAdapter(opts: MnemonicAdapterOptions): MnemonicAdapter {
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
				count: 1,
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
				publicKeyHex: publicKeyHex ?? undefined,
			}
		},

		async disconnect(): Promise<void> {
			publicKeyHex = null
			selectedAddress = null
		},

		async signSighash(sighashHex: string, context?: SigningContext): Promise<SignSighashResult> {
			if (!publicKeyHex) {
				throw new Error('Connect the mnemonic wallet first.')
			}
			if (!context) {
				const result = await tauriCall<SignatureResult>('sign_message_with_mnemonic_path', {
					mnemonic: opts.mnemonic,
					passphrase: opts.passphrase,
					derivationPath,
					message: sighashHex,
				})
				if (!result.ok) throw new Error(result.error)
				return {
					publicKeyHex: result.data.publicKeyHex,
					signatureHex: result.data.signatureHex,
					signatureFormat: 'bitcoin-message',
				}
			}
			const result = await tauriCall<SignatureResult>('sign_with_mnemonic_path', {
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

		getMnemonic(): string {
			return opts.mnemonic
		},
	}
}
