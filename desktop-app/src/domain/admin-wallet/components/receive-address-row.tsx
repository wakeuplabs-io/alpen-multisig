import { CopyButton } from '@/components/copy-button'

export type ReceiveAddressRowProps = {
	address: string
	isLoading?: boolean
}

export function ReceiveAddressRow({ address, isLoading }: ReceiveAddressRowProps) {
	if (isLoading) {
		return (
			<div className="flex items-center gap-2">
				<div className="h-4 flex-1 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="h-7 w-14 animate-pulse rounded bg-[#e5e7eb]" />
			</div>
		)
	}

	if (!address) return null

	return (
		<div className="flex min-w-0 flex-col gap-1">
			<span className="text-[10px] font-semibold uppercase tracking-[0.06em] text-[#9ca3af]">Receive address</span>
			<div className="flex min-w-0 items-center gap-2">
				<span className="min-w-0 flex-1 truncate font-mono text-[12px] leading-[1.45] text-[#374151]" title={address}>
					{address}
				</span>
				<CopyButton text={address} />
			</div>
		</div>
	)
}
