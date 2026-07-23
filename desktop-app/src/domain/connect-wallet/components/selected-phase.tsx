import type { HwAddressEntry, WalletAccountInfo, WalletVendor } from '@/wallet/types'
import { deviceCopy } from '@/lib/device-copy'

type Props = {
	account: WalletAccountInfo
	selectedEntry: HwAddressEntry
	/** Signer being connected — drives the device-specific verification copy. */
	walletVendor: WalletVendor
	isVerifyingAddress: boolean
	verifyMessage: string | null
	onVerifyOnDevice: () => void
}

export function SelectedPhase({
	account: _account,
	selectedEntry: _selectedEntry,
	walletVendor,
	isVerifyingAddress,
	verifyMessage,
	onVerifyOnDevice,
}: Props) {
	return (
		<>
			<div className="flex flex-wrap gap-2">
				<button
					className="mt-4 w-full rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-[0.7rem] text-[0.92rem] font-medium text-white disabled:cursor-not-allowed disabled:opacity-45"
					onClick={onVerifyOnDevice}
					disabled={isVerifyingAddress}
				>
					{isVerifyingAddress ? 'Check device…' : 'Verify key/path on device'}
				</button>
			</div>
			{isVerifyingAddress && (
				<p className="mt-4 text-[0.85rem] text-[#666]">{deviceCopy(walletVendor).verifyOnDeviceHint}</p>
			)}
			{verifyMessage && (
				<p
					className={`mt-3 text-[0.85rem] ${verifyMessage.startsWith('Verification failed') ? 'text-[#c0392b]' : 'text-[#1d7a34]'}`}
				>
					{verifyMessage}
				</p>
			)}
		</>
	)
}
