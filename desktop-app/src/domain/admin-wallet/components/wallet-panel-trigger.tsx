type WalletPanelTriggerProps = {
	isOpen: boolean
	onToggle(): void
}

export function WalletPanelTrigger({ isOpen, onToggle }: WalletPanelTriggerProps) {
	return (
		<button
			type="button"
			aria-pressed={isOpen}
			aria-label="Toggle wallet panel"
			onClick={onToggle}
			className={[
				'inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-[13px] font-medium transition',
				isOpen ? 'bg-[#111827] text-white' : 'border border-[#e5e7eb] bg-white text-[#374151] hover:bg-[#f9fafb]',
			].join(' ')}
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
				<rect x="2" y="5" width="20" height="14" rx="2" />
				<path d="M16 12h.01" />
			</svg>
			Wallet
		</button>
	)
}
