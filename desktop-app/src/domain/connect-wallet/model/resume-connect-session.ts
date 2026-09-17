import type { HwAddressEntry, WalletAccountInfo } from '@/wallet/types'
import type { HwWalletPhase } from '@/domain/connect-wallet/model/hw-wallet-connect.types'

export type AuthorityConnectStep = 'select-authority' | 'authenticate-session'

export type ManualReturnTo = {
	path: string
	authorityStep: AuthorityConnectStep
}

/** Seed the HW connect wizard from an already-connected session wallet (no re-connect). */
export function selectedStateFromWallet(wallet: WalletAccountInfo): {
	phase: Extract<HwWalletPhase, 'selected'>
	account: WalletAccountInfo
	selectedEntry: HwAddressEntry
} {
	return {
		phase: 'selected',
		account: wallet,
		selectedEntry: {
			index: 0,
			derivationPath: wallet.derivationPath,
			address: wallet.addressSample ?? '',
			publicKeyHex: wallet.publicKeyHex ?? '',
		},
	}
}

export function readAuthorityStepFromLocationState(state: unknown): AuthorityConnectStep {
	const step = (state as { authorityStep?: unknown } | null)?.authorityStep
	if (step === 'authenticate-session' || step === 'select-authority') {
		return step
	}
	return 'select-authority'
}

/** Resolve Manual execution Back: explicit returnTo wins over history. */
export function resolveManualBackNavigation(
	state: unknown,
): { kind: 'returnTo'; path: string; authorityStep: AuthorityConnectStep } | { kind: 'history' } {
	const returnTo = (state as { returnTo?: ManualReturnTo } | null)?.returnTo
	if (
		returnTo &&
		typeof returnTo.path === 'string' &&
		returnTo.path.length > 0 &&
		(returnTo.authorityStep === 'authenticate-session' || returnTo.authorityStep === 'select-authority')
	) {
		return { kind: 'returnTo', path: returnTo.path, authorityStep: returnTo.authorityStep }
	}
	return { kind: 'history' }
}
