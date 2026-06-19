import type { Blocker } from 'react-router-dom'

type Props = {
	blocker: Blocker
	title?: string
	message?: string
}

export function NavigationGuardModal({
	blocker,
	title = 'Unsaved changes',
	message = 'You have unsaved changes that will be lost if you leave this page. Are you sure you want to leave?',
}: Props) {
	if (blocker.state !== 'blocked') return null

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
			<div className="w-full max-w-md rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-lg">
				<h2 className="m-0 text-body-lg font-semibold text-[#111827]">{title}</h2>
				<p className="m-0 mt-2 text-body-sm text-[#6b7280]">{message}</p>
				<div className="mt-5 flex justify-end gap-3">
					<button
						type="button"
						className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
						onClick={() => blocker.reset?.()}
					>
						Stay
					</button>
					<button
						type="button"
						className="rounded-lg border border-[#dc2626] bg-[#dc2626] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-[#b91c1c]"
						onClick={() => blocker.proceed?.()}
					>
						Leave
					</button>
				</div>
			</div>
		</div>
	)
}
