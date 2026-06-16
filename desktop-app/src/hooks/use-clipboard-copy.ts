import { useCallback, useEffect, useRef, useState } from 'react'
import { writeClipboard } from '@/api/tauri-bridge'

export type ClipboardCopy = {
	/** True for `resetMs` after a successful copy — drives "Copied!" feedback. */
	copied: boolean
	/** Write `text` to the clipboard and flash the `copied` flag. No-op for empty text. */
	copy: (text: string) => void
}

/**
 * Shared copy-to-clipboard hook with transient "copied" feedback. Used by every
 * click-to-copy surface (copy buttons, address text, receive QR) so the success
 * affordance is implemented once and the reset timer is always cleaned up.
 */
export function useClipboardCopy(resetMs = 2000): ClipboardCopy {
	const [copied, setCopied] = useState(false)
	const timeoutRef = useRef<number | null>(null)

	useEffect(() => {
		return () => {
			if (timeoutRef.current !== null) window.clearTimeout(timeoutRef.current)
		}
	}, [])

	const copy = useCallback(
		(text: string) => {
			if (!text) return
			void writeClipboard(text).then(() => {
				setCopied(true)
				if (timeoutRef.current !== null) window.clearTimeout(timeoutRef.current)
				timeoutRef.current = window.setTimeout(() => setCopied(false), resetMs)
			})
		},
		[resetMs],
	)

	return { copied, copy }
}
