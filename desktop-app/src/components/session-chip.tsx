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
		'inline-flex items-center gap-2 rounded-full border px-3 py-1.25 text-[12px] whitespace-nowrap flex-none transition'
	const palette = warning
		? 'border-[#fde68a] bg-[#fffbeb] text-[#d97706]'
		: 'border-[#e5e7eb] bg-[#f8f8fb] text-[#111827]'
	const interactiveClasses = onActivate
		? `${isActive ? 'ring-2 ring-[#9480f5] ring-offset-1' : 'hover:border-[#a3a3a3] hover:bg-[#f1f1f5]'} cursor-pointer`
		: ''

	const content = (
		<>
			{warning ? (
				<ClockSessionWarningIcon width={12} height={12} className="block shrink-0" />
			) : (
				<ClockSessionDefaultIcon width={12} height={12} className="block shrink-0" />
			)}
			<span className="font-mono text-[11px] font-medium">Session · {timeLabel}</span>
			<span className="h-3 w-px" style={{ background: warning ? '#fde68a' : '#e5e7eb' }} aria-hidden="true" />
			{warning ? (
				<UsbSessionWarningIcon width={12} height={12} className="block shrink-0" />
			) : (
				<UsbSessionDefaultIcon width={12} height={12} className="block shrink-0" />
			)}
			<span className="font-mono text-[11px]" style={{ color: warning ? '#d97706' : '#6b7280' }}>
				{signerLabel}
			</span>
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
			</button>
		)
	}

	return <span className={`${baseClasses} ${palette}`}>{content}</span>
}
