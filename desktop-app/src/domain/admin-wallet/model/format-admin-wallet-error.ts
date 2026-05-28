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
				body: 'Admin Wallet is not enabled for this environment. Set COMMIT_FUNDING=admin_wallet, BITCOIN_NETWORK=regtest, and ALLOW_DEV_MNEMONIC_SIGNING=1 to enable.',
				severity: 'info',
			}
		case 'RpcUnreachable':
			return {
				title: 'Bitcoin node unreachable',
				body: 'Cannot reach Bitcoin node. Check BITCOIN_RPC_URL.',
				severity: 'warning',
			}
		case 'RpcAuthFailed':
			return {
				title: 'RPC auth failed',
				body: 'Bitcoin RPC authentication failed. Check BITCOIN_RPC_USER and BITCOIN_RPC_PASS.',
				severity: 'warning',
			}
		case 'DescriptorParseError':
			return {
				title: 'Invalid descriptor',
				body: 'Admin Wallet descriptor invalid — check ADMIN_WALLET_REGTEST_MNEMONIC.',
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
