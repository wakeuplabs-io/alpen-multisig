export function LabelWithTooltip({ label, tooltip }: { label: string; tooltip: string }) {
	return (
		<div className="flex items-center gap-1.5">
			<span className="text-sm font-medium text-[#111827]">{label}</span>
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
					? 'border-[#5b44c9] bg-[#f5f3ff]'
					: 'border-[#e5e7eb] bg-white hover:border-[#c4b5fd] hover:bg-[#faf9ff]'
			}`}
		>
			<p className={`m-0 text-sm font-semibold ${selected ? 'text-[#3730a3]' : 'text-[#111827]'}`}>{title}</p>
			<p className={`m-0 mt-1 text-xs ${selected ? 'text-[#5b44c9]' : 'text-[#6b7280]'}`}>{description}</p>
		</button>
	)
}
