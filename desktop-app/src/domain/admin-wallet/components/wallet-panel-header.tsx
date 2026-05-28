type WalletPanelHeaderProps = {
	onClose(): void
	title?: string
}

export function WalletPanelHeader({ onClose, title = 'Admin Wallet' }: WalletPanelHeaderProps) {
	return (
		<div className="flex items-center justify-between border-b border-[#e5e7eb] px-5 py-4">
			<h2 id="wallet-panel-title" className="m-0 text-[15px] font-semibold text-[#111827]">
				{title}
			</h2>
			<button
				type="button"
				aria-label="Close wallet panel"
				onClick={onClose}
				className="rounded-md p-1.5 text-[#6b7280] transition hover:bg-[#f3f4f6] hover:text-[#111827]"
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="16"
					height="16"
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
