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
/** A resolved message, carrying the exact inputs it was resolved for. */
export type Resolved = DeviceSigningMessage & { seqno: number; actionHex: string }

export function useDeviceSigningMessage(seqno: number | null, actionHex: string | null): DeviceSigningMessage {
	const [resolved, setResolved] = useState<Resolved | null>(null)

	useEffect(() => {
		if (seqno === null || !actionHex) {
			setResolved(null)
			return
		}
		let cancelled = false
		void (async () => {
			const result = await renderSigningMessage(seqno, actionHex)
			if (cancelled || !result.ok) return
			const hash = await sha256Hex(result.data)
			if (cancelled) return
			setResolved({ seqno, actionHex, message: result.data, messageHash: hash })
		})()
		return () => {
			cancelled = true
		}
	}, [seqno, actionHex])

	return messageForInputs(resolved, seqno, actionHex)
}

/**
 * The pairing guard, extracted so the property that matters can be asserted directly: a message
 * resolved for one action is never returned for another — not even for the frame between a new
 * render and the effect that would have cleared it. A signer comparing a stale message against
 * their device would be verifying the wrong action, and callers must not have to guard against it.
 */
export function messageForInputs(
	resolved: Resolved | null,
	seqno: number | null,
	actionHex: string | null,
): DeviceSigningMessage {
	if (resolved === null || resolved.seqno !== seqno || resolved.actionHex !== actionHex) {
		return { message: null, messageHash: null }
	}
	return { message: resolved.message, messageHash: resolved.messageHash }
}
