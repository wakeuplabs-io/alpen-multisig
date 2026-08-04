import { useEffect, useState } from 'react'
import { tauriCall } from '@/api/tauri-bridge'
import type { WalletVendor } from '@/wallet/types'

type HwDeviceCapabilities = {
	model?: string | null
	supportsPassphraseEntry: boolean
}

export type DevicePassphraseEntry = {
	/** `null` until the device has answered, or when no device could be read. */
	supported: boolean | null
	/** The device's own model string ('T', 'Safe 3', 'Safe 5', '1'), for the hint. */
	model: string | null
}

/**
 * Asks the connected device whether it can take a passphrase on its own keypad (#448).
 *
 * The connect screen needs this *before* connecting, to decide whether to offer on-device
 * entry at all: a Trezor One has no keypad for it and rejects the request outright. The
 * read runs on `Initialize` alone, so it derives no key and prompts for nothing on the
 * device.
 *
 * A failure is not surfaced as an error: with no device plugged in there is simply nothing
 * to offer yet, and the connect screen already reports connection failures on its own.
 */
export function useDevicePassphraseEntry(vendor: WalletVendor): DevicePassphraseEntry {
	const [entry, setEntry] = useState<DevicePassphraseEntry>({ supported: null, model: null })

	useEffect(() => {
		if (vendor !== 'trezor') {
			setEntry({ supported: null, model: null })
			return
		}
		let active = true
		void (async () => {
			const result = await tauriCall<HwDeviceCapabilities>('hw_wallet_capabilities', { vendor })
			if (!active) return
			if (!result.ok) {
				setEntry({ supported: null, model: null })
				return
			}
			setEntry({ supported: result.data.supportsPassphraseEntry, model: result.data.model ?? null })
		})()
		return () => {
			active = false
		}
	}, [vendor])

	return entry
}
