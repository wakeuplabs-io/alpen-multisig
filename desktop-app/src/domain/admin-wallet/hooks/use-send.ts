import { useCallback, useState } from 'react'
import { sendFromAdminWallet } from '@/api/admin-wallet'
import type { AdminWalletError, SendInput, SendResultDto } from '@/api/admin-wallet'
import { parseAdminWalletError } from './parse-admin-wallet-error'

export type SendState =
	| { status: 'idle' }
	| { status: 'submitting' }
	| { status: 'success'; result: SendResultDto }
	| { status: 'error'; error: AdminWalletError }

type UseSendReturn = {
	state: SendState
	/** Submits the send; resolves true on success. */
	send(input: SendInput): Promise<boolean>
	reset(): void
}

/**
 * Send submission state machine: idle → submitting → success | error
 * (Phase 6, PRD §4.3.5). On error the caller keeps the form state untouched —
 * the §4.3.5.5.1 reject/retry contract; this hook owns no field state.
 */
export function useSend(): UseSendReturn {
	const [state, setState] = useState<SendState>({ status: 'idle' })

	const send = useCallback(async (input: SendInput) => {
		setState({ status: 'submitting' })
		const result = await sendFromAdminWallet(input)
		if (result.ok) {
			setState({ status: 'success', result: result.data })
			return true
		}
		setState({ status: 'error', error: parseAdminWalletError(result.error) })
		return false
	}, [])

	const reset = useCallback(() => setState({ status: 'idle' }), [])

	return { state, send, reset }
}
