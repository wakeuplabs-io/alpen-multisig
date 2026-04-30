import { ConnectionIcon, SuccessIcon } from '@/domain/connect-wallet/components/hw-wallet-connect-icons'
import type { ConnectViewState } from '@/domain/connect-wallet/model/hw-wallet-connect.types'

type Props = {
	loading: boolean
	connectViewState: ConnectViewState
	error: string | null
	onConnect: () => void
}

export function ConnectPhase({ loading, connectViewState, error, onConnect }: Props) {
	return (
		<>
			<div
				className={`mb-5 flex h-[116px] items-center justify-center rounded-xl border border-[#e5e7eb] bg-[#f3f4f6] ${
					loading ? 'opacity-75' : ''
				}`}
			>
				<div
					className={`flex h-[42px] w-[42px] items-center justify-center rounded-[10px] border bg-white ${
						connectViewState === 'success'
							? 'border-[#a7f3d0] bg-[#ecfdf5] text-[#059669]'
							: loading
								? 'border-[#ddd8ff] ring-4 ring-[#9480f533]'
								: 'border-[#e5e7eb]'
					}`}
				>
					<ConnectionIcon state={connectViewState} />
				</div>
			</div>
			<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2.15rem] font-normal leading-[1.1] tracking-[-0.01em] text-[#0a0a0a]">
				{connectViewState === 'success' ? 'Device connected' : 'Connect your hardware wallet'}
			</h1>
			<p className="mb-0 mt-3 text-[0.88rem] leading-[1.55] text-[#6b7280]">
				{connectViewState === 'success'
					? 'Trezor detected. Loading available addresses...'
					: 'Plug in your Trezor and unlock it. We will detect the device automatically - no password or seed is ever shared.'}
			</p>
			<button
				className={`mt-4 flex w-full items-center justify-center gap-2 rounded-lg px-4 py-[0.7rem] text-[0.92rem] font-medium ${
					connectViewState === 'success'
						? 'cursor-not-allowed border border-[#a3a3a3] bg-[#a3a3a3] text-white'
						: 'border border-[#0a0a0a] bg-[#0a0a0a] text-white disabled:cursor-not-allowed disabled:opacity-45'
				}`}
				onClick={onConnect}
				disabled={loading || connectViewState === 'success'}
			>
				{loading && connectViewState !== 'success' && (
					<span
						className="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/40 border-t-white"
						aria-hidden="true"
					/>
				)}
				{connectViewState === 'success' ? (
					<>
						<SuccessIcon />
						Connected
					</>
				) : loading ? (
					'Reading addresses from device...'
				) : (
					'Connect Trezor'
				)}
			</button>
			{loading && <p className="mb-0 mt-3 text-center text-xs text-[#6b7280]">Looking for a Trezor on USB...</p>}
			{connectViewState === 'success' && (
				<div className="mt-3 flex items-start gap-2 rounded-lg border border-[#a7f3d0] bg-[#ecfdf5] px-3 py-2">
					<span className="mt-px text-[#059669]">
						<SuccessIcon />
					</span>
					<div>
						<p className="m-0 text-xs font-medium text-[#059669]">Trezor Model T detected</p>
						<p className="m-0 text-xs text-[#047857]">Advancing to address selection...</p>
					</div>
				</div>
			)}
			<p className="mb-0 mt-4 text-center text-xs text-[#9ca3af]">
				Your keys never leave the device. Alpen only receives signatures.
			</p>
			{connectViewState !== 'success' && (
				<p className="mb-0 mt-4 text-center text-xs text-[#9ca3af]">
					Ledger support · <span className="font-medium text-[#6b7280]">Coming Soon</span>
				</p>
			)}
			{error && <p className="mt-3 text-[0.85rem] text-[#c0392b]">{error}</p>}
		</>
	)
}
