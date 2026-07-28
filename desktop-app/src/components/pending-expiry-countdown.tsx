import { useEffect, useState } from 'react'

type Props = {
	expiresAtMs: number
}

function formatTimeLeft(ms: number): string {
	if (ms <= 0) return 'Expired'
	const totalSeconds = Math.floor(ms / 1000)
	const days = Math.floor(totalSeconds / 86400)
	const hours = Math.floor((totalSeconds % 86400) / 3600)
	const minutes = Math.floor((totalSeconds % 3600) / 60)
	if (days > 0) return `Expires in ${days} d ${hours} h`
	if (hours > 0) return `Expires in ${hours} h ${minutes} m`
	return `Expires in ${minutes} m`
}

export function PendingExpiryCountdown({ expiresAtMs }: Props) {
	const [timeLeftMs, setTimeLeftMs] = useState(() => expiresAtMs - Date.now())

	useEffect(() => {
		setTimeLeftMs(expiresAtMs - Date.now())
		const id = setInterval(() => {
			setTimeLeftMs(expiresAtMs - Date.now())
		}, 60_000)
		return () => clearInterval(id)
	}, [expiresAtMs])

	if (timeLeftMs <= 0) {
		return (
			<span className="inline-flex items-center gap-1 text-label font-medium text-emphasis-soft">
				<span aria-hidden="true">⏱</span>
				Expired
			</span>
		)
	}

	const isUrgent = timeLeftMs < 60 * 60 * 1000
	const isWarning = timeLeftMs < 24 * 60 * 60 * 1000

	// Running out of time is a status, not a failure (#416): urgency is carried by
	// the ⚠ and the wording, and shown as a darker neutral rather than red.
	const label = isWarning ? `⚠ Expiring soon — ${formatTimeLeft(timeLeftMs)}` : formatTimeLeft(timeLeftMs)

	return (
		<span
			className={`inline-flex items-center gap-1 text-label font-medium ${isUrgent ? 'text-emphasis' : 'text-emphasis-soft'}`}
		>
			<span aria-hidden="true">⏱</span>
			{label}
		</span>
	)
}
