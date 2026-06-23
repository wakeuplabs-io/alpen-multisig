import { useMemo } from 'react'
import { useLocation } from 'react-router-dom'

export type Breadcrumb = {
	label: string
	path: string | null
}

const SEGMENT_LABELS: Record<string, string> = {
	proposals: 'Proposals',
	create: 'Create',
	sign: 'Sign',
	broadcast: 'Send',
	cancel: 'Cancel',
	manual: 'Manual',
	'block-payouts': 'Block Payouts',
}

function truncateId(id: string): string {
	return id.length > 8 ? `#${id.slice(0, 6)}…` : `#${id}`
}

export function useBreadcrumbs(): Breadcrumb[] {
	const { pathname } = useLocation()

	return useMemo(() => {
		const segments = pathname.split('/').filter(Boolean)
		if (segments.length === 0) return []

		const crumbs: Breadcrumb[] = []

		for (let i = 0; i < segments.length; i++) {
			const segment = segments[i]
			const path = '/' + segments.slice(0, i + 1).join('/')
			const isLast = i === segments.length - 1
			const label = SEGMENT_LABELS[segment] ?? truncateId(segment)

			crumbs.push({
				label,
				path: isLast ? null : path,
			})
		}

		return crumbs
	}, [pathname])
}
