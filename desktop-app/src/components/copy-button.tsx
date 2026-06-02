import { useState } from 'react'
import { writeClipboard } from '@/api/tauri-bridge'
import { CheckEmeraldIcon, CopyClipboardIcon } from '@/assets/icons'

export function CopyButton({ text, label }: { text: string; label?: string }) {
	const [copied, setCopied] = useState(false)

	function handleCopy() {
		void writeClipboard(text).then(() => {
			setCopied(true)
			setTimeout(() => setCopied(false), 2000)
		})
	}

	return (
		<button
			type="button"
			onClick={handleCopy}
			className={`inline-flex shrink-0 items-center gap-1 rounded-md border px-2.5 py-1.5 text-xs font-medium transition ${
				copied
					? 'border-[#6ee7b7] bg-[#ecfdf5] text-[#065f46]'
					: 'border-[#e5e7eb] bg-white text-[#6b7280] hover:border-[#d1d5db] hover:text-[#111827]'
			}`}
		>
			{copied ? <CheckEmeraldIcon width={12} height={12} /> : <CopyClipboardIcon width={12} height={12} />}
			{copied ? 'Copied!' : (label ?? 'Copy')}
		</button>
	)
}
