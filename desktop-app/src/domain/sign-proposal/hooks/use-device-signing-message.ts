import { useEffect, useState } from 'react'
import { renderSigningMessage } from '@/api/signing'
import { sha256Hex } from '@/lib/sha256'

type DeviceSigningMessage = {
	/** Canonical SPS-65 message text (what Trezor shows). */
	message: string | null
	/** `sha256(message)` (what Ledger shows as "Message hash"). */
	messageHash: string | null
}

/**
 * Resolves the on-device verification values for an SPS-65 action: the canonical message
 * text (Trezor) and its SHA-256 (Ledger "Message hash"). Lets the signing UI present what
 * the device actually displays instead of the BIP-137 sighash, which never appears on-device.
 */
export function useDeviceSigningMessage(seqno: number | null, actionHex: string | null): DeviceSigningMessage {
	const [message, setMessage] = useState<string | null>(null)
	const [messageHash, setMessageHash] = useState<string | null>(null)

	useEffect(() => {
		if (seqno === null || !actionHex) {
			setMessage(null)
			setMessageHash(null)
			return
		}
		let cancelled = false
		void (async () => {
			const result = await renderSigningMessage(seqno, actionHex)
			if (cancelled || !result.ok) return
			const hash = await sha256Hex(result.data)
			if (cancelled) return
			setMessage(result.data)
			setMessageHash(hash)
		})()
		return () => {
			cancelled = true
		}
	}, [seqno, actionHex])

	return { message, messageHash }
}
