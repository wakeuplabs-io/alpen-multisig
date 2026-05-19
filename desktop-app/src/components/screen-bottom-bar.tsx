import type { ReactNode } from 'react'

type Props = {
	left?: ReactNode
	actions: ReactNode
}

export function ScreenBottomBar({ left, actions }: Props) {
	return (
		<div className="fixed inset-x-0 bottom-0 z-20 border-t border-[#e5e7eb] bg-white/95 px-4 py-3 backdrop-blur-sm">
			<div className="mx-auto flex w-full max-w-150 items-center justify-between gap-4">
				{left ? <div className="min-w-0 flex-1">{left}</div> : <div />}
				<div className="flex shrink-0 items-center gap-2">{actions}</div>
			</div>
		</div>
	)
}
