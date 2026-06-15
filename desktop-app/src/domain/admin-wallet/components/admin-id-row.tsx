import { CopyButton } from '@/components/copy-button'

export type AdminIdRowProps = {
	address: string | undefined
}

export function AdminIdRow({ address }: AdminIdRowProps) {
	if (!address) {
		return null
	}

	return (
		<div className="rounded-xl border border-[#f3f4f6] px-4 py-3" data-testid="e2e-admin-id-row">
			<span className="text-[11px] font-medium uppercase tracking-[0.08em] text-[#9ca3af]">Admin ID</span>
			<div className="mt-1.5 flex items-center justify-between gap-2">
				<span
					className="min-w-0 flex-1 break-all font-mono text-[12px] leading-[1.45] text-[#374151]"
					title={address}
					data-testid="e2e-admin-id-value"
				>
					{address}
				</span>
				<CopyButton text={address} variant="icon" />
			</div>
		</div>
	)
}
