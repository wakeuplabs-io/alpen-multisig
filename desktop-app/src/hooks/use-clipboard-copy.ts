import { useCallback, useEffect, useRef, useState } from 'react'
import { writeClipboard } from '@/api/tauri-bridge'

export type ClipboardCopy = {
	/** True for `resetMs` after a successful copy — drives "Copied!" feedback. */
	copied: boolean
	/** Set when the last copy failed, so the UI can say so instead of looking inert. */
	error: string | null
	/** Write `text` to the clipboard and flash the `copied` flag. No-op for empty text. */
	copy: (text: string) => void
}

/**
 * Shared copy-to-clipboard hook with transient "copied" feedback. Used by every
 * click-to-copy surface (copy buttons, address text, receive QR) so the success
 * affordance is implemented once and the reset timer is always cleaned up.
 *
 * Writes go through the Tauri IPC rather than `navigator.clipboard`, which the
 * WebView can reject. A rejected write is surfaced as `error` instead of being
 * swallowed: a button that silently does nothing is indistinguishable from a
 * broken one, which is how #428 was reported (feedback 2026-07-01).
 */
export function useClipboardCopy(resetMs = 2000): ClipboardCopy {
	const [copied, setCopied] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const timeoutRef = useRef<number | null>(null)

	useEffect(() => {
		return () => {
			if (timeoutRef.current !== null) window.clearTimeout(timeoutRef.current)
		}
	}, [])

	const copy = useCallback(
		(text: string) => {
			if (!text) return
			void writeClipboard(text).then(
				() => {
					setError(null)
					setCopied(true)
					if (timeoutRef.current !== null) window.clearTimeout(timeoutRef.current)
					timeoutRef.current = window.setTimeout(() => setCopied(false), resetMs)
				},
				(reason: unknown) => {
					setCopied(false)
					setError(`Could not copy to the clipboard: ${String(reason)}`)
				},
			)
		},
		[resetMs],
	)

	return { copied, error, copy }
}
