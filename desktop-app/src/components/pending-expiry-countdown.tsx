import { useEffect, useState } from 'react'
import { expiryUrgency, formatExpiryTimeLeft } from '@/lib/proposal-expiry'

type Props = {
	createdAtMs: number
	/** From the backend that enforces the window; `null` from one that predates the field. */
	expiresAtMs: number | null
}

export function PendingExpiryCountdown({ createdAtMs, expiresAtMs }: Props) {
	const [nowMs, setNowMs] = useState(() => Date.now())

	useEffect(() => {
		setNowMs(Date.now())
		const id = setInterval(() => {
			setNowMs(Date.now())
		}, 60_000)
		return () => clearInterval(id)
	}, [expiresAtMs])

	// No countdown is better than one computed against a window we cannot know (#551).
	if (expiresAtMs === null) return null

	const timeLeftMs = expiresAtMs - nowMs

	if (timeLeftMs <= 0) {
		return (
			<span className="inline-flex items-center gap-1 text-label font-medium text-emphasis-soft">
				<span aria-hidden="true">⏱</span>
				Expired
			</span>
		)
	}

	const urgency = expiryUrgency(timeLeftMs, expiresAtMs - createdAtMs)

	// Running out of time is a status, not a failure (#416): urgency is carried by
	// the ⚠ and the wording, and shown as a darker neutral rather than red.
	const label =
		urgency === 'none' ? formatExpiryTimeLeft(timeLeftMs) : `⚠ Expiring soon — ${formatExpiryTimeLeft(timeLeftMs)}`

	return (
		<span
			className={`inline-flex items-center gap-1 text-label font-medium ${urgency === 'urgent' ? 'text-emphasis' : 'text-emphasis-soft'}`}
		>
			<span aria-hidden="true">⏱</span>
			{label}
		</span>
	)
}
