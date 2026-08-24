import { useEffect, useId, useRef, type ReactNode } from 'react'
import { CloseIcon } from '@/assets/icons'

const FOCUSABLE_SELECTOR = 'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])'

type Props = {
	isOpen: boolean
	onClose: () => void
	title: string
	children: ReactNode
	panelClassName?: string
	titleClassName?: string
	/**
	 * Accessible name for a close (X) control in the panel's top-right corner. Opt-in:
	 * a dialog whose whole point is an explicit choice (the navigation guard) is worse
	 * with an ambiguous X, so it does not get one.
	 */
	closeLabel?: string
}

export function AccessibleDialog({
	isOpen,
	onClose,
	title,
	children,
	panelClassName,
	titleClassName,
	closeLabel,
}: Props) {
	const titleId = useId()
	const panelRef = useRef<HTMLDivElement>(null)
	const previousFocusRef = useRef<Element | null>(null)

	useEffect(() => {
		if (!isOpen) return

		previousFocusRef.current = document.activeElement

		const panel = panelRef.current
		if (panel) {
			const focusable = panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)
			if (focusable.length > 0) {
				focusable[0].focus()
			}
		}

		return () => {
			if (previousFocusRef.current instanceof HTMLElement) {
				previousFocusRef.current.focus()
			}
			previousFocusRef.current = null
		}
	}, [isOpen])

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

			// The focused element can unmount while the dialog is open (a button replaced by
			// a status chip, say). Focus then falls to <body>, which is neither first nor
			// last, so the wrap-around below would never fire and the next Tab would walk
			// out of the dialog into the page behind it.
			if (!panel.contains(document.activeElement)) {
				e.preventDefault()
				first.focus()
				return
			}

			if (e.shiftKey) {
				if (document.activeElement === first) {
					e.preventDefault()
					last.focus()
				}
			} else if (document.activeElement === last) {
				e.preventDefault()
				first.focus()
			}
		}

		document.addEventListener('keydown', handleKeyDown)
		return () => document.removeEventListener('keydown', handleKeyDown)
	}, [isOpen, onClose])

	if (!isOpen) return null

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
			<div className="absolute inset-0" onClick={onClose} aria-hidden="true" />
			<div
				ref={panelRef}
				role="dialog"
				aria-modal="true"
				aria-labelledby={titleId}
				className={
					panelClassName ?? 'relative w-full max-w-md rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-lg'
				}
			>
				{closeLabel !== undefined && (
					<button
						type="button"
						onClick={onClose}
						aria-label={closeLabel}
						data-testid="e2e-dialog-close"
						className="absolute right-4 top-4 inline-flex items-center justify-center rounded-md p-1.5 text-[#6b7280] transition hover:bg-[#f3f4f6] hover:text-[#111827]"
					>
						<CloseIcon width={16} height={16} />
					</button>
				)}
				<h2 id={titleId} className={`m-0 text-body-lg font-semibold text-[#111827] ${titleClassName ?? ''}`}>
					{title}
				</h2>
				{children}
			</div>
		</div>
	)
}
