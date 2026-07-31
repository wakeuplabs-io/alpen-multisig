import type { ButtonHTMLAttributes, ReactNode } from 'react'

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'destructive'
type ButtonSize = 'sm' | 'md' | 'lg'

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
	variant?: ButtonVariant
	size?: ButtonSize
	loading?: boolean
	children: ReactNode
}

const BASE =
	'inline-flex items-center justify-center gap-1.5 font-medium transition-all active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-40 disabled:active:scale-100'

const VARIANT: Record<ButtonVariant, string> = {
	primary: 'rounded-xl border border-[#111827] bg-[#111827] text-white hover:bg-[#2a2a2a]',
	secondary:
		'rounded-xl border border-[#111827] bg-transparent text-[#0a0a0a] hover:bg-bg-surface hover:border-accent-border',
	ghost: 'rounded-lg bg-transparent font-normal text-accent hover:text-accent-hover hover:underline',
	destructive: 'rounded-xl border border-danger bg-danger text-white hover:bg-danger-strong',
}

const SIZE: Record<ButtonSize, string> = {
	sm: 'px-3.5 py-1.5 text-body-sm',
	md: 'px-5 py-2.5 text-body',
	lg: 'px-6 py-3 text-body-lg',
}

export function Button({
	variant = 'primary',
	size = 'md',
	loading = false,
	disabled,
	className = '',
	children,
	...rest
}: Props) {
	return (
		<button
			type="button"
			disabled={disabled || loading}
			className={`${BASE} ${VARIANT[variant]} ${SIZE[size]} ${className}`}
			{...rest}
		>
			{loading && <Spinner variant={variant} />}
			{children}
		</button>
	)
}

function Spinner({ variant }: { variant: ButtonVariant }) {
	const colorClass =
		variant === 'primary' || variant === 'destructive'
			? 'border-white/30 border-t-white'
			: 'border-[#0a0a0a]/15 border-t-[#0a0a0a]'

	return (
		<span className={`block size-3.5 shrink-0 animate-spin rounded-full border-2 ${colorClass}`} aria-hidden="true" />
	)
}
