import { useState } from 'react'

export function ConflictingInputIcon() {
	const [visible, setVisible] = useState(false)

	return (
		<span className="relative inline-flex items-center">
			<button
				type="button"
				className="inline-flex items-center justify-center rounded-full text-[#d97706] transition hover:text-[#b45309] focus:outline-none"
				onMouseEnter={() => setVisible(true)}
				onMouseLeave={() => setVisible(false)}
				onFocus={() => setVisible(true)}
				onBlur={() => setVisible(false)}
				aria-label="This input is included in multiple Pending transactions."
			>
				<svg width={14} height={14} viewBox="0 0 24 24" fill="none" aria-hidden>
					<circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.5" />
					<path d="M12 8v4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
					<circle cx="12" cy="16" r="1" fill="currentColor" />
				</svg>
			</button>
			{visible && (
				<span
					className="absolute bottom-full left-1/2 z-50 mb-2 w-56 -translate-x-1/2 rounded-lg border border-[#fde68a] bg-[#fffbeb] px-3 py-2 text-[12px] leading-[1.5] text-[#92400e] shadow-sm"
					role="tooltip"
				>
					This input is included in multiple Pending transactions.
					<span
						className="absolute left-1/2 top-full -translate-x-1/2 border-4 border-transparent border-t-[#fde68a]"
						aria-hidden="true"
					/>
				</span>
			)}
		</span>
	)
}
