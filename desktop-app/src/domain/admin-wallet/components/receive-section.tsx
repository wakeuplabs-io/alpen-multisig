import { useState } from 'react'

export type ReceiveSectionProps = {
	address: string | null
	isLoading?: boolean
}

export function ReceiveSection({ address }: ReceiveSectionProps) {
	const [expanded, setExpanded] = useState(false)
	const disabled = !address

	return (
		<div className="flex flex-col gap-2.5">
			<button
				type="button"
				onClick={() => setExpanded((prev) => !prev)}
				className="inline-flex w-fit items-center gap-1.5 rounded-md border border-[#e5e7eb] bg-white px-3 py-1.5 text-[12px] font-medium text-[#374151] transition hover:border-[#d1d5db] hover:bg-[#f9fafb] disabled:cursor-not-allowed disabled:opacity-60"
				aria-expanded={expanded}
				disabled={disabled}
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
					aria-hidden="true"
				>
					{expanded ? (
						<polyline points="18 15 12 9 6 15" />
					) : (
						<>
							<rect x="2" y="5" width="20" height="14" rx="2" />
							<path d="M16 12h.01" />
						</>
					)}
				</svg>
				{expanded ? 'Hide QR' : 'Receive'}
			</button>

			{expanded && (
				<div className="flex flex-col items-center gap-2.5 rounded-[10px] border border-[#e5e7eb] bg-[#f9fafb] p-3">
					<div className="flex h-[200px] w-full items-center justify-center rounded bg-white text-[12px] text-[#9ca3af]">
						QR preview unavailable.
					</div>
					<p className="m-0 text-[11px] text-[#9ca3af]">QR rendering arrives in Phase 6 (receive rotation).</p>
				</div>
			)}
		</div>
	)
}
