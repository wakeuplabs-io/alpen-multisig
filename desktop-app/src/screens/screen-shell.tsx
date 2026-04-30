import type { ReactNode } from 'react'

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

function AlpenMark() {
	return (
		<svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
			<path d="M4.5 19.5L10.1 4.5h3.8l-5.5 15H4.5Z" fill="#0A0A0A" />
			<path d="M12.2 19.5l3.7-10h3.8l-3.8 10h-3.7Z" fill="#0A0A0A" />
		</svg>
	)
}
