import type { HwDeviceType } from '../model/hw-device'
import { QrCode } from '@/components/qr-code'
import { useClipboardCopy } from '@/hooks/use-clipboard-copy'
import { CopyClipboardIcon, CheckEmeraldIcon } from '@/assets/icons'
import { buildReceiveQrValue } from '../model/build-receive-qr-value'
import { receiveVerifyPathFromAccount } from '../model/build-verify-path'
import { networkFromPath } from '../model/network-from-path'
import { VerifyOnDeviceButton } from './verify-on-device-button'

/** Present only for HW sessions: drives the verify-on-device affordance (PRD §4.3.4.2). */
export type ReceiveVerifyContext = {
	deviceType: HwDeviceType
	/** Connect-returned BIP-86 account path (device-specific coin type), e.g. m/86'/1'/73'. */
	accountPath: string
	index: number
	/**
	 * Device-accurate verification value: the receive address re-encoded with the HRP the
	 * device shows (tb1… / bc1…). Absent on mainnet, where it equals the funding address.
	 */
	verifyAddress?: string
}

export type ReceiveAddressRowProps = {
	address: string
	isLoading?: boolean
	/** When set, renders a "Verify on device" affordance for the receive address (P2TR). */
	verify?: ReceiveVerifyContext
}

/**
 * Receive address card (PRD §4.3.4.1): shows the first unused address as text
 * AND a QR code. Clicking the address text or the QR copies the address to the
 * clipboard (§4.3.4.1.1), with shared "Copied!" feedback announced to assistive
 * tech.
 */
export function ReceiveAddressRow({ address, isLoading, verify }: ReceiveAddressRowProps) {
	const { copied, copy } = useClipboardCopy()

	if (isLoading) {
		return (
			<div className="rounded-xl border border-[#f3f4f6] px-4 py-3">
				<div className="mb-3 h-3 w-24 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="mx-auto mb-3 h-[132px] w-[132px] animate-pulse rounded-lg bg-[#e5e7eb]" />
				<div className="flex items-center justify-between gap-2">
					<div className="h-4 flex-1 animate-pulse rounded bg-[#e5e7eb]" />
					<div className="h-7 w-7 animate-pulse rounded bg-[#e5e7eb]" />
				</div>
			</div>
		)
	}

	if (!address) {
		return (
			<div className="rounded-xl border border-[#f3f4f6] px-4 py-3">
				<span className="text-mono-sm font-medium uppercase tracking-[0.08em] text-[#9ca3af]">Receive address</span>
				<p className="mt-1.5 text-label text-[#9ca3af]">No receive address yet</p>
			</div>
		)
	}

	const qrValue = buildReceiveQrValue(address)

	function handleCopy() {
		copy(qrValue)
	}

	return (
		<div className="rounded-xl border border-[#f3f4f6] px-4 py-3" data-testid="e2e-wallet-receive-address-row">
			<div className="flex items-center justify-between gap-2">
				<span className="text-mono-sm font-medium uppercase tracking-[0.08em] text-[#9ca3af]">Receive address</span>
				<span
					aria-live="polite"
					className={`text-mono-sm font-medium transition-opacity ${copied ? 'text-[#059669] opacity-100' : 'text-[#9ca3af] opacity-100'}`}
				>
					{copied ? 'Copied!' : ''}
				</span>
			</div>

			<div className="mt-3 flex flex-col items-center gap-3">
				<button
					type="button"
					onClick={handleCopy}
					aria-label={copied ? 'Address copied' : 'Copy receive address from QR code'}
					title="Click to copy"
					data-testid="e2e-wallet-receive-qr"
					className="rounded-lg border border-[#f3f4f6] bg-white p-2.5 transition hover:border-[#d1d5db] focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
				>
					<QrCode value={qrValue} size={132} ariaLabel="Receive address QR code" />
				</button>

				<div className="flex w-full items-center justify-between gap-2">
					<button
						type="button"
						onClick={handleCopy}
						title={address}
						aria-label={copied ? 'Address copied' : 'Copy receive address'}
						data-testid="e2e-wallet-receive-address-value"
						className="min-w-0 flex-1 truncate rounded-md text-left font-mono text-label leading-[1.45] text-[#374151] transition hover:text-[#111827] focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
					>
						{address}
					</button>
					<button
						type="button"
						onClick={handleCopy}
						aria-label={copied ? 'Address copied' : 'Copy receive address'}
						title={copied ? 'Copied' : 'Copy'}
						className="inline-flex shrink-0 items-center justify-center rounded-md p-1.5 text-[#9ca3af] transition hover:bg-[#f3f4f6] hover:text-[#6b7280]"
					>
						{copied ? <CheckEmeraldIcon width={14} height={14} /> : <CopyClipboardIcon width={14} height={14} />}
					</button>
				</div>

				{verify && (
					<div className="w-full">
						<VerifyOnDeviceButton
							deviceType={verify.deviceType}
							network={networkFromPath(verify.accountPath)}
							derivationPath={receiveVerifyPathFromAccount(verify.accountPath, verify.index)}
							scriptType="p2tr"
							subject="receive address"
						/>
						{verify.verifyAddress && verify.verifyAddress !== address && (
							<div className="mt-2 rounded-lg border border-[#f3f4f6] bg-[#fafafa] px-3 py-2">
								<p className="text-mono-sm font-medium uppercase tracking-[0.08em] text-[#9ca3af]">
									On your device, confirm this exact address
								</p>
								<p className="mt-1 break-all font-mono text-label leading-[1.45] text-[#374151]">
									{verify.verifyAddress}
								</p>
								<p className="mt-1 text-mono-sm leading-[1.45] text-[#9ca3af]">
									The prefix differs from the funding address above (the device firmware has no regtest coin); the
									characters after the prefix must match exactly.
								</p>
							</div>
						)}
					</div>
				)}
			</div>
		</div>
	)
}
