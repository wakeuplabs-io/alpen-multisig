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
				body: 'Admin Wallet is not available. Connect your wallet to bind it to your session.',
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
				body: 'Admin Wallet descriptor is invalid. Reconnect your wallet and try again.',
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
		case 'ReadOnly':
			return {
				title: 'Watch-only wallet',
				body: 'This wallet is in watch-only mode. Signing operations are not available.',
				severity: 'info',
			}
		case 'InvalidMnemonic':
			return {
				title: 'Invalid mnemonic',
				body: `Admin Wallet could not be derived from the login mnemonic.${err.message ? ` (${err.message})` : ''}`,
				severity: 'fatal',
			}
		case 'Descriptor':
		case 'WalletCreation':
			return {
				title: 'Admin Wallet could not be built',
				body: `Failed to construct the Admin Wallet from the account key.${err.message ? ` (${err.message})` : ''}`,
				severity: 'fatal',
			}
	}
}
