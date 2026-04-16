import { tauriCall } from '@/api/tauri-bridge'
import type {
	SignSighashResult,
	SignTestPayloadResult,
	WalletAccountInfo,
	WalletAdapter,
} from './types'

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

		async signTestPayload(payloadUtf8: string): Promise<SignTestPayloadResult> {
			const encoded: Uint8Array<ArrayBuffer> = new Uint8Array(new TextEncoder().encode(payloadUtf8))
			const hashBuffer = await crypto.subtle.digest('SHA-256', encoded)
			const sighashHex = Array.from(new Uint8Array(hashBuffer))
				.map((b) => b.toString(16).padStart(2, '0'))
				.join('')
			const result = await tauriCall<SignatureResult>('sign_with_trezor', { sighashHex, derivationPath })
			if (!result.ok) throw new Error(result.error)
			return {
				signatureHex: result.data.signatureHex,
				note: 'Trezor PSBT sign_tx: ECDSA over BIP143 P2WPKH sighash; payload digest is OP_RETURN–bound (not BIP-137).',
			}
		},

		async signSighash(sighashHex: string): Promise<SignSighashResult> {
			if (!publicKeyHex) throw new Error('Connect the Trezor first.')
			const result = await tauriCall<SignatureResult>('sign_with_trezor', { sighashHex, derivationPath })
			if (!result.ok) throw new Error(result.error)
			return {
				publicKeyHex: result.data.publicKeyHex,
				signatureHex: result.data.signatureHex,
				signatureFormat: 'p2wpkh-tx-binding',
			}
		},
	}
}
