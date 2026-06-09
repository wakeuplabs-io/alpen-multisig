import { useState } from 'react'
import { CheckEmeraldIcon, DownloadIcon } from '@/assets/icons'

export function DownloadButton({ text, filename, label }: { text: string; filename: string; label?: string }) {
	const [downloaded, setDownloaded] = useState(false)

	function handleDownload() {
		const blob = new Blob([text], { type: 'text/plain' })
		const url = URL.createObjectURL(blob)
		const a = document.createElement('a')
		a.href = url
		a.download = filename
		a.click()
		URL.revokeObjectURL(url)
		setDownloaded(true)
		setTimeout(() => setDownloaded(false), 2000)
	}

	return (
		<button
			type="button"
			onClick={handleDownload}
			className={`inline-flex shrink-0 items-center gap-1 rounded-md border px-2.5 py-1.5 text-xs font-medium transition ${
				downloaded
					? 'border-[#6ee7b7] bg-[#ecfdf5] text-[#065f46]'
					: 'border-[#e5e7eb] bg-white text-[#6b7280] hover:border-[#d1d5db] hover:text-[#111827]'
			}`}
		>
			{downloaded ? <CheckEmeraldIcon width={12} height={12} /> : <DownloadIcon width={12} height={12} />}
			{downloaded ? 'Saved!' : (label ?? 'Download')}
		</button>
	)
}
