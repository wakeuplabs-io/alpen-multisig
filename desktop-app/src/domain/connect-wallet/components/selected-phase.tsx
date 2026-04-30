import type { HwAddressEntry, WalletAccountInfo } from '@/wallet/types'
import { Label, Mono } from '@/domain/connect-wallet/components/hw-wallet-connect-shared'
import { buildAsmConfigSnippet } from '@/domain/connect-wallet/utils/hw-wallet-connect-utils'

type Props = {
	account: WalletAccountInfo
	selectedEntry: HwAddressEntry
	isVerifyingAddress: boolean
	verifyMessage: string | null
	onVerifyOnDevice: () => void
	onChangeAddress: () => void
	onDisconnect: () => void
}

export function SelectedPhase({
	account,
	selectedEntry,
	isVerifyingAddress,
	verifyMessage,
	onVerifyOnDevice,
	onChangeAddress,
	onDisconnect,
}: Props) {
	return (
		<>
			<div className="mt-4 flex flex-col gap-1 rounded-lg bg-[#f9f9f9] px-4 py-3">
				<Label>Device</Label>
				<Mono>{account.deviceLabel}</Mono>
				<Label className="mt-2">Selected path</Label>
				<Mono>{selectedEntry.derivationPath}</Mono>
				<Label className="mt-2">Selected address</Label>
				<Mono>{selectedEntry.address}</Mono>
				<Label className="mt-2">Signer public key (for ASM keys)</Label>
				<Mono>{selectedEntry.publicKeyHex}</Mono>
				<Label className="mt-2">ASM config snippet (copy/paste)</Label>
				<Mono>{buildAsmConfigSnippet(selectedEntry.publicKeyHex)}</Mono>
			</div>
			<div className="flex flex-wrap gap-2">
				<button
					className="mt-4 w-full rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-[0.7rem] text-[0.92rem] font-medium text-white disabled:cursor-not-allowed disabled:opacity-45"
					onClick={onVerifyOnDevice}
					disabled={isVerifyingAddress}
				>
					{isVerifyingAddress ? 'Check device…' : 'Verify key/path on device'}
				</button>
				<button
					className="mt-4 w-full rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-[0.7rem] text-[0.92rem] font-medium text-white"
					onClick={onChangeAddress}
				>
					Change address
				</button>
				<button
					className="mt-4 w-full rounded-lg border border-[#0a0a0a] bg-white px-4 py-[0.7rem] text-[0.92rem] font-medium text-[#222]"
					onClick={onDisconnect}
				>
					Disconnect
				</button>
			</div>
			{isVerifyingAddress && (
				<p className="mt-4 text-[0.85rem] text-[#666]">
					Check your Trezor screen and confirm the selected path/public key shown by the device.
				</p>
			)}
			{verifyMessage && (
				<p className={`mt-3 text-[0.85rem] ${verifyMessage.startsWith('Verification failed') ? 'text-[#c0392b]' : 'text-[#1d7a34]'}`}>
					{verifyMessage}
				</p>
			)}
		</>
	)
}
