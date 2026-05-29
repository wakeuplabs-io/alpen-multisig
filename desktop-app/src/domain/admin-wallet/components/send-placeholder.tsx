export function SendPlaceholder() {
	return (
		<div>
			<button
				type="button"
				disabled
				className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
			>
				Send
			</button>
			<p className="mt-2 text-[13px] text-[#6b7280]">Send is not available yet.</p>
		</div>
	)
}
