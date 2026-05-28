import { useState } from 'react'
import { SectionLabel } from '@/components/section-label'
import { ReceiveAddressRow } from './receive-address-row'

export type ReceiveSectionProps = {
	address: string | null
	isLoading?: boolean
}

export function ReceiveSection({ address, isLoading }: ReceiveSectionProps) {
	const [expanded, setExpanded] = useState(false)

	return (
		<div>
			<button
				type="button"
				onClick={() => setExpanded((prev) => !prev)}
				className="flex items-center gap-1 text-[13px] font-medium text-[#374151] transition hover:text-[#111827]"
				aria-expanded={expanded}
			>
				Receive
				<span aria-hidden="true">{expanded ? '▴' : '▾'}</span>
			</button>

			{expanded && (
				<div className="mt-3 space-y-2">
					<SectionLabel>Deposit address</SectionLabel>
					<ReceiveAddressRow address={address ?? ''} isLoading={isLoading ?? address === null} />
					<p className="text-[11px] text-[#9ca3af]">QR rendering arrives in Phase 6 (receive rotation).</p>
				</div>
			)}
		</div>
	)
}
