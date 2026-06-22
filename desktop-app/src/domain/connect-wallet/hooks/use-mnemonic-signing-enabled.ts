import { useEffect, useState } from 'react'
import { tauriCall } from '@/api/tauri-bridge'

// Mirrors the backend gate (dev_mnemonic_signing_ipc_enabled): true for debug builds or
// release with ALLOW_DEV_MNEMONIC_SIGNING=1. Defaults to false until resolved so the
// Mnemonic option never flashes in production builds where signing is disabled.
export function useMnemonicSigningEnabled(): boolean {
	const [enabled, setEnabled] = useState(false)
	useEffect(() => {
		let active = true
		void tauriCall<boolean>('dev_mnemonic_signing_enabled').then((result) => {
			if (active && result.ok) {
				setEnabled(result.data)
			}
		})
		return () => {
			active = false
		}
	}, [])
	return enabled
}
