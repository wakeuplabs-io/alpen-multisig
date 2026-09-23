import type { HwAddressEntry, WalletAccountInfo } from '@/wallet/types'
import type { HwWalletPhase } from '@/domain/connect-wallet/model/hw-wallet-connect.types'

export type AuthorityConnectStep = 'select-authority' | 'authenticate-session'

export type ManualReturnTo = {
	path: string
	authorityStep: AuthorityConnectStep
}

export type ConnectWizardState = {
	phase: HwWalletPhase
	account: WalletAccountInfo | null
	selectedEntry: HwAddressEntry | null
}

/**
 * The wizard state that matches a session wallet: seeded past Connect signer when the session
 * already holds one (Back from offline), and back on Connect signer when it holds none.
 *
 * The session wallet is the source of truth. A wizard phase that outlives the wallet strands the
 * signer on a card for a device that is no longer connected — and since the header's Disconnect
 * only shows while that phase lasts, pressing it there looks like it does nothing.
 */
export function connectStateForWallet(wallet: WalletAccountInfo | null): ConnectWizardState {
	if (wallet === null) {
		return { phase: 'connect', account: null, selectedEntry: null }
	}
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

/** True when a navigation tagged an authority step, as opposed to a plain entry with no state. */
export function hasAuthorityStep(state: unknown): boolean {
	return (state as { authorityStep?: unknown } | null)?.authorityStep != null
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
