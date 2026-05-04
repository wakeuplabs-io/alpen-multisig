import type { ReactNode } from 'react'
import { AlpenMark } from '@/assets/icons'

type Props = {
	children: ReactNode
	headerContent?: ReactNode
	centerContent?: boolean
}

/** Shared centered layout for signer-facing screens. */
export function ScreenShell({ children, headerContent, centerContent = false }: Props) {
	return (
		<div className="min-h-screen bg-[#f8f8fb]">
			<header className="flex h-[60px] items-center justify-between border-b border-[#e5e7eb] bg-white px-8">
				<div className="inline-flex items-center gap-1.5 text-[#0a0a0a]">
					<AlpenMark />
					<span className="text-lg font-medium">Alpen</span>
				</div>
				{headerContent ? <div className="flex items-center gap-2">{headerContent}</div> : null}
			</header>
			<main
				className={`flex min-h-[calc(100vh-60px)] justify-center px-8 py-8 ${
					centerContent ? 'items-center' : 'items-start'
				}`}
			>
				<div className="flex w-full max-w-[1360px] flex-col gap-5">{children}</div>
			</main>
		</div>
	)
}
