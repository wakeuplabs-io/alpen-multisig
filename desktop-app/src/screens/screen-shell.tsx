import type { ReactNode } from 'react'

type Props = {
	children: ReactNode
}

/** Shared centered layout for signer-facing screens. */
export function ScreenShell({ children }: Props) {
	return (
		<div className="min-h-screen bg-[#f8f8fb]">
			<header className="flex h-14 items-center border-b border-[#e5e7eb] bg-white px-4">
				<div className="inline-flex items-center gap-1.5 text-[#0a0a0a]">
					<AlpenMark />
					<span className="text-lg font-medium">Alpen</span>
				</div>
			</header>
			<main className="flex min-h-screen justify-center px-4 py-12">
				<div className="flex w-full max-w-136 flex-col items-center gap-5">{children}</div>
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
