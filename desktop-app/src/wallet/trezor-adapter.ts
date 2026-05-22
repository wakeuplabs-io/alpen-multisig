import { tauriCall } from '@/api/tauri-bridge'
import type { HwAddressEntry, SignSighashResult, SigningContext, WalletAccountInfo, WalletAdapter } from './types'

/** BIP-84 Admin ID path (P2WPKH) for message signing — non-Payout-Admin multisigs. */
const ADMIN_ID_PATH = "m/84'/0'/73'/0/0"

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

export function createTrezorAdapter(): WalletAdapter {
	let publicKeyHex: string | null = null
	let currentDerivationPath = ADMIN_ID_PATH

	return {
		vendor: 'trezor',
		supportsSighashSigning: true,

		async connect(): Promise<WalletAccountInfo> {
			const result = await tauriCall<HwWalletInfo>('get_trezor_info', { derivationPath: ADMIN_ID_PATH })
			if (!result.ok) throw new Error(result.error)
			const info = result.data
			publicKeyHex = info.xpubOrFingerprint ?? null
			currentDerivationPath = ADMIN_ID_PATH
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
			currentDerivationPath = ADMIN_ID_PATH
		},

		setDerivationPath(nextPath: string): void {
			currentDerivationPath = nextPath
		},

		async signSighash(sighashHex: string, context?: SigningContext): Promise<SignSighashResult> {
			if (!publicKeyHex) throw new Error('Connect the Trezor first.')
			if (!context) {
				const result = await tauriCall<SignatureResult>('sign_challenge_with_trezor', {
					challengeHex: sighashHex,
					derivationPath: currentDerivationPath,
				})
				if (!result.ok) throw new Error(result.error)
				return {
					publicKeyHex: result.data.publicKeyHex,
					signatureHex: result.data.signatureHex,
					signatureFormat: 'bitcoin-message',
				}
			}
			const result = await tauriCall<SignatureResult>('sign_with_trezor', {
				seqno: context.seqno,
				actionHex: context.actionHex,
				derivationPath: currentDerivationPath,
			})
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
