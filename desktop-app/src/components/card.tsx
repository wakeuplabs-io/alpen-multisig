import type { ReactNode } from 'react'

type CardVariant = 'default' | 'xl' | 'surface'

type Props = {
	variant?: CardVariant
	className?: string
	children: ReactNode
}

const VARIANT: Record<CardVariant, string> = {
	default: 'rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-[var(--shadow-card)]',
	xl: 'rounded-2xl border border-[#e5e7eb] bg-white p-6 shadow-[var(--shadow-card)]',
	surface: 'rounded-xl border border-accent-border bg-bg-surface p-5',
}

export function Card({ variant = 'default', className = '', children }: Props) {
	return <div className={`${VARIANT[variant]} ${className}`}>{children}</div>
}
