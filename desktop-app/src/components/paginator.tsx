type Props = {
	page: number
	totalPages: number
	onPageChange: (page: number) => void
	className?: string
}

export function Paginator({ page, totalPages, onPageChange, className }: Props) {
	if (totalPages <= 1) return null

	return (
		<div className={className ?? 'mt-1 flex items-center justify-center gap-1.5'}>
			<PaginatorButton label="First page" disabled={page === 1} onClick={() => onPageChange(1)}>
				«
			</PaginatorButton>
			<PaginatorButton label="Previous page" disabled={page === 1} onClick={() => onPageChange(page - 1)}>
				‹
			</PaginatorButton>
			<span className="min-w-8 rounded border border-[#d1d5db] bg-white px-2 py-0.5 text-center text-label font-medium text-[#374151]">
				{page}
			</span>
			<PaginatorButton label="Next page" disabled={page === totalPages} onClick={() => onPageChange(page + 1)}>
				›
			</PaginatorButton>
			<PaginatorButton label="Last page" disabled={page === totalPages} onClick={() => onPageChange(totalPages)}>
				»
			</PaginatorButton>
			<span className="ml-1 text-mono-sm text-[#9ca3af]">
				Page {page} of {totalPages}
			</span>
		</div>
	)
}

function PaginatorButton({
	label,
	disabled,
	onClick,
	children,
}: {
	label: string
	disabled: boolean
	onClick: () => void
	children: React.ReactNode
}) {
	return (
		<button
			type="button"
			aria-label={label}
			disabled={disabled}
			onClick={onClick}
			className="flex h-6 w-6 items-center justify-center rounded border border-[#e5e7eb] bg-white text-label text-[#6b7280] transition hover:border-[#d1d5db] hover:bg-[#f9fafb] hover:text-[#374151] disabled:cursor-not-allowed disabled:opacity-40"
		>
			{children}
		</button>
	)
}
