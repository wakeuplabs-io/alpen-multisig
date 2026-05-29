import type { AdminWalletError } from '@/api/admin-wallet'

type ErrorView = {
	title: string
	body: string
	severity: 'fatal' | 'warning' | 'info'
}

export function formatAdminWalletError(err: AdminWalletError): ErrorView {
	switch (err.type) {
		case 'Disabled':
			return {
				title: 'Admin Wallet not enabled',
				body: 'Admin Wallet is not available. Log in with Palabras (dev mnemonic) to bind the wallet to your session.',
				severity: 'info',
			}
		case 'RpcUnreachable':
			return {
				title: 'Bitcoin node unreachable',
				body: `Cannot reach Bitcoin node. Check BITCOIN_RPC_URL.${err.message ? ` (${err.message})` : ''}`,
				severity: 'warning',
			}
		case 'RpcAuthFailed':
			return {
				title: 'RPC auth failed',
				body: `Bitcoin RPC authentication failed. Check BITCOIN_RPC_USER and BITCOIN_RPC_PASS.${err.message ? ` (${err.message})` : ''}`,
				severity: 'warning',
			}
		case 'DescriptorParseError':
			return {
				title: 'Invalid descriptor',
				body: 'Admin Wallet descriptor invalid — check the mnemonic used at login (Palabras).',
				severity: 'fatal',
			}
		case 'SyncIncomplete':
			return {
				title: 'Sync incomplete',
				body: err.message,
				severity: 'warning',
			}
		case 'RegtestGuardViolation':
			return {
				title: 'Regtest guard violation',
				body: err.message,
				severity: 'info',
			}
	}
}
