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
		<div className="flex items-center justify-between gap-3 border-b border-[#e5e7eb] px-5 py-4">
			<div className="min-w-0 flex-1">
				<div className="flex min-w-0 items-center gap-2">
					<h2
						id="wallet-panel-title"
						className="m-0 truncate font-sans text-[15px] font-medium tracking-[0.02em] text-[#111827]"
					>
						{title}
					</h2>
					{isWatchOnly && (
						<span className="inline-flex shrink-0 items-center gap-1 rounded-full bg-[#f3f4f6] px-2 py-0.5 text-[11px] text-[#9ca3af]">
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
								<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
								<circle cx="12" cy="12" r="3" />
							</svg>
							Watch-only
						</span>
					)}
				</div>
				{subtitle && <p className="m-0 mt-0.5 truncate font-mono text-[11px] text-[#4b5563]">{subtitle}</p>}
			</div>
			<button
				type="button"
				aria-label="Close wallet panel"
				onClick={onClose}
				className="rounded-lg p-2 text-[#6b7280] transition hover:bg-[#f3f4f6] hover:text-[#111827]"
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
