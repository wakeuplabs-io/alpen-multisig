import { useCallback, useState } from 'react'
import { verifyAddressOnDevice } from '@/api/admin-wallet'
import type { HwDeviceType, VerifyScriptType } from '@/api/admin-wallet'

export type VerifyOnDeviceState =
	| { status: 'idle' }
	| { status: 'verifying' }
	| { status: 'verified' }
	| { status: 'failed'; message: string }

type Params = {
	deviceType: HwDeviceType
	network: string
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
 * on-screen. A device rejection, mismatch surfacing, or timeout lands on `failed`.
 * Presentation only — never touches signing material.
 */
export function useVerifyOnDevice({ deviceType, network }: Params): UseVerifyOnDeviceReturn {
	const [state, setState] = useState<VerifyOnDeviceState>({ status: 'idle' })

	const verify = useCallback(
		async (derivationPath: string, scriptType: VerifyScriptType) => {
			setState({ status: 'verifying' })
			const result = await verifyAddressOnDevice({ derivationPath, deviceType, scriptType, network })
			if (result.ok) {
				setState({ status: 'verified' })
				return
			}
			setState({ status: 'failed', message: result.error })
		},
		[deviceType, network],
	)

	const reset = useCallback(() => setState({ status: 'idle' }), [])

	return { state, verify, reset }
}
