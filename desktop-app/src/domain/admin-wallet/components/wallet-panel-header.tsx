type WalletPanelHeaderProps = {
	onClose(): void
	/** Primary title. Defaults to "Admin Wallet". */
	title?: string
	/** Secondary line shown under the title (e.g. session and signer context). */
	subtitle?: string
	/** When true, shows a subtle watch-only badge (HW session without signing). */
	isWatchOnly?: boolean
}

export function WalletPanelHeader({
	onClose,
	title = 'Admin Wallet',
	subtitle,
	isWatchOnly = false,
}: WalletPanelHeaderProps) {
	return (
		<div className="flex items-center justify-between gap-3 border-b border-[#e5e7eb] px-[18px] py-4">
			<div className="min-w-0 flex-1">
				<div className="flex min-w-0 items-center gap-2">
					<h2
						id="wallet-panel-title"
						className="m-0 truncate font-mono text-[14px] font-medium tracking-[0.02em] text-[#111827]"
					>
						{title}
					</h2>
					{isWatchOnly && (
						<span className="shrink-0 rounded-md border border-[#e5e7eb] bg-[#f9fafb] px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-[#6b7280]">
							Watch-only
						</span>
					)}
				</div>
				{subtitle && <p className="m-0 mt-0.5 truncate font-mono text-[11px] text-[#6b7280]">{subtitle}</p>}
			</div>
			<button
				type="button"
				aria-label="Close wallet panel"
				onClick={onClose}
				className="rounded-md p-1.5 text-[#6b7280] transition hover:bg-[#f3f4f6] hover:text-[#111827]"
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
					aria-hidden="true"
				>
					<line x1="18" y1="6" x2="6" y2="18" />
					<line x1="6" y1="6" x2="18" y2="18" />
				</svg>
			</button>
		</div>
	)
}
