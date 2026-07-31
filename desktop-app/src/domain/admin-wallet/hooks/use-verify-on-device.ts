import { useCallback, useState } from 'react'
import { verifyAddressOnDevice } from '@/api/admin-wallet'
import type { HwDeviceType, VerifyScriptType } from '@/api/admin-wallet'
import { matchesDeviceAddress } from '@/lib/admin-id'

export type VerifyOnDeviceState =
	| { status: 'idle' }
	| { status: 'verifying' }
	/** Device confirmed and showed exactly the address the app expected. */
	| { status: 'verified'; address: string }
	/** Device confirmed but showed a different address — a security alarm, not a transport error. */
	| { status: 'mismatch'; address: string }
	| { status: 'failed'; message: string }

type Params = {
	deviceType: HwDeviceType
	network: string
	/** When set, the address the device shows is compared against this value. */
	expectedAddress?: string
}

type UseVerifyOnDeviceReturn = {
	state: VerifyOnDeviceState
	/** Prompts the connected device to display the address at `derivationPath`. */
	verify(derivationPath: string, scriptType: VerifyScriptType): Promise<void>
	reset(): void
}

/**
 * Verify-on-device state machine (Phase 8, PRD §4.2 / §4.3.4.2): prompts the
 * connected hardware device to render an address so the signer can compare it
 * on-screen. The device returns the string it rendered; when `expectedAddress` is
 * provided the two are compared, so a device holding a different key lands on
 * `mismatch` instead of silently reading as success. A device rejection or timeout
 * lands on `failed`. Presentation only — never touches signing material.
 */
export function useVerifyOnDevice({ deviceType, network, expectedAddress }: Params): UseVerifyOnDeviceReturn {
	const [state, setState] = useState<VerifyOnDeviceState>({ status: 'idle' })

	const verify = useCallback(
		async (derivationPath: string, scriptType: VerifyScriptType) => {
			setState({ status: 'verifying' })
			const result = await verifyAddressOnDevice({ derivationPath, deviceType, scriptType, network })
			if (!result.ok) {
				setState({ status: 'failed', message: result.error })
				return
			}

			const shown = result.data
			if (expectedAddress && !matchesDeviceAddress(expectedAddress, shown)) {
				setState({ status: 'mismatch', address: shown })
				return
			}
			setState({ status: 'verified', address: shown })
		},
		[deviceType, network, expectedAddress],
	)

	const reset = useCallback(() => setState({ status: 'idle' }), [])

	return { state, verify, reset }
}
