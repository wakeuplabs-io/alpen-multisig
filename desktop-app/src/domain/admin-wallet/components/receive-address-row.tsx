import { CopyButton } from '@/components/copy-button'

export type ReceiveAddressRowProps = {
	address: string
	isLoading?: boolean
}

export function ReceiveAddressRow({ address, isLoading }: ReceiveAddressRowProps) {
	if (isLoading) {
		return (
			<div className="rounded-xl border border-[#f3f4f6] px-4 py-3">
				<div className="mb-2 h-3 w-24 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="flex items-center justify-between gap-2">
					<div className="h-4 flex-1 animate-pulse rounded bg-[#e5e7eb]" />
					<div className="h-7 w-14 animate-pulse rounded bg-[#e5e7eb]" />
				</div>
			</div>
		)
	}

	if (!address) {
		return (
			<div className="rounded-xl border border-[#f3f4f6] px-4 py-3">
				<span className="text-[11px] font-medium uppercase tracking-[0.08em] text-[#9ca3af]">Receive address</span>
				<p className="mt-1.5 text-[12px] text-[#9ca3af]">No receive address yet</p>
			</div>
		)
	}

	return (
		<div className="rounded-xl border border-[#f3f4f6] px-4 py-3" data-testid="e2e-wallet-receive-address-row">
			<span className="text-[11px] font-medium uppercase tracking-[0.08em] text-[#9ca3af]">Receive address</span>
			<div className="mt-1.5 flex items-center justify-between gap-2">
				<span
					className="min-w-0 flex-1 truncate font-mono text-[12px] leading-[1.45] text-[#374151]"
					title={address}
					data-testid="e2e-wallet-receive-address-value"
				>
					{address}
				</span>
				<CopyButton text={address} variant="icon" />
			</div>
		</div>
	)
}
