import { QrCode } from '@/components/qr-code'
import { useClipboardCopy } from '@/hooks/use-clipboard-copy'
import { CopyClipboardIcon, CheckEmeraldIcon } from '@/assets/icons'
import { buildReceiveQrValue } from '../model/build-receive-qr-value'

export type ReceiveAddressRowProps = {
	address: string
	isLoading?: boolean
}

/**
 * Receive address card (PRD §4.3.4.1): shows the first unused address as text
 * AND a QR code. Clicking the address text or the QR copies the address to the
 * clipboard (§4.3.4.1.1), with shared "Copied!" feedback announced to assistive
 * tech.
 */
export function ReceiveAddressRow({ address, isLoading }: ReceiveAddressRowProps) {
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
					{copied ? 'Copied!' : 'Tap to copy'}
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
			</div>
		</div>
	)
}
