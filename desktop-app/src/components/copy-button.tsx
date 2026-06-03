import { useState } from 'react'
import { CopyClipboardIcon, CheckEmeraldIcon } from '@/assets/icons'

export function CopyButton({ text, variant = 'labeled' }: { text: string; variant?: 'labeled' | 'icon' }) {
	const [copied, setCopied] = useState(false)

	function handleCopy() {
		void navigator.clipboard.writeText(text).then(() => {
			setCopied(true)
			setTimeout(() => setCopied(false), 2000)
		})
	}

	if (variant === 'icon') {
		return (
			<button
				type="button"
				onClick={handleCopy}
				aria-label={copied ? 'Copied' : 'Copy address'}
				title={copied ? 'Copied' : 'Copy'}
				className="inline-flex shrink-0 items-center justify-center rounded-md p-1.5 text-[#9ca3af] transition hover:bg-[#f3f4f6] hover:text-[#6b7280]"
			>
				{copied ? <CheckEmeraldIcon width={14} height={14} /> : <CopyClipboardIcon width={14} height={14} />}
			</button>
		)
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
