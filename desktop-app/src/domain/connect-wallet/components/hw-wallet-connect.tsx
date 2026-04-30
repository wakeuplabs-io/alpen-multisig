import {
	AuthoritySelectionPhase,
	type AuthorityOption,
} from '@/domain/connect-wallet/components/authority-selection-phase'
import { AuthenticateSessionPhase } from '@/domain/connect-wallet/components/authenticate-session-phase'
import { ConnectPhase } from '@/domain/connect-wallet/components/connect-phase'
import { PickingPhase } from '@/domain/connect-wallet/components/picking-phase'
import { SelectedPhase } from '@/domain/connect-wallet/components/selected-phase'
import { useHwWalletConnect } from '@/domain/connect-wallet/hooks/use-hw-wallet-connect'
import type { WalletAccountInfo, WalletAdapter } from '@/wallet/types'

type Props = {
	adapter: WalletAdapter
	onConnected: (info: WalletAccountInfo | null) => void
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

export function HwWalletConnect({ adapter, onConnected, authoritySelection }: Props) {
	const { state, actions } = useHwWalletConnect({ adapter, onConnected })
	const isWidePhase = state.phase === 'picking' || (state.phase === 'selected' && authoritySelection !== null)

	return (
		<section
			className={`w-full ${
				isWidePhase
					? 'max-w-[900px] rounded-none border-none bg-transparent p-0 shadow-none'
					: 'max-w-[520px] rounded-2xl border border-[#e5e7eb] bg-white p-7 shadow-[0_1px_4px_rgba(0,0,0,0.06)]'
			}`}
		>
			{state.phase === 'connect' && (
				<ConnectPhase
					loading={state.loading}
					connectViewState={state.connectViewState}
					error={state.error}
					onConnect={() => void actions.connect()}
				/>
			)}
			{state.phase === 'picking' && (
				<PickingPhase
					addresses={state.addresses}
					selectedIndex={state.selectedIndex}
					onSelectIndex={actions.selectAddressIndex}
					onBack={actions.goBackToConnect}
					onUseAddress={actions.useAddress}
					onDisconnect={actions.disconnect}
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
						onDisconnect={actions.disconnect}
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
					onDisconnect={actions.disconnect}
				/>
			)}
		</section>
	)
}
