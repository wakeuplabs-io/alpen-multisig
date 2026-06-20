import { useNavigate } from 'react-router-dom'
import { useBreadcrumbs, type Breadcrumb } from '@/hooks/use-breadcrumbs'

function ChevronSeparator() {
	return (
		<span className="text-[#9ca3af]" aria-hidden="true">
			›
		</span>
	)
}

function BreadcrumbItem({ crumb }: { crumb: Breadcrumb }) {
	const navigate = useNavigate()

	if (crumb.path === null) {
		return <span className="text-body-sm font-medium text-[#111827]">{crumb.label}</span>
	}

	return (
		<button
			type="button"
			className="text-body-sm text-[#6b7280] transition hover:text-[#111827]"
			onClick={() => navigate(crumb.path!)}
		>
			{crumb.label}
		</button>
	)
}

export function Breadcrumbs() {
	const crumbs = useBreadcrumbs()

	if (crumbs.length <= 1) return null

	return (
		<nav aria-label="Breadcrumb" className="flex items-center gap-1.5">
			{crumbs.map((crumb, i) => (
				<span key={crumb.label + i} className="flex items-center gap-1.5">
					{i > 0 && <ChevronSeparator />}
					<BreadcrumbItem crumb={crumb} />
				</span>
			))}
		</nav>
	)
}
