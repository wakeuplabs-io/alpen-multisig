import {
	ClockSessionDefaultIcon,
	ClockSessionWarningIcon,
	UsbSessionDefaultIcon,
	UsbSessionWarningIcon,
} from '@/assets/icons'

export type SessionChipProps = {
	timeLabel: string
	signerLabel: string
	warning: boolean
	/** When provided, the chip renders as a button that opens the wallet panel. */
	onActivate?: () => void
	isActive?: boolean
	panelId?: string
}

export function SessionChip({ timeLabel, signerLabel, warning, onActivate, isActive, panelId }: SessionChipProps) {
	const baseClasses =
		'inline-flex items-center gap-2 rounded-full border px-3 py-1.25 text-label whitespace-nowrap flex-none transition'
	const palette = warning
		? 'border-accent-border bg-highlight-surface text-emphasis-soft'
		: 'border-[#e5e7eb] bg-bg-base text-[#111827]'
	const interactiveClasses = onActivate
		? `${isActive ? 'ring-2 ring-accent ring-offset-1' : 'hover:border-[#a3a3a3] hover:bg-bg-surface'} cursor-pointer`
		: ''

	const content = (
		<>
			{warning ? (
				<ClockSessionWarningIcon width={12} height={12} className="block shrink-0" />
			) : (
				<ClockSessionDefaultIcon width={12} height={12} className="block shrink-0" />
			)}
			<span className="font-mono text-mono-sm font-medium">Session · {timeLabel}</span>
			<span className="h-3 w-px bg-[#e5e7eb]" aria-hidden="true" />
			{warning ? (
				<UsbSessionWarningIcon width={12} height={12} className="block shrink-0" />
			) : (
				<UsbSessionDefaultIcon width={12} height={12} className="block shrink-0" />
			)}
			<span className="font-mono text-mono-sm text-[#6b7280]">{signerLabel}</span>
		</>
	)

	if (onActivate) {
		return (
			<button
				type="button"
				onClick={onActivate}
				aria-expanded={isActive ?? false}
				aria-haspopup="dialog"
				aria-controls={panelId}
				className={`${baseClasses} ${palette} ${interactiveClasses}`}
				data-testid="e2e-session-chip-trigger"
			>
				{content}
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
					aria-hidden="true"
					className={`ml-0.5 block flex-none text-[#9ca3af] transition-transform duration-150 ${isActive ? 'rotate-180' : ''}`}
				>
					<polyline points="6 9 12 15 18 9" />
				</svg>
			</button>
		)
	}

	return <span className={`${baseClasses} ${palette}`}>{content}</span>
}
