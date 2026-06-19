import { useCallback, useEffect, useState } from 'react'
import { useSession } from '@/hooks/use-session'

const WARNING_THRESHOLD_MS = 2 * 60 * 1000

export function SessionExpiryModal() {
	const { remainingMs, sessionTimeLabel, wallet, ensureOrchestratorSession } = useSession()
	const [dismissed, setDismissed] = useState(false)
	const [isExtending, setIsExtending] = useState(false)
	const [extendError, setExtendError] = useState<string | null>(null)

	const isActive = wallet !== null
	const shouldShow = isActive && remainingMs > 0 && remainingMs <= WARNING_THRESHOLD_MS && !dismissed

	useEffect(() => {
		if (remainingMs > WARNING_THRESHOLD_MS) {
			setDismissed(false)
			setExtendError(null)
		}
	}, [remainingMs])

	const handleExtend = useCallback(async () => {
		setIsExtending(true)
		setExtendError(null)
		try {
			await ensureOrchestratorSession()
			setDismissed(true)
		} catch (e) {
			setExtendError(String(e))
		} finally {
			setIsExtending(false)
		}
	}, [ensureOrchestratorSession])

	if (!shouldShow) return null

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
			<div className="w-full max-w-md rounded-xl border border-[#fde68a] bg-white p-6 shadow-lg">
				<h2 className="m-0 text-body-lg font-semibold text-[#111827]">Session expiring soon</h2>
				<p className="m-0 mt-2 text-body-sm text-[#6b7280]">
					Your session expires in <span className="font-semibold text-[#d97706]">{sessionTimeLabel}</span>. Extend it to
					avoid losing your current work.
				</p>

				{extendError && (
					<div className="mt-3 rounded-lg border border-[#fecaca] bg-[#fef2f2] px-3 py-2">
						<p className="m-0 text-label text-[#991b1b]">{extendError}</p>
					</div>
				)}

				<div className="mt-5 flex justify-end gap-3">
					<button
						type="button"
						className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
						onClick={() => setDismissed(true)}
					>
						Dismiss
					</button>
					<button
						type="button"
						disabled={isExtending}
						className="rounded-lg border border-[#111827] bg-[#111827] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-black disabled:opacity-50"
						onClick={() => void handleExtend()}
					>
						{isExtending ? 'Extending...' : 'Extend session'}
					</button>
				</div>
			</div>
		</div>
	)
}
