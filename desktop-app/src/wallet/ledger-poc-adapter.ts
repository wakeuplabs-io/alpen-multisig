import { tauriCall } from '@/api/tauri-bridge'
import type { SignSighashResult, SignTestPayloadResult, WalletAccountInfo, WalletAdapter } from './types'

/** BIP-84 first receive address — must match the Rust DEFAULT_PATH. */
const DEFAULT_DERIVATION_PATH = "84'/0'/0'/0/0"

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

		async signTestPayload(payloadUtf8: string): Promise<SignTestPayloadResult> {
			const encoded: Uint8Array<ArrayBuffer> = new Uint8Array(new TextEncoder().encode(payloadUtf8))
			const hashBuffer = await crypto.subtle.digest('SHA-256', encoded)
			const sighashHex = Array.from(new Uint8Array(hashBuffer))
				.map((b) => b.toString(16).padStart(2, '0'))
				.join('')
			const result = await tauriCall<SignatureResult>('sign_with_ledger', { sighashHex, derivationPath })
			if (!result.ok) throw new Error(result.error)
			return {
				signatureHex: result.data.signatureHex,
				note: 'Bitcoin Signed Message (BIP-137) via Ledger — Rust HID. Confirm on device.',
			}
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
