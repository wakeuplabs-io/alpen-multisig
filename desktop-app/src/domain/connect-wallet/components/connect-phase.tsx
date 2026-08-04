import { useState } from 'react'
import { tauriCall } from '@/api/tauri-bridge'
import { ShieldCheckMutedIcon, UsbStrokeWhiteIcon } from '@/assets/icons'
import { ConnectionIcon, SuccessIcon } from '@/domain/connect-wallet/components/hw-wallet-connect-icons'
import { useDevicePassphraseEntry } from '@/domain/connect-wallet/hooks/use-device-passphrase-entry'
import type { ConnectViewState } from '@/domain/connect-wallet/model/hw-wallet-connect.types'
import { deviceCopy } from '@/lib/device-copy'
import { DEMO_MNEMONIC } from '@/wallet/demo-mnemonic'
import type { HwAddressEntry, WalletVendor } from '@/wallet/types'

type Props = {
	loading: boolean
	connectViewState: ConnectViewState
	error: string | null
	onConnect: () => void
	onConnectMnemonic?: (mnemonic: string) => void
	onConnectTrezor?: () => void
	walletVendor: WalletVendor
	onSelectWalletMethod: (method: 'trezor' | 'ledger' | 'mnemonic', mnemonic?: string) => void
	mnemonicEnabled: boolean
}

export function ConnectPhase({
	loading,
	connectViewState,
	error,
	onConnect,
	onConnectMnemonic,
	onConnectTrezor,
	walletVendor,
	onSelectWalletMethod,
	mnemonicEnabled,
}: Props) {
	const isDetecting = loading && connectViewState !== 'success'
	const isSuccess = connectViewState === 'success'
	const [mnemonicInput, setMnemonicInput] = useState(DEMO_MNEMONIC)
	const [mnemonicError, setMnemonicError] = useState<string | null>(null)
	const [debugEntry, setDebugEntry] = useState<HwAddressEntry | null>(null)
	const [debugLoading, setDebugLoading] = useState(false)
	const [debugError, setDebugError] = useState<string | null>(null)
	const passphraseEntry = useDevicePassphraseEntry(walletVendor)
	const passphraseCopy = deviceCopy(walletVendor).passphraseOnDevice

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

	async function handleShowDebugAddress() {
		const words = mnemonicInput.trim() || DEMO_MNEMONIC
		setDebugLoading(true)
		setDebugError(null)
		setDebugEntry(null)
		const result = await tauriCall<HwAddressEntry[]>('list_mnemonic_addresses', {
			mnemonic: words,
			count: 1,
		})
		setDebugLoading(false)
		if (!result.ok) {
			setDebugError(result.error)
			return
		}
		setDebugEntry(result.data[0] ?? null)
	}

	return (
		<>
			{/* Device icon area */}
			<div
				className={`relative mb-5 flex h-33 items-center justify-center rounded-xl border border-[#e5e7eb] bg-bg-base ${
					isDetecting ? 'hw-detect-pulse' : ''
				}`}
			>
				<div
					className={`relative z-10 flex h-14 w-14 items-center justify-center rounded-[14px] border bg-white transition-all duration-200 ${
						isSuccess
							? 'border-[#a7f3d0] bg-[#ecfdf5] text-[#059669]'
							: isDetecting
								? 'border-accent-border'
								: 'border-[#e5e7eb]'
					}`}
				>
					<ConnectionIcon state={connectViewState} />
				</div>
			</div>

			{/* Heading */}
			<h1 className="m-0 font-display text-[32px] font-normal leading-[1.2] tracking-[-0.01em] text-[#0a0a0a]">
				{isSuccess
					? 'Device connected'
					: walletVendor === 'mnemonic'
						? 'Connect with your seed words'
						: 'Connect your hardware wallet'}
			</h1>

			{/* Subtitle */}
			<p className="mb-0 mt-2.5 text-body leading-[1.6] text-[#6b7280]">
				{isSuccess
					? 'Device detected. Loading canonical signer…'
					: walletVendor === 'mnemonic'
						? 'Mnemonic mode selected. Connect to continue with the words provided below.'
						: walletVendor === 'ledger'
							? 'Plug in your Ledger and unlock it. Open the Bitcoin app on the device before connecting.'
							: 'Plug in your Trezor and unlock it. We will detect the device automatically — no password or seed is ever shared.'}
			</p>

			<div className="mt-4 rounded-lg border border-[#e5e7eb] bg-[#fafafa] p-3">
				<p className="m-0 text-mono-sm font-medium uppercase tracking-[0.12em] text-[#9ca3af]">Connection method</p>
				<div className="mt-2 flex items-center gap-2">
					<button
						type="button"
						data-testid="e2e-connect-trezor"
						className={`rounded-md border px-3 py-1.5 text-label font-medium transition ${
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
						className={`rounded-md border px-3 py-1.5 text-label font-medium transition ${
							walletVendor === 'ledger'
								? 'border-[#0a0a0a] bg-[#0a0a0a] text-white'
								: 'border-[#d1d5db] bg-white text-[#374151] hover:bg-[#f3f4f6]'
						}`}
						onClick={handleUseLedger}
					>
						Ledger
					</button>
					{mnemonicEnabled && (
						<button
							type="button"
							data-testid="e2e-connect-mnemonic"
							className={`rounded-md border px-3 py-1.5 text-label font-medium transition ${
								walletVendor === 'mnemonic'
									? 'border-[#0a0a0a] bg-[#0a0a0a] text-white'
									: 'border-[#d1d5db] bg-white text-[#374151] hover:bg-[#f3f4f6]'
							}`}
							onClick={handleUseMnemonic}
						>
							Mnemonic
						</button>
					)}
				</div>
				{mnemonicEnabled && (
					<textarea
						data-testid="e2e-connect-mnemonic-textarea"
						className="mt-2 w-full rounded-md border border-[#d1d5db] bg-white px-3 py-2 text-label text-[#111827] outline-none focus:border-[#9ca3af]"
						rows={2}
						placeholder="seed words..."
						value={mnemonicInput}
						onChange={(event) => setMnemonicInput(event.target.value)}
					/>
				)}
				{passphraseCopy && passphraseEntry.supported === true && (
					<div className="mt-3" data-testid="e2e-passphrase-on-device">
						<button
							type="button"
							data-testid="e2e-passphrase-on-device-button"
							className="w-full rounded-md border border-[#d1d5db] bg-white px-3 py-2 text-label font-medium text-[#374151] transition hover:bg-[#f3f4f6] disabled:cursor-not-allowed disabled:opacity-60"
							onClick={() => onConnectTrezor?.()}
							disabled={loading || isSuccess}
						>
							{passphraseCopy.label}
						</button>
						<p className="m-0 mt-1.5 text-label leading-[1.5] text-[#6b7280]">{passphraseCopy.hint}</p>
					</div>
				)}
				{passphraseCopy && passphraseEntry.supported === false && (
					<p
						className="m-0 mt-3 text-label leading-[1.5] text-[#6b7280]"
						data-testid="e2e-passphrase-on-device-unsupported"
					>
						{passphraseCopy.unsupportedHint}
					</p>
				)}
				{mnemonicError !== null && <p className="m-0 mt-1 text-label text-danger">{mnemonicError}</p>}

				{walletVendor === 'mnemonic' && import.meta.env.DEV && (
					<>
						<button
							type="button"
							data-testid="e2e-debug-show-address"
							className="mt-2 w-full rounded-md border border-[#d1d5db] bg-white px-3 py-1.5 text-label font-medium text-[#374151] transition hover:bg-[#f3f4f6] disabled:cursor-not-allowed disabled:opacity-60"
							onClick={handleShowDebugAddress}
							disabled={debugLoading}
						>
							{debugLoading ? 'Deriving address…' : 'Show compressed pubkey / address (debug)'}
						</button>
						{debugError !== null && <p className="m-0 mt-1 text-label text-danger">{debugError}</p>}
						{debugEntry !== null && (
							<div className="mt-2 rounded-md border border-[#e5e7eb] bg-white p-2 text-mono-sm text-[#374151]">
								<p className="m-0 break-all">
									<span className="font-medium text-[#6b7280]">Public key: </span>
									{debugEntry.publicKeyHex}
								</p>
								<p className="m-0 mt-1 break-all">
									<span className="font-medium text-[#6b7280]">Address: </span>
									{debugEntry.address}
								</p>
								<p className="m-0 mt-1 break-all">
									<span className="font-medium text-[#6b7280]">Path: </span>
									{debugEntry.derivationPath}
								</p>
							</div>
						)}
					</>
				)}
			</div>

			{/* Status message */}
			{isDetecting && (
				<div className="mt-5 flex items-center gap-2.5 rounded-lg border border-accent-border bg-bg-surface px-3.5 py-3 text-body-sm">
					<span
						className="h-3.5 w-3.5 flex-none animate-spin rounded-full border-2"
						style={{ borderColor: 'var(--color-accent-border)', borderTopColor: 'var(--color-accent)' }}
						aria-hidden="true"
					/>
					<div className="flex-1">
						<div className="font-medium text-[#0a0a0a]">Detecting device…</div>
						<div className="mt-0.5 text-label text-[#6b7280]">
							{walletVendor === 'ledger' ? 'Looking for a Ledger on USB.' : 'Looking for a Trezor on USB.'}
						</div>
					</div>
				</div>
			)}

			{isSuccess && (
				<div className="mt-5 flex items-center gap-2.5 rounded-lg border border-[#a7f3d0] bg-[#ecfdf5] px-3.5 py-3 text-body-sm">
					<SuccessIcon tone="emerald" />
					<div className="flex-1">
						<div className="font-medium text-[#059669]">
							{walletVendor === 'ledger' ? 'Ledger detected' : 'Trezor detected'}
						</div>
						<div className="mt-0.5 text-label text-[#047857]">Advancing to authority selection…</div>
					</div>
				</div>
			)}

			{/* Action button */}
			<button
				data-testid="e2e-connect-with-words"
				className={`mt-5 flex w-full items-center justify-center gap-2 rounded-lg px-4 py-3 text-body font-medium transition active:scale-[0.98] ${
					isSuccess || isDetecting
						? 'cursor-not-allowed border border-[#a3a3a3] bg-[#a3a3a3] text-white opacity-70'
						: 'border border-[#0a0a0a] bg-[#0a0a0a] text-white hover:bg-[#2a2a2a]'
				}`}
				onClick={
					walletVendor === 'mnemonic' && onConnectMnemonic
						? () => onConnectMnemonic(mnemonicInput.trim() || DEMO_MNEMONIC)
						: walletVendor === 'trezor' && onConnectTrezor
							? () => onConnectTrezor()
							: onConnect
				}
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
						{walletVendor === 'mnemonic' ? 'Connect with mnemonic' : 'Connect wallet'}
					</>
				)}
			</button>

			{/* Security note */}
			<p className="mb-0 mt-5 flex items-center justify-center gap-2.5 text-center text-label text-[#9ca3af]">
				<ShieldCheckMutedIcon width={16} height={16} className="block shrink-0" />
				{walletVendor === 'mnemonic'
					? 'Your seed words are used locally to derive keys. Strata Multisig only receives signatures.'
					: 'Your keys never leave the device. Strata Multisig only receives signatures.'}
			</p>

			{error && <p className="mt-3 text-body-sm text-danger">{error}</p>}
		</>
	)
}
