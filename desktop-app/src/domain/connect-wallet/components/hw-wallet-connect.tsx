import { useEffect, useRef, useState, type MutableRefObject } from 'react'
import {
	AuthoritySelectionPhase,
	type AuthorityOption,
} from '@/domain/connect-wallet/components/authority-selection-phase'
import { AuthenticateSessionPhase } from '@/domain/connect-wallet/components/authenticate-session-phase'
import type { SigningStepInfo } from '@/contexts/session-context'
import { ConnectPhase } from '@/domain/connect-wallet/components/connect-phase'
import { SelectedPhase } from '@/domain/connect-wallet/components/selected-phase'
import { deviceCopy } from '@/lib/device-copy'
import { useHwWalletConnect } from '@/domain/connect-wallet/hooks/use-hw-wallet-connect'
import { useAuthorityMembership } from '@/domain/connect-wallet/hooks/use-authority-membership'
import { useMnemonicSigningEnabled } from '@/domain/connect-wallet/hooks/use-mnemonic-signing-enabled'
import type { WalletAccountInfo, WalletAdapter, WalletKind, WalletVendor } from '@/wallet/types'

type Props = {
	adapter: WalletAdapter
	walletVendor: WalletVendor
	onSelectWalletMethod: (method: 'trezor' | 'ledger' | 'mnemonic', mnemonic?: string) => void
	onConnected: (info: WalletAccountInfo | null) => void
	/** Wired to the shell header so Disconnect lives only in the top bar. */
	disconnectRef?: MutableRefObject<(() => void) | null>
	onHardwareSessionChange?: (active: boolean) => void
	authoritySelection: {
		step: 'select-authority' | 'authenticate-session'
		selectedAuthorityId: string | null
		selectedAuthorityLabel: string | null
		options: AuthorityOption[]
		isAuthenticating: boolean
		isAuthenticated: boolean
		authError: string | null
		authOkMessage: string | null
		signingStep: SigningStepInfo | null
		onSelectAuthority: (authorityId: string) => void
		onContinueToAuthenticate: () => void
		onBackToAuthority: () => void
		onAuthenticate: () => void
		onManualProposal: () => void
	} | null
}

export function HwWalletConnect({
	adapter,
	walletVendor,
	onSelectWalletMethod,
	onConnected,
	disconnectRef,
	onHardwareSessionChange,
	authoritySelection,
}: Props) {
	const { state, actions } = useHwWalletConnect({ adapter, onConnected })
	const mnemonicEnabled = useMnemonicSigningEnabled()
	const isWidePhase = state.phase === 'selected' && authoritySelection !== null

	// Holds the wallet the pending auto-connect is for, or null when none is queued. It carries
	// the kind rather than a bare flag because selecting the vendor swaps the adapter through
	// context asynchronously, so the choice has to survive until the adapter is in place.
	const [pendingConnect, setPendingConnect] = useState<WalletKind | null>(null)
	const connectRef = useRef(actions.connect)
	useEffect(() => {
		connectRef.current = actions.connect
	})
	useEffect(() => {
		if (pendingConnect === null) return
		setPendingConnect(null)
		void connectRef.current(pendingConnect)
	}, [pendingConnect])

	function handleConnectTrezor(kind: WalletKind = 'standard') {
		onSelectWalletMethod('trezor')
		setPendingConnect(kind)
	}

	function handleConnectMnemonic(mnemonic: string) {
		onSelectWalletMethod('mnemonic', mnemonic)
		setPendingConnect('standard')
	}

	const signerPubkeyHex = state.phase === 'selected' ? (state.selectedEntry?.publicKeyHex ?? null) : null
	const { resolvedOptions, isChecking } = useAuthorityMembership(signerPubkeyHex, authoritySelection?.options ?? [])

	// TODO: Refactor this to use a context
	useEffect(() => {
		if (!disconnectRef) {
			return
		}
		disconnectRef.current = actions.disconnect
		return () => {
			disconnectRef.current = null
		}
	}, [disconnectRef, actions.disconnect])

	useEffect(() => {
		onHardwareSessionChange?.(state.phase !== 'connect')
	}, [state.phase, onHardwareSessionChange])

	return (
		<section
			className={`mx-auto w-full ${
				isWidePhase
					? 'max-w-150 rounded-none border-none bg-transparent p-0 shadow-none'
					: 'max-w-130 rounded-2xl border border-[#e5e7eb] bg-white p-7 shadow-[0_1px_4px_rgba(0,0,0,0.06)]'
			}`}
		>
			{state.phase === 'connect' && (
				<ConnectPhase
					loading={state.loading}
					connectViewState={state.connectViewState}
					error={state.error}
					onConnect={() => void actions.connect()}
					onConnectTrezor={handleConnectTrezor}
					onConnectMnemonic={handleConnectMnemonic}
					walletVendor={walletVendor}
					onSelectWalletMethod={onSelectWalletMethod}
					mnemonicEnabled={mnemonicEnabled}
				/>
			)}
			{state.phase === 'selected' &&
				state.selectedEntry &&
				state.account &&
				authoritySelection !== null &&
				authoritySelection.step === 'select-authority' && (
					<AuthoritySelectionPhase
						selectedAuthorityId={authoritySelection.selectedAuthorityId}
						options={resolvedOptions}
						adminId={state.selectedEntry.publicKeyHex}
						isChecking={isChecking}
						onSelectAuthority={authoritySelection.onSelectAuthority}
						onContinueToAuthenticate={authoritySelection.onContinueToAuthenticate}
						onBack={actions.goBackToConnect}
					/>
				)}
			{state.phase === 'selected' &&
				state.selectedEntry &&
				state.account &&
				authoritySelection !== null &&
				authoritySelection.step === 'authenticate-session' && (
					<AuthenticateSessionPhase
						authorityLabel={authoritySelection.selectedAuthorityLabel ?? 'Selected authority'}
						adapterLabel={deviceCopy(walletVendor).label}
						compressedPublicKey={state.selectedEntry.publicKeyHex}
						isAuthenticating={authoritySelection.isAuthenticating}
						authError={authoritySelection.authError}
						authOkMessage={authoritySelection.authOkMessage}
						signingStep={authoritySelection.signingStep}
						walletVendor={walletVendor}
						onBackToAuthority={authoritySelection.onBackToAuthority}
						onAuthenticate={authoritySelection.onAuthenticate}
						onManualProposal={authoritySelection.onManualProposal}
					/>
				)}
			{state.phase === 'selected' && state.selectedEntry && state.account && authoritySelection === null && (
				<SelectedPhase
					account={state.account}
					selectedEntry={state.selectedEntry}
					walletVendor={walletVendor}
					isVerifyingAddress={state.isVerifyingAddress}
					verifyMessage={state.verifyMessage}
					onVerifyOnDevice={() => void actions.verifyOnDevice()}
				/>
			)}
		</section>
	)
}
