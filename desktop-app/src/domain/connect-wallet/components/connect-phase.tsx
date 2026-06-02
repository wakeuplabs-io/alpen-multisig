import { useState } from 'react'
import { ShieldCheckMutedIcon, UsbStrokeWhiteIcon } from '@/assets/icons'
import { ConnectionIcon, SuccessIcon } from '@/domain/connect-wallet/components/hw-wallet-connect-icons'
import type { ConnectViewState } from '@/domain/connect-wallet/model/hw-wallet-connect.types'
import { DEMO_MNEMONIC } from '@/wallet/demo-mnemonic'
import type { WalletVendor } from '@/wallet/types'

type Props = {
	loading: boolean
	connectViewState: ConnectViewState
	error: string | null
	onConnect: () => void
	walletVendor: WalletVendor
	onSelectWalletMethod: (method: 'trezor' | 'ledger' | 'mnemonic', mnemonic?: string) => void
}

export function ConnectPhase({
	loading,
	connectViewState,
	error,
	onConnect,
	walletVendor,
	onSelectWalletMethod,
}: Props) {
	const isDetecting = loading && connectViewState !== 'success'
	const isSuccess = connectViewState === 'success'
	const [mnemonicInput, setMnemonicInput] = useState(DEMO_MNEMONIC)
	const [mnemonicError, setMnemonicError] = useState<string | null>(null)

	function handleUseTrezor() {
		onSelectWalletMethod('trezor')
		setMnemonicError(null)
	}

	function handleUseLedger() {
		onSelectWalletMethod('ledger')
		setMnemonicError(null)
	}

	function handleUseMnemonic() {
		const words = mnemonicInput.trim() || DEMO_MNEMONIC
		if (!words) {
			setMnemonicError('Enter your mnemonic words first.')
			return
		}
		onSelectWalletMethod('mnemonic', words)
		setMnemonicError(null)
	}

	return (
		<>
			{/* Device icon area */}
			<div
				className={`relative mb-5 flex h-33 items-center justify-center rounded-xl border border-[#e5e7eb] bg-[#f8f8fb] ${
					isDetecting ? 'hw-detect-pulse' : ''
				}`}
			>
				<div
					className={`relative z-10 flex h-14 w-14 items-center justify-center rounded-[14px] border bg-white transition-all duration-200 ${
						isSuccess
							? 'border-[#a7f3d0] bg-[#ecfdf5] text-[#059669]'
							: isDetecting
								? 'border-[#ddd8ff]'
								: 'border-[#e5e7eb]'
					}`}
				>
					<ConnectionIcon state={connectViewState} />
				</div>
			</div>

			{/* Heading */}
			<h1 className="m-0 font-['BIZ_UDPMincho'] text-[32px] font-normal leading-[1.2] tracking-[-0.01em] text-[#0a0a0a]">
				{isSuccess
					? 'Device connected'
					: walletVendor === 'mnemonic'
						? 'Connect with your seed words'
						: 'Connect your hardware wallet'}
			</h1>

			{/* Subtitle */}
			<p className="mb-0 mt-2.5 text-[14px] leading-[1.6] text-[#6b7280]">
				{isSuccess
					? 'Device detected. Loading canonical signer…'
					: walletVendor === 'mnemonic'
						? 'Mnemonic mode selected. Connect to continue with the words provided below.'
						: walletVendor === 'ledger'
							? 'Plug in your Ledger and unlock it. Open the Bitcoin app on the device before connecting.'
							: 'Plug in your Trezor and unlock it. We will detect the device automatically — no password or seed is ever shared.'}
			</p>

			<div className="mt-4 rounded-lg border border-[#e5e7eb] bg-[#fafafa] p-3">
				<p className="m-0 text-[11px] font-medium uppercase tracking-[0.12em] text-[#9ca3af]">Connection method</p>
				<div className="mt-2 flex items-center gap-2">
					<button
						type="button"
						className={`rounded-md border px-3 py-1.5 text-xs font-medium transition ${
							walletVendor === 'trezor'
								? 'border-[#0a0a0a] bg-[#0a0a0a] text-white'
								: 'border-[#d1d5db] bg-white text-[#374151] hover:bg-[#f3f4f6]'
						}`}
						onClick={handleUseTrezor}
					>
						Trezor
					</button>
					<button
						type="button"
						data-testid="e2e-connect-ledger"
						className={`rounded-md border px-3 py-1.5 text-xs font-medium transition ${
							walletVendor === 'ledger'
								? 'border-[#0a0a0a] bg-[#0a0a0a] text-white'
								: 'border-[#d1d5db] bg-white text-[#374151] hover:bg-[#f3f4f6]'
						}`}
						onClick={handleUseLedger}
					>
						Ledger
					</button>
					<button
						type="button"
						data-testid="e2e-connect-palabras"
						className={`rounded-md border px-3 py-1.5 text-xs font-medium transition ${
							walletVendor === 'mnemonic'
								? 'border-[#0a0a0a] bg-[#0a0a0a] text-white'
								: 'border-[#d1d5db] bg-white text-[#374151] hover:bg-[#f3f4f6]'
						}`}
						onClick={handleUseMnemonic}
					>
						Palabras
					</button>
				</div>
				<textarea
					data-testid="e2e-connect-mnemonic-textarea"
					className="mt-2 w-full rounded-md border border-[#d1d5db] bg-white px-3 py-2 text-xs text-[#111827] outline-none focus:border-[#9ca3af]"
					rows={2}
					placeholder="seed words..."
					value={mnemonicInput}
					onChange={(event) => setMnemonicInput(event.target.value)}
				/>
				{mnemonicError !== null && <p className="m-0 mt-1 text-[12px] text-[#dc2626]">{mnemonicError}</p>}
			</div>

			{/* Status message */}
			{isDetecting && (
				<div className="mt-5 flex items-center gap-2.5 rounded-lg border border-[#ddd8ff] bg-[#f8f7ff] px-3.5 py-3 text-[13px]">
					<span
						className="h-3.5 w-3.5 flex-none animate-spin rounded-full border-2"
						style={{ borderColor: '#ddd8ff', borderTopColor: '#9480f5' }}
						aria-hidden="true"
					/>
					<div className="flex-1">
						<div className="font-medium text-[#0a0a0a]">Detecting device…</div>
						<div className="mt-0.5 text-[12px] text-[#6b7280]">
							{walletVendor === 'ledger' ? 'Looking for a Ledger on USB.' : 'Looking for a Trezor on USB.'}
						</div>
					</div>
				</div>
			)}

			{isSuccess && (
				<div className="mt-5 flex items-center gap-2.5 rounded-lg border border-[#a7f3d0] bg-[#ecfdf5] px-3.5 py-3 text-[13px]">
					<SuccessIcon tone="emerald" />
					<div className="flex-1">
						<div className="font-medium text-[#059669]">
							{walletVendor === 'ledger' ? 'Ledger detected' : 'Trezor detected'}
						</div>
						<div className="mt-0.5 text-[12px] text-[#047857]">Advancing to authority selection…</div>
					</div>
				</div>
			)}

			{/* Action button */}
			<button
				data-testid="e2e-connect-with-words"
				className={`mt-5 flex w-full items-center justify-center gap-2 rounded-lg px-4 py-3 text-[14px] font-medium transition active:scale-[0.98] ${
					isSuccess || isDetecting
						? 'cursor-not-allowed border border-[#a3a3a3] bg-[#a3a3a3] text-white opacity-70'
						: 'border border-[#0a0a0a] bg-[#0a0a0a] text-white hover:bg-[#2a2a2a]'
				}`}
				onClick={onConnect}
				disabled={loading || isSuccess}
			>
				{isSuccess ? (
					<>
						<SuccessIcon tone="white" />
						Connected
					</>
				) : isDetecting ? (
					'Detecting…'
				) : (
					<>
						<UsbStrokeWhiteIcon width={20} height={20} className="block shrink-0" />
						{walletVendor === 'mnemonic' ? 'Connect with words' : 'Connect wallet'}
					</>
				)}
			</button>

			{/* Security note */}
			<p className="mb-0 mt-5 flex items-center justify-center gap-2.5 text-center text-[12px] text-[#9ca3af]">
				<ShieldCheckMutedIcon width={16} height={16} className="block shrink-0" />
				{walletVendor === 'mnemonic'
					? 'Your seed words are used locally to derive keys. Alpen only receives signatures.'
					: 'Your keys never leave the device. Alpen only receives signatures.'}
			</p>

			{error && <p className="mt-3 text-[13px] text-[#dc2626]">{error}</p>}
		</>
	)
}
