import type { ReactNode } from 'react'

export function Label({ children, className = '' }: { children: ReactNode; className?: string }) {
	return <span className={`text-[0.78rem] text-[#888] ${className}`}>{children}</span>
}

export function Mono({ children }: { children: ReactNode }) {
	return <span className="break-all font-mono text-[0.85rem]">{children}</span>
}
