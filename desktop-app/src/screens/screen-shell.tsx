import type { ReactNode } from 'react'
import strataIcon from '@/assets/strata-icon.png'

type Props = {
	children: ReactNode
	headerContent?: ReactNode
	authorityBadge?: ReactNode
	centerContent?: boolean
}

/** Shared centered layout for signer-facing screens. */
export function ScreenShell({ children, headerContent, authorityBadge, centerContent = false }: Props) {
	return (
		<div className="min-h-screen bg-bg-base">
			<header className="flex h-[60px] items-center justify-between border-b border-[#e5e7eb] bg-white px-8">
				<div className="inline-flex items-center gap-3 text-[#0a0a0a]">
					<img src={strataIcon} alt="Strata" className="h-5 w-auto" />
					<span className="text-body-lg font-semibold tracking-[0.04em]">STRATA</span>
					{authorityBadge}
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
			<div className="pointer-events-none fixed bottom-2 right-3 text-mono-sm text-[#9ca3af]">v{__APP_VERSION__}</div>
		</div>
	)
}
