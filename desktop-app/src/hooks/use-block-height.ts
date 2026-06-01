import { useEffect, useState } from 'react'
import { getBitcoinBlockHeight } from '@/api/asm-state'

const POLL_INTERVAL_MS = 15_000

export function useBlockHeight(): number | null {
	const [height, setHeight] = useState<number | null>(null)

	useEffect(() => {
		let cancelled = false

		async function fetch() {
			const result = await getBitcoinBlockHeight()
			if (!cancelled && result.ok) {
				setHeight(result.data)
			}
		}

		fetch()
		const id = setInterval(fetch, POLL_INTERVAL_MS)
		return () => {
			cancelled = true
			clearInterval(id)
		}
	}, [])

	return height
}
