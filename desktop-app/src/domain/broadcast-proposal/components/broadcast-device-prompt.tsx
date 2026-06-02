export function BroadcastDevicePrompt() {
	return (
		<div className="rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-4 py-3">
			<div className="flex items-center gap-3">
				<svg
					className="h-6 w-6 shrink-0 text-[#6b7280]"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					strokeWidth={1.5}
					aria-hidden="true"
				>
					<rect x="5" y="2" width="14" height="20" rx="2" ry="2" />
					<line x1="12" y1="18" x2="12" y2="18.01" />
				</svg>
				<div>
					<p className="m-0 text-[13px] font-medium text-[#111827]">Confirm on your device</p>
					<p className="m-0 mt-1 text-[12px] text-[#6b7280]">
						Broadcast uses two Ledger steps (not the same as login “Sign message”): first register the Admin Wallet
						policy, then review the commit transaction. Press <strong>right</strong> on every screen until the app
						finishes.
					</p>
				</div>
			</div>
		</div>
	)
}
