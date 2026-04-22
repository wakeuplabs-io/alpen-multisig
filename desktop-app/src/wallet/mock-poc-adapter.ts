import { tauriCall } from '@/api/tauri-bridge'
import type { WalletAccountInfo, WalletAdapter } from './types'

function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('')
}

export function createMockPocAdapter(): WalletAdapter {
	let connected = false
	let secretKeyHex: string | null = null
	let cachedPublicKeyHex: string | null = null

	return {
		vendor: 'mock',
		supportsSighashSigning: true,

		async connect(): Promise<WalletAccountInfo> {
			connected = true
			const bytes = crypto.getRandomValues(new Uint8Array(32))
			secretKeyHex = bytesToHex(bytes)

			const warmup = await tauriCall<{ public_key_hex: string; signature_hex: string }>('sign_action_sighash', {
				secretKeyHex,
				sighashHex: '00'.repeat(32),
			})
			if (!warmup.ok) {
				secretKeyHex = null
				throw new Error(`Mock wallet init failed: ${warmup.error}`)
			}
			cachedPublicKeyHex = warmup.data.public_key_hex
			return {
				deviceLabel: 'Mock Wallet',
				derivationPath: "m/86'/0'/73'/0/0",
				addressSample: 'bc1p0q0wnl9lhp92uh65589uu0sdf62j2ea2n8203eddumps3sjr00hqc4shtx',
				xpubOrFingerprint: `${cachedPublicKeyHex.slice(0, 16)}…`,
			}
		},

		async disconnect(): Promise<void> {
			connected = false
			secretKeyHex = null
			cachedPublicKeyHex = null
		},

		async signSighash(sighashHex: string) {
			if (!connected || !secretKeyHex) {
				throw new Error('Connect the Mock wallet first.')
			}
			const result = await tauriCall<{ public_key_hex: string; signature_hex: string }>('sign_action_sighash', {
				secretKeyHex,
				sighashHex,
			})
			if (!result.ok) {
				throw new Error(result.error)
			}
			return {
				publicKeyHex: result.data.public_key_hex,
				signatureHex: result.data.signature_hex,
				signatureFormat: 'raw-ecdsa' as const,
			}
		},
	}
}
