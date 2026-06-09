import type { AdminWalletError } from '@/api/admin-wallet'

export function parseAdminWalletError(raw: string): AdminWalletError {
	try {
		return JSON.parse(raw) as AdminWalletError
	} catch {
		return { type: 'RpcUnreachable', message: raw }
	}
}
