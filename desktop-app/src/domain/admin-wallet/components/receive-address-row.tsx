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

	return (
		<div className="flex items-center gap-2">
			<p className="flex-1 truncate font-mono text-[12px] text-[#374151]">{address}</p>
			<CopyButton text={address} />
		</div>
	)
}
