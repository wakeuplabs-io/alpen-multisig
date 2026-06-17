export function LabelWithTooltip({ label, tooltip }: { label: string; tooltip: string }) {
	return (
		<div className="flex items-center gap-1.5">
			<span className="text-body font-medium text-[#111827]">{label}</span>
			<span
				className="flex h-4 w-4 shrink-0 cursor-help items-center justify-center rounded-full border border-[#d1d5db] text-[10px] text-[#9ca3af]"
				title={tooltip}
			>
				?
			</span>
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
					? 'border-[#d97706] bg-[#fffbeb]'
					: 'border-[#e5e7eb] bg-white hover:border-[#fbbf24] hover:bg-[#fffdf5]'
			}`}
		>
			<p className={`m-0 text-body font-semibold ${selected ? 'text-[#92400e]' : 'text-[#111827]'}`}>{title}</p>
			<p className={`m-0 mt-1 text-label ${selected ? 'text-[#b45309]' : 'text-[#6b7280]'}`}>{description}</p>
		</button>
	)
}
