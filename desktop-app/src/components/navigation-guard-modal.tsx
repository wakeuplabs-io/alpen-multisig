import { useCallback } from 'react'
import type { Blocker } from 'react-router-dom'
import { AccessibleDialog } from '@/components/accessible-dialog'
import { Button } from '@/components/button'

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
	const isOpen = blocker.state === 'blocked'

	const handleStay = useCallback(() => {
		blocker.reset?.()
	}, [blocker])

	return (
		<AccessibleDialog isOpen={isOpen} onClose={handleStay} title={title}>
			<p className="m-0 mt-2 text-body-sm text-[#6b7280]">{message}</p>
			<div className="mt-5 flex justify-end gap-3">
				<button
					type="button"
					className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
					onClick={handleStay}
				>
					Stay
				</button>
				<Button variant="destructive" size="sm" onClick={() => blocker.proceed?.()}>
					Leave
				</Button>
			</div>
		</AccessibleDialog>
	)
}
