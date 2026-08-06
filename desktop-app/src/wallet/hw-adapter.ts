import { tauriCall } from '@/api/tauri-bridge'
import type {
	SignSighashResult,
	SigningContext,
	WalletAccountInfo,
	WalletAdapter,
	WalletKind,
	WalletVendor,
} from './types'

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

/**
 * No passphrase crosses this boundary. A Trezor passphrase is entered on the device
 * keypad, so there is nothing here to hold or forward (#448). `kind` selects *which* wallet
 * behind the seed to open, which is a choice, not a secret: 'standard' needs no passphrase
 * at all and 'hidden' defers entry to the device.
 */
export function createHwAdapter(vendor: WalletVendor): WalletAdapter {
	let publicKeyHex: string | null = null
	let currentDerivationPath = ''

	return {
		vendor,
		supportsSighashSigning: true,

		async connect(kind: WalletKind = 'standard'): Promise<WalletAccountInfo> {
			const result = await tauriCall<HwWalletInfo>('hw_wallet_connect', {
				vendor,
				derivationPath: null,
				walletKind: kind,
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
			// The device session is not ended here on purpose. Callers fire disconnect without
			// awaiting it, so ending it from this side could land after the next connect and
			// wipe that session instead. `connect` starts a clean session itself.
		},

		async signSighash(sighashHex: string, context?: SigningContext): Promise<SignSighashResult> {
			if (!publicKeyHex) throw new Error(`Connect the ${vendor} first.`)
			if (!context) {
				const result = await tauriCall<SignatureResult>('hw_wallet_sign_challenge', {
					vendor,
					challengeMessage: sighashHex,
					derivationPath: currentDerivationPath,
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
			})
			if (!result.ok) throw new Error(result.error)
			return {
				publicKeyHex: result.data.publicKeyHex,
				signatureHex: result.data.signatureHex,
				signatureFormat: 'bitcoin-message',
			}
		},

		async getAccountXpub(): Promise<string> {
			const result = await tauriCall<string>('hw_wallet_get_xpub', { vendor })
			if (!result.ok) throw new Error(result.error)
			return result.data
		},

		async getMasterFingerprint(): Promise<number> {
			const result = await tauriCall<number>('hw_wallet_get_fingerprint', { vendor })
			if (!result.ok) throw new Error(result.error)
			if (result.data === 0) {
				throw new Error(`${vendor} returned an invalid master fingerprint`)
			}
			return result.data
		},
	}
}
