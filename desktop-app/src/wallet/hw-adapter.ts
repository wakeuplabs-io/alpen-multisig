import { tauriCall } from '@/api/tauri-bridge'
import type { SignSighashResult, SigningContext, WalletAccountInfo, WalletAdapter, WalletVendor } from './types'

type HwWalletInfo = {
	deviceLabel: string
	derivationPath: string
	addressSample?: string
	publicKeyHex?: string
	xpubOrFingerprint?: string
	keyLabel?: string
}

type SignatureResult = {
	publicKeyHex: string
	signatureHex: string
}

export function createHwAdapter(vendor: WalletVendor, passphrase?: string): WalletAdapter {
	let publicKeyHex: string | null = null
	let currentDerivationPath = ''
	const pp = passphrase ?? ''

	return {
		vendor,
		supportsSighashSigning: true,
		passphrase: pp || undefined,

		async connect(): Promise<WalletAccountInfo> {
			const result = await tauriCall<HwWalletInfo>('hw_wallet_connect', {
				vendor,
				derivationPath: null,
				passphrase: pp,
			})
			if (!result.ok) throw new Error(result.error)
			const info = result.data
			publicKeyHex = info.publicKeyHex ?? info.xpubOrFingerprint ?? null
			currentDerivationPath = info.derivationPath
			return {
				deviceLabel: info.deviceLabel,
				derivationPath: info.derivationPath,
				addressSample: info.addressSample,
				publicKeyHex: publicKeyHex ?? undefined,
				xpubOrFingerprint: info.xpubOrFingerprint,
				keyLabel: info.keyLabel,
			}
		},

		async disconnect(): Promise<void> {
			publicKeyHex = null
			currentDerivationPath = ''
		},

		async signSighash(sighashHex: string, context?: SigningContext): Promise<SignSighashResult> {
			if (!publicKeyHex) throw new Error(`Connect the ${vendor} first.`)
			if (!context) {
				const result = await tauriCall<SignatureResult>('hw_wallet_sign_challenge', {
					vendor,
					challengeMessage: sighashHex,
					derivationPath: currentDerivationPath,
					passphrase: pp,
				})
				if (!result.ok) throw new Error(result.error)
				return {
					publicKeyHex: result.data.publicKeyHex,
					signatureHex: result.data.signatureHex,
					signatureFormat: 'bitcoin-message',
				}
			}
			const result = await tauriCall<SignatureResult>('hw_wallet_sign', {
				vendor,
				seqno: context.seqno,
				actionHex: context.actionHex,
				derivationPath: currentDerivationPath,
				passphrase: pp,
			})
			if (!result.ok) throw new Error(result.error)
			return {
				publicKeyHex: result.data.publicKeyHex,
				signatureHex: result.data.signatureHex,
				signatureFormat: 'bitcoin-message',
			}
		},

		async getAccountXpub(): Promise<string> {
			const result = await tauriCall<string>('hw_wallet_get_xpub', {
				vendor,
				passphrase: pp,
			})
			if (!result.ok) throw new Error(result.error)
			return result.data
		},

		async getMasterFingerprint(): Promise<number> {
			const result = await tauriCall<number>('hw_wallet_get_fingerprint', {
				vendor,
				passphrase: pp,
			})
			if (!result.ok) throw new Error(result.error)
			if (result.data === 0) {
				throw new Error(`${vendor} returned an invalid master fingerprint`)
			}
			return result.data
		},
	}
}
