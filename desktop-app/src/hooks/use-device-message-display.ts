import { useEffect, useState } from 'react'
import { sha256Hex } from '@/lib/sha256'
import { deviceSigningDisplay, type DeviceSigningDisplay } from '@/lib/device-signing-display'
import type { WalletVendor } from '@/wallet/types'

/**
 * Resolves what the connected device displays for a message it signs: Ledger renders either
 * the message text or its SHA-256 ("Message hash") depending on the model and Bitcoin app
 * version, so both are resolved; Trezor shows the message text; software signers show
 * nothing. Use when the message string is already known (e.g. the session-auth challenge).
 */
export function useDeviceMessageDisplay(vendor: WalletVendor, message: string | null): DeviceSigningDisplay {
	// The message and its hash are held together, never separately: the message arrives
	// synchronously but its SHA-256 does not, so pairing the incoming message with the previous
	// hash would show a Ledger signer two values belonging to different messages (#402).
	const [resolved, setResolved] = useState<{ message: string; hash: string } | null>(null)

	useEffect(() => {
		setResolved(null)
		if (!message) {
			return
		}
		let cancelled = false
		void sha256Hex(message).then((hash) => {
			if (!cancelled) setResolved({ message, hash })
		})
		return () => {
			cancelled = true
		}
	}, [message])

	return deviceSigningDisplay(vendor, { message: resolved?.message ?? null, messageHash: resolved?.hash ?? null })
}
