import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { readDroppedJson } from '@/api/tauri-bridge'

type Options = {
	isEnabled: boolean
	onFile: (file: File) => void
}

type JsonFileDrop = {
	isDragging: boolean
	error: string | null
}

/**
 * Loads a `.json` file dropped onto the window (#606).
 *
 * Drops come from Tauri's native drag-drop event, not the HTML5 `drop` event: WebKitGTK
 * never hands dropped files to the DOM, and Windows blocks them while native drag-drop is
 * on. The event carries paths, so the file is read back through the IPC — which only
 * accepts a path from the last drop — and handed over as a `File`, the same shape the file
 * picker produces.
 */
export function useJsonFileDrop({ isEnabled, onFile }: Options): JsonFileDrop {
	const [isDragging, setIsDragging] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const isEnabledRef = useRef(isEnabled)
	const onFileRef = useRef(onFile)
	useEffect(() => {
		isEnabledRef.current = isEnabled
		onFileRef.current = onFile
	})

	useEffect(() => {
		if (!isTauri()) return

		async function handleDrop(paths: string[]) {
			const path = paths.find((p) => /\.json$/i.test(p))
			if (!path) {
				setError('Drop a .json bundle file.')
				return
			}
			setError(null)
			try {
				const text = await readDroppedJson(path)
				const name = path.split(/[\\/]/).pop() ?? 'bundle.json'
				onFileRef.current(new File([text], name, { type: 'application/json' }))
			} catch (err) {
				setError(err instanceof Error ? err.message : String(err))
			}
		}

		let unlisten: (() => void) | undefined
		let isCancelled = false
		getCurrentWebview()
			.onDragDropEvent((event) => {
				const payload = event.payload
				if (payload.type === 'enter') {
					setError(null)
					setIsDragging(true)
					return
				}
				if (payload.type === 'over') {
					setIsDragging(true)
					return
				}
				setIsDragging(false)
				if (payload.type === 'drop' && isEnabledRef.current) void handleDrop(payload.paths)
			})
			.then((stop) => {
				if (isCancelled) stop()
				else unlisten = stop
			})
			.catch((err: unknown) => {
				setError(`File drop is unavailable: ${err instanceof Error ? err.message : String(err)}`)
			})

		return () => {
			isCancelled = true
			unlisten?.()
		}
	}, [])

	return { isDragging, error }
}
