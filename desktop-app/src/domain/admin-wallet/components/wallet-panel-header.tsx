type WalletPanelHeaderProps = {
	onClose(): void
	/** Optional primary title (e.g. `Session · 12:00`). Defaults to "Admin Wallet". */
	title?: string
	/** Optional secondary line shown under the title (e.g. truncated signer address). */
	subtitle?: string
}

export function WalletPanelHeader({ onClose, title = 'Admin Wallet', subtitle }: WalletPanelHeaderProps) {
	return (
		<div className="flex items-center justify-between gap-3 border-b border-[#e5e7eb] px-[18px] py-4">
			<div className="min-w-0 flex-1">
				<h2
					id="wallet-panel-title"
					className="m-0 truncate font-mono text-[14px] font-medium tracking-[0.02em] text-[#111827]"
				>
					{title}
				</h2>
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
