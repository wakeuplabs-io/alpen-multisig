import { useState } from 'react'
import type { PastedSignature } from '@/domain/proposal-detail/model/pasted-signature'

type Props = {
	existingSignatures: Array<{ signerPubkey: string }>
	knownSigners: string[]
	onImport: (sigs: PastedSignature[]) => void
	onClose: () => void
}

type RowResult = { ok: true; sig: PastedSignature; warning: string | null } | { ok: false; error: string }

function parseSignaturesInput(raw: string): RowResult[] {
	const trimmed = raw.trim()
	if (!trimmed) return []

	// Try JSON array or object
	if (trimmed.startsWith('[') || trimmed.startsWith('{')) {
		try {
			const parsed: unknown = JSON.parse(trimmed)
			const items: unknown[] = Array.isArray(parsed) ? parsed : [parsed]
			return items.map((item, i): RowResult => {
				if (typeof item !== 'object' || item === null) {
					return { ok: false, error: `Item ${i}: expected object` }
				}
				const obj = item as Record<string, unknown>
				if (typeof obj['signerPubkey'] !== 'string' || typeof obj['signatureHex'] !== 'string') {
					return { ok: false, error: `Item ${i}: missing signerPubkey or signatureHex` }
				}
				return validateRow(obj['signerPubkey'], obj['signatureHex'], i)
			})
		} catch {
			return [{ ok: false, error: 'Invalid JSON — expected array or object' }]
		}
	}

	// Newline-separated "pubkey sig" pairs
	return trimmed
		.split('\n')
		.map((line) => line.trim())
		.filter(Boolean)
		.map((line, i) => {
			const parts = line.split(/\s+/)
			if (parts.length < 2) {
				return { ok: false, error: `Line ${i + 1}: expected "<pubkey> <signatureHex>"` }
			}
			return validateRow(parts[0]!, parts[1]!, i)
		})
}

function validateRow(pubkey: string, sig: string, i: number): RowResult {
	const p = pubkey.toLowerCase().replace(/^0x/, '')
	const s = sig.toLowerCase().replace(/^0x/, '')
	if (!/^[0-9a-f]+$/.test(p) || p.length !== 66) {
		return { ok: false, error: `Entry ${i}: invalid pubkey (expected 33-byte compressed hex, 66 chars)` }
	}
	if (!/^[0-9a-f]+$/.test(s) || s.length !== 128) {
		return { ok: false, error: `Entry ${i}: invalid signature (expected 64-byte Schnorr hex, 128 chars)` }
	}
	return { ok: true, sig: { signerPubkey: p, signatureHex: s }, warning: null }
}

function truncate(hex: string) {
	return `${hex.slice(0, 10)}…${hex.slice(-6)}`
}

export function PasteSignaturesModal({ existingSignatures, knownSigners, onImport, onClose }: Props) {
	const [raw, setRaw] = useState('')
	const parsed = raw.trim() ? parseSignaturesInput(raw) : null

	const validRows = parsed?.filter((r): r is Extract<RowResult, { ok: true }> => r.ok) ?? []
	const errorRows = parsed?.filter((r): r is Extract<RowResult, { ok: false }> => !r.ok) ?? []

	const existingPubkeys = new Set(existingSignatures.map((s) => s.signerPubkey.toLowerCase()))
	const knownSet = new Set(knownSigners.map((s) => s.toLowerCase()))

	const dedupedValid = validRows.filter((r) => !existingPubkeys.has(r.sig.signerPubkey))
	const duplicateCount = validRows.length - dedupedValid.length

	const unknownWarnings = dedupedValid.filter((r) => !knownSet.has(r.sig.signerPubkey))

	function handleImport() {
		if (dedupedValid.length === 0) return
		onImport(dedupedValid.map((r) => r.sig))
		onClose()
	}

	return (
		<div
			className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
			onClick={(e) => {
				if (e.target === e.currentTarget) onClose()
			}}
		>
			<div className="mx-4 w-full max-w-lg rounded-xl border border-[#e5e7eb] bg-white shadow-xl">
				<div className="border-b border-[#f3f4f6] px-6 py-4">
					<h3 className="m-0 text-[15px] font-semibold text-[#111827]">Paste signatures</h3>
					<p className="m-0 mt-0.5 text-[12px] text-[#6b7280]">
						JSON array, single object, or one <code>pubkey signatureHex</code> per line.
					</p>
				</div>

				<div className="p-6">
					<textarea
						className="w-full rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5 font-mono text-[12px] text-[#111827] outline-none focus:border-[#9ca3af] resize-none"
						rows={6}
						placeholder={'[{ "signerPubkey": "02…", "signatureHex": "…" }]'}
						value={raw}
						onChange={(e) => setRaw(e.target.value)}
						autoFocus
					/>

					{/* Errors */}
					{errorRows.length > 0 && (
						<div className="mt-3 space-y-1">
							{errorRows.map((r, i) => (
								<p key={i} className="m-0 text-[12px] text-[#dc2626]">
									{r.error}
								</p>
							))}
						</div>
					)}

					{/* Duplicates notice */}
					{duplicateCount > 0 && (
						<p className="mt-2 m-0 text-[12px] text-[#d97706]">
							{duplicateCount} duplicate{duplicateCount > 1 ? 's' : ''} skipped (already signed).
						</p>
					)}

					{/* Preview table */}
					{dedupedValid.length > 0 && (
						<div className="mt-3 divide-y divide-[#f3f4f6] rounded-lg border border-[#e5e7eb]">
							{dedupedValid.map((r, i) => {
								const isUnknown = !knownSet.has(r.sig.signerPubkey)
								return (
									<div key={i} className="flex items-center gap-3 px-3 py-2">
										<span className="flex-1 font-mono text-[11px] text-[#374151]">{truncate(r.sig.signerPubkey)}</span>
										{isUnknown && (
											<span className="rounded-full border border-[#fde68a] bg-[#fffbeb] px-1.5 py-0.5 text-[10px] text-[#d97706]">
												unknown signer
											</span>
										)}
										<span className="text-[11px] text-[#9ca3af]">✓ valid</span>
									</div>
								)
							})}
						</div>
					)}

					{/* Unknown signer warning */}
					{unknownWarnings.length > 0 && (
						<p className="mt-2 m-0 text-[12px] text-[#d97706]">
							⚠ {unknownWarnings.length} pubkey{unknownWarnings.length > 1 ? 's' : ''} not in the known signer set —
							verify before importing.
						</p>
					)}
				</div>

				<div className="flex items-center justify-end gap-2 border-t border-[#f3f4f6] px-6 py-4">
					<button
						type="button"
						className="rounded-lg border border-[#e5e7eb] bg-white px-3 py-1.5 text-[13px] font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
						onClick={onClose}
					>
						Cancel
					</button>
					<button
						type="button"
						disabled={dedupedValid.length === 0}
						className="rounded-lg border border-[#111827] bg-[#111827] px-3 py-1.5 text-[13px] font-medium text-white transition hover:bg-black disabled:cursor-not-allowed disabled:opacity-40"
						onClick={handleImport}
					>
						Import{' '}
						{dedupedValid.length > 0 ? `${dedupedValid.length} signature${dedupedValid.length > 1 ? 's' : ''}` : ''}
					</button>
				</div>
			</div>
		</div>
	)
}
