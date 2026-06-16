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
		case 'ElectrumUnreachable':
			return {
				title: 'Electrum indexer unreachable',
				body: `Cannot reach the Electrum indexer used for wallet sync. Check Settings > Node > Electrum URL.${err.message ? ` (${err.message})` : ''}`,
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
		// Phase 5 — fee bump (PRD §4.3.3)
		case 'SignerNotAllowedOnNetwork':
			return {
				title: 'Signer not allowed on this network',
				body: 'The session signer cannot sign on the active network. Connect a hardware wallet to sign.',
				severity: 'warning',
			}
		case 'InvalidTxid':
			return {
				title: 'Invalid transaction id',
				body: err.message,
				severity: 'warning',
			}
		case 'TxNotFound':
			return {
				title: 'Transaction not found',
				body: 'This transaction is not in the wallet. Refresh and try again.',
				severity: 'warning',
			}
		case 'TxAlreadyConfirmed':
			return {
				title: 'Already confirmed',
				body: 'This transaction has confirmed — its fee can no longer be bumped.',
				severity: 'info',
			}
		case 'TxNotReplaceable':
			return {
				title: 'Not replaceable',
				body: 'This transaction does not signal RBF, so it cannot be replaced with a higher-fee version.',
				severity: 'info',
			}
		case 'CpfpOutputUnavailable':
			return {
				title: 'Cannot accelerate this commit',
				body: `The reveal change output needed for the acceleration is not spendable. ${err.message}`,
				severity: 'warning',
			}
		case 'FeeTooLow':
		case 'FeeRateTooLow':
			return {
				title: 'Fee too low to bump',
				body: `The new rate must exceed what the transaction already pays. ${err.message}`,
				severity: 'warning',
			}
		case 'InvalidFeeRate':
			return {
				title: 'Invalid fee rate',
				body: err.message,
				severity: 'warning',
			}
		case 'InsufficientFunds':
			return {
				title: 'Insufficient funds',
				body: 'The wallet balance cannot cover this transaction and its fee.',
				severity: 'warning',
			}
		case 'BuildFailed':
			return {
				title: 'Could not build replacement',
				body: err.message,
				severity: 'warning',
			}
		case 'SignFailed':
			return {
				title: 'Signing failed',
				body: err.message,
				severity: 'warning',
			}
		case 'BroadcastFailed':
			return {
				title: 'Broadcast failed',
				body: `The transaction was signed but could not be broadcast. ${err.message}`,
				severity: 'warning',
			}
		// Phase 6 — Send (PRD §4.3.5)
		case 'InvalidAddress':
			return {
				title: 'Invalid destination',
				body: 'Destination must be a bitcoin address.',
				severity: 'warning',
			}
		case 'WrongNetwork':
			return {
				title: 'Wrong network',
				body: err.message,
				severity: 'warning',
			}
		case 'InvalidAmount':
			return {
				title: 'Invalid amount',
				body: 'Send amount must be greater than zero.',
				severity: 'warning',
			}
		case 'AmountBelowDust':
			return {
				title: 'Amount too small',
				body: 'Amount is below the dust limit.',
				severity: 'warning',
			}
	}
}
