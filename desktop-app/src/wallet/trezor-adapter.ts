import { tauriCall } from '@/api/tauri-bridge'
import type { SignSighashResult, SigningContext, WalletAccountInfo, WalletAdapter } from './types'

/** BIP-84 Admin ID path (P2WPKH) for message signing — non-Payout-Admin multisigs. */
const ADMIN_ID_PATH = "m/84'/0'/73'/0/0"

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

export function createTrezorAdapter(passphrase?: string): WalletAdapter {
	let publicKeyHex: string | null = null
	let currentDerivationPath = ADMIN_ID_PATH
	const pp = passphrase ?? ''

	return {
		vendor: 'trezor',
		supportsSighashSigning: true,
		passphrase: pp || undefined,

		async connect(): Promise<WalletAccountInfo> {
			const result = await tauriCall<HwWalletInfo>('get_trezor_info', {
				derivationPath: ADMIN_ID_PATH,
				passphrase: pp,
			})
			if (!result.ok) throw new Error(result.error)
			const info = result.data
			publicKeyHex = info.publicKeyHex ?? info.xpubOrFingerprint ?? null
			currentDerivationPath = ADMIN_ID_PATH
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
			currentDerivationPath = ADMIN_ID_PATH
		},

		async signSighash(sighashHex: string, context?: SigningContext): Promise<SignSighashResult> {
			if (!publicKeyHex) throw new Error('Connect the Trezor first.')
			if (!context) {
				const result = await tauriCall<SignatureResult>('sign_challenge_with_trezor', {
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
			const result = await tauriCall<SignatureResult>('sign_with_trezor', {
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
			const result = await tauriCall<string>('get_trezor_admin_wallet_xpub', { passphrase: pp })
			if (!result.ok) throw new Error(result.error)
			return result.data
		},

		async getMasterFingerprint(): Promise<number> {
			const result = await tauriCall<number>('get_trezor_master_fingerprint', { passphrase: pp })
			if (!result.ok) {
				throw new Error(result.error)
			}
			if (result.data === 0) {
				throw new Error('Trezor returned an invalid master fingerprint')
			}
			return result.data
		},
	}
}
