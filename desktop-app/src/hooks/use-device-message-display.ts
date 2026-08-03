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
	const [messageHash, setMessageHash] = useState<string | null>(null)

	useEffect(() => {
		if (!message) {
			setMessageHash(null)
			return
		}
		let cancelled = false
		void sha256Hex(message).then((hash) => {
			if (!cancelled) setMessageHash(hash)
		})
		return () => {
			cancelled = true
		}
	}, [message])

	return deviceSigningDisplay(vendor, { message, messageHash })
}
