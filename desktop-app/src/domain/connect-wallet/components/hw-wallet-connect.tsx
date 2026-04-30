import { ConnectPhase } from '@/domain/connect-wallet/components/connect-phase'
import { PickingPhase } from '@/domain/connect-wallet/components/picking-phase'
import { SelectedPhase } from '@/domain/connect-wallet/components/selected-phase'
import { useHwWalletConnect } from '@/domain/connect-wallet/hooks/use-hw-wallet-connect'
import type { WalletAccountInfo, WalletAdapter } from '@/wallet/types'

type Props = {
	adapter: WalletAdapter
	onConnected: (info: WalletAccountInfo | null) => void
}

export function HwWalletConnect({ adapter, onConnected }: Props) {
	const { state, actions } = useHwWalletConnect({ adapter, onConnected })
	const isPickingPhase = state.phase === 'picking'

	return (
		<section
			className={`w-full ${
				isPickingPhase
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
			{state.phase === 'selected' && state.selectedEntry && state.account && (
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
