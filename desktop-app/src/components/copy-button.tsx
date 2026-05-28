import { useState } from 'react'
import { CopyClipboardIcon } from '@/assets/icons'

export function CopyButton({ text }: { text: string }) {
	const [copied, setCopied] = useState(false)

	function handleCopy() {
		void navigator.clipboard.writeText(text).then(() => {
			setCopied(true)
			setTimeout(() => setCopied(false), 2000)
		})
	}

	return (
		<button
			type="button"
			onClick={handleCopy}
			className="inline-flex shrink-0 items-center gap-1 rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-xs font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
		>
			<CopyClipboardIcon width={12} height={12} />
			{copied ? 'Copied!' : 'Copy'}
		</button>
	)
}
