import { ReactNode, useEffect, useRef, useState } from 'react'

export const WALLET_SLIDE_TRANSITION_MS = 300

/** iOS sheet / drawer easing — feels deliberate on open and close. */
const WALLET_SLIDE_EASING = 'cubic-bezier(0.32, 0.72, 0, 1)'

const FOCUSABLE_SELECTOR = 'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])'

export type WalletPanelProps = {
	isOpen: boolean
	onClose(): void
	panelId?: string
	children: ReactNode
}

export function WalletPanel({ isOpen, onClose, panelId = 'wallet-slide-dialog', children }: WalletPanelProps) {
	const [entered, setEntered] = useState(false)
	const panelRef = useRef<HTMLDivElement>(null)
	const previousFocusRef = useRef<Element | null>(null)

	// Double-RAF to ensure initial transform applies before transition
	useEffect(() => {
		if (isOpen) {
			previousFocusRef.current = document.activeElement
			requestAnimationFrame(() => {
				requestAnimationFrame(() => {
					setEntered(true)
				})
			})
		} else {
			setEntered(false)
			if (previousFocusRef.current instanceof HTMLElement) {
				previousFocusRef.current.focus()
			}
			previousFocusRef.current = null
		}
	}, [isOpen])

	// Focus trap: Tab / Shift+Tab cycling
	useEffect(() => {
		if (!isOpen) return

		const handleKeyDown = (e: KeyboardEvent) => {
			if (e.key === 'Escape') {
				onClose()
				return
			}

			if (e.key !== 'Tab') return

			const panel = panelRef.current
			if (!panel) return

			const focusable = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
				(el) => !el.closest('[aria-hidden="true"]'),
			)

			if (focusable.length === 0) return

			const first = focusable[0]
			const last = focusable[focusable.length - 1]

			if (e.shiftKey) {
				if (document.activeElement === first) {
					e.preventDefault()
					last.focus()
				}
			} else {
				if (document.activeElement === last) {
					e.preventDefault()
					first.focus()
				}
			}
		}

		document.addEventListener('keydown', handleKeyDown)
		return () => document.removeEventListener('keydown', handleKeyDown)
	}, [isOpen, onClose])

	if (!isOpen && !entered) return null

	return (
		<>
			{/* Backdrop */}
			<div
				className={[
					'fixed inset-0 z-40 bg-black/20',
					'transition-opacity ease-in-out duration-200',
					entered ? 'opacity-100' : 'opacity-0',
				].join(' ')}
				onClick={onClose}
				aria-hidden="true"
			/>

			{/* Panel */}
			<div
				id={panelId}
				ref={panelRef}
				role="dialog"
				aria-modal="true"
				aria-labelledby="wallet-panel-title"
				style={{ transitionTimingFunction: WALLET_SLIDE_EASING, transitionDuration: `${WALLET_SLIDE_TRANSITION_MS}ms` }}
				className={[
					'fixed inset-y-0 right-0 z-50 flex w-full max-w-[400px] flex-col border-l border-[#e5e7eb] bg-white',
					'shadow-[-8px_0_32px_rgba(10,10,10,0.10)]',
					'transition-transform',
					entered ? 'translate-x-0' : 'translate-x-full',
				].join(' ')}
			>
				{children}
			</div>
		</>
	)
}
