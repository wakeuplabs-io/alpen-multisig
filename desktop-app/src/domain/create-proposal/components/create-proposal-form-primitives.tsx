import { useState, useRef, useEffect } from 'react'

export function LabelWithTooltip({ label, tooltip }: { label: string; tooltip: string }) {
	const [open, setOpen] = useState(false)
	const ref = useRef<HTMLDivElement>(null)

	useEffect(() => {
		if (!open) return
		function handleClickOutside(e: MouseEvent) {
			if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
		}
		document.addEventListener('mousedown', handleClickOutside)
		return () => document.removeEventListener('mousedown', handleClickOutside)
	}, [open])

	return (
		<div className="flex items-center gap-1.5">
			<span className="text-body font-medium text-[#111827]">{label}</span>
			<div ref={ref} className="relative">
				<button
					type="button"
					className="flex h-4 w-4 shrink-0 cursor-help items-center justify-center rounded-full border border-[#d1d5db] text-[10px] text-[#9ca3af] hover:border-[#9ca3af] hover:text-[#6b7280]"
					onClick={() => setOpen((v) => !v)}
					aria-label={`Info: ${label}`}
				>
					?
				</button>
				{open && (
					<div className="absolute left-1/2 top-full z-50 mt-1.5 w-64 -translate-x-1/2 rounded-lg border border-[#e5e7eb] bg-white p-2.5 text-label text-[#374151] shadow-md">
						{tooltip}
					</div>
				)}
			</div>
		</div>
	)
}

export function ActionTypeCard({
	title,
	description,
	selected,
	onClick,
}: {
	title: string
	description: string
	selected: boolean
	onClick: () => void
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={`rounded-xl border-2 p-4 text-left transition-colors ${
				selected
					? 'border-accent-border bg-highlight-surface'
					: 'border-[#e5e7eb] bg-white hover:border-accent-border hover:bg-highlight-surface/40'
			}`}
		>
			<p className="m-0 text-body font-semibold text-emphasis">{title}</p>
			<p className="m-0 mt-1 text-label text-emphasis-soft">{description}</p>
		</button>
	)
}
