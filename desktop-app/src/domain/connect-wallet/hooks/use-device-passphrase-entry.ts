import { useCallback, useEffect, useState } from 'react'
import { z } from 'zod'
import { tauriCall } from '@/api/tauri-bridge'
import type { WalletVendor } from '@/wallet/types'

const hwDeviceCapabilitiesSchema = z.object({
	model: z.string().nullish(),
	supportsPassphraseEntry: z.boolean(),
})

export type DevicePassphraseEntry = {
	/**
	 * `false` only when a device said so. `null` means unknown — no device answered yet —
	 * and callers should assume the affordance applies rather than hide it, since every
	 * supported model has a keypad.
	 */
	supported: boolean | null
	/** The device's own model string ('T', 'Safe 3', 'Safe 5'), when it reported one. */
	model: string | null
	/** Re-reads the device, for when one is plugged in after the screen was opened. */
	recheck: () => void
}

/**
 * Asks the connected device whether it can take a passphrase on its own keypad (#448).
 *
 * The connect screen needs this *before* connecting, to decide whether to offer on-device
 * entry: a device without a keypad for it rejects the request outright. The read runs on
 * `Initialize` alone, so it derives no key and prompts for nothing on the device.
 *
 * A failure is not an error state: the expected way to reach this screen is with the Trezor
 * not yet plugged in, which is why `recheck` exists — the connect screen calls it when the
 * signer picks the Trezor method, by which point the device is usually there.
 */
export function useDevicePassphraseEntry(vendor: WalletVendor): DevicePassphraseEntry {
	const [entry, setEntry] = useState<{ supported: boolean | null; model: string | null }>({
		supported: null,
		model: null,
	})
	const [attempt, setAttempt] = useState(0)

	const recheck = useCallback(() => setAttempt((n) => n + 1), [])

	useEffect(() => {
		if (vendor !== 'trezor') {
			setEntry({ supported: null, model: null })
			return
		}
		let active = true
		void (async () => {
			const result = await tauriCall('hw_wallet_capabilities', { vendor }, hwDeviceCapabilitiesSchema)
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
	}, [vendor, attempt])

	return { ...entry, recheck }
}
