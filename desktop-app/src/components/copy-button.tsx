import { useClipboardCopy } from '@/hooks/use-clipboard-copy'

import { CopyClipboardIcon, CheckEmeraldIcon } from '@/assets/icons'

export function CopyButton({ text, variant = 'labeled' }: { text: string; variant?: 'labeled' | 'icon' }) {
	const { copied, copy } = useClipboardCopy()

	function handleCopy() {
		copy(text)
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
			className={`inline-flex shrink-0 items-center gap-1 rounded-md border px-2.5 py-1.5 text-xs font-medium transition ${
				copied
					? 'border-[#6ee7b7] bg-[#ecfdf5] text-[#065f46]'
					: 'border-[#e5e7eb] bg-white text-[#6b7280] hover:border-[#d1d5db] hover:text-[#111827]'
			}`}
		>
			{copied ? <CheckEmeraldIcon width={12} height={12} /> : <CopyClipboardIcon width={12} height={12} />}
			{copied ? 'Copied!' : 'Copy'}
		</button>
	)
}
