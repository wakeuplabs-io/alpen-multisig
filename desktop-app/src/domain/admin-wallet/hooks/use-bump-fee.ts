import { useCallback, useState } from 'react'
import { bumpAdminWalletFee } from '@/api/admin-wallet'
import type { AdminWalletError, BumpFeeResultDto } from '@/api/admin-wallet'
import { parseAdminWalletError } from './parse-admin-wallet-error'

export type BumpFeeState =
	| { status: 'idle' }
	| { status: 'submitting' }
	| { status: 'success'; result: BumpFeeResultDto }
	| { status: 'error'; error: AdminWalletError }

type UseBumpFeeReturn = {
	state: BumpFeeState
	/** Submits the replacement; resolves true on success. */
	bump(txid: string, feeRateSatPerKvb: number): Promise<boolean>
	reset(): void
}

/** Fee-bump submission state machine: idle → submitting → success | error (Phase 5, PRD §4.3.3). */
export function useBumpFee(): UseBumpFeeReturn {
	const [state, setState] = useState<BumpFeeState>({ status: 'idle' })

	const bump = useCallback(async (txid: string, feeRateSatPerKvb: number) => {
		setState({ status: 'submitting' })
		const result = await bumpAdminWalletFee({ txid, feeRateSatPerKvb })
		if (result.ok) {
			setState({ status: 'success', result: result.data })
			return true
		}
		setState({ status: 'error', error: parseAdminWalletError(result.error) })
		return false
	}, [])

	const reset = useCallback(() => setState({ status: 'idle' }), [])

	return { state, bump, reset }
}
