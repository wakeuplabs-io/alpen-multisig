import { CopyButton } from '@/components/copy-button'

/**
 * A labelled, truncated hex value with a copy button — a txid on the progress
 * stepper, a raw signed transaction on the manual-send panel.
 *
 * One implementation on purpose: the panel that renders raw transactions was
 * extracted from the stepper, and copying this row alongside it would have been
 * the same "one rule, two copies" the cancel gate cost this phase.
 */
export function HexCopyRow({ label, value }: { label: string; value: string }) {
	return (
		<div>
			<p className="mb-1.5 text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">{label}</p>
			<div className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
				<span className="min-w-0 flex-1 truncate font-mono text-label text-[#111827]">{value}</span>
				<CopyButton text={value} />
			</div>
		</div>
	)
}
