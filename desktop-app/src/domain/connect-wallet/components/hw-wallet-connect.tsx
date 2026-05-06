import { useEffect, type MutableRefObject } from 'react'
import {
	AuthoritySelectionPhase,
	type AuthorityOption,
} from '@/domain/connect-wallet/components/authority-selection-phase'
import { AuthenticateSessionPhase } from '@/domain/connect-wallet/components/authenticate-session-phase'
import { ConnectPhase } from '@/domain/connect-wallet/components/connect-phase'
import { PickingPhase } from '@/domain/connect-wallet/components/picking-phase'
import { SelectedPhase } from '@/domain/connect-wallet/components/selected-phase'
import { useHwWalletConnect } from '@/domain/connect-wallet/hooks/use-hw-wallet-connect'
import type { WalletAccountInfo, WalletAdapter, WalletVendor } from '@/wallet/types'

type Props = {
	adapter: WalletAdapter
	walletVendor: WalletVendor
	onSelectWalletMethod: (method: 'trezor' | 'mnemonic', mnemonic?: string) => void
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
		onSelectAuthority: (authorityId: string) => void
		onContinueToAuthenticate: () => void
		onBackToAuthority: () => void
		onAuthenticate: () => void
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
	const isWidePhase = state.phase === 'picking' || (state.phase === 'selected' && authoritySelection !== null)

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
					? 'max-w-[840px] rounded-none border-none bg-transparent p-0 shadow-none'
					: 'max-w-[520px] rounded-2xl border border-[#e5e7eb] bg-white p-7 shadow-[0_1px_4px_rgba(0,0,0,0.06)]'
			}`}
		>
			{state.phase === 'connect' && (
				<ConnectPhase
					loading={state.loading}
					connectViewState={state.connectViewState}
					error={state.error}
					onConnect={() => void actions.connect()}
					walletVendor={walletVendor}
					onSelectWalletMethod={onSelectWalletMethod}
				/>
			)}
			{state.phase === 'picking' && (
				<PickingPhase
					addresses={state.addresses}
					selectedIndex={state.selectedIndex}
					onSelectIndex={actions.selectAddressIndex}
					onBack={actions.goBackToConnect}
					onUseAddress={actions.useAddress}
				/>
			)}
			{state.phase === 'selected' &&
				state.selectedEntry &&
				state.account &&
				authoritySelection !== null &&
				authoritySelection.step === 'select-authority' && (
					<AuthoritySelectionPhase
						selectedAuthorityId={authoritySelection.selectedAuthorityId}
						options={authoritySelection.options}
						onSelectAuthority={authoritySelection.onSelectAuthority}
						onContinueToAuthenticate={authoritySelection.onContinueToAuthenticate}
						onBackToAddresses={actions.changeAddress}
					/>
				)}
			{state.phase === 'selected' &&
				state.selectedEntry &&
				state.account &&
				authoritySelection !== null &&
				authoritySelection.step === 'authenticate-session' && (
					<AuthenticateSessionPhase
						authorityLabel={authoritySelection.selectedAuthorityLabel ?? 'Selected authority'}
						signerAddress={state.selectedEntry.address}
						compressedPublicKey={state.selectedEntry.publicKeyHex}
						isAuthenticating={authoritySelection.isAuthenticating}
						authError={authoritySelection.authError}
						authOkMessage={authoritySelection.authOkMessage}
						onBackToAuthority={authoritySelection.onBackToAuthority}
						onAuthenticate={authoritySelection.onAuthenticate}
					/>
				)}
			{state.phase === 'selected' && state.selectedEntry && state.account && authoritySelection === null && (
				<SelectedPhase
					account={state.account}
					selectedEntry={state.selectedEntry}
					isVerifyingAddress={state.isVerifyingAddress}
					verifyMessage={state.verifyMessage}
					onVerifyOnDevice={() => void actions.verifyOnDevice()}
					onChangeAddress={actions.changeAddress}
				/>
			)}
		</section>
	)
}
