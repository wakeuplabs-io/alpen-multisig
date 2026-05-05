type Props = {
	title: string
	actionType: 'signer_update' | 'vk_update'
	seqNo: string
	keysToAdd: string[]
	keysToRemove: string[]
	threshold: string
	resultingSignerCount: number | null
	newVkHex: string
	sighashHex: string | null
}

export function CreateProposalPreview({
	title,
	actionType,
	seqNo,
	keysToAdd,
	keysToRemove,
	threshold,
	resultingSignerCount,
	newVkHex,
	sighashHex,
}: Props) {
	const previewActionLabel = actionType === 'signer_update' ? 'Signer update' : 'Verification key update'

	return (
		<div className="rounded-2xl border border-[#d8d9e7] bg-[#f5f5fb] p-6">
			<p className="m-0 text-xs font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">Preview</p>
			<div className="mt-4 space-y-3">
				<p className="m-0 text-[1.75rem] font-semibold leading-none text-[#111827]">{title || '-'}</p>
				<p className="m-0 text-sm text-[#6b7280]">
					<span className="text-[#9ca3af]">Action:</span> {previewActionLabel}
				</p>
				<p className="m-0 text-sm text-[#6b7280]">
					<span className="text-[#9ca3af]">Seqno:</span> {seqNo || '-'}
				</p>
			</div>

			{actionType === 'signer_update' ? (
				<div className="mt-5 space-y-3 text-sm">
					{keysToAdd.length > 0 && (
						<div>
							<p className="m-0 mb-1 text-[#9ca3af]">Adding:</p>
							{keysToAdd.map((key) => (
								<p key={`add-${key}`} className="m-0 break-all font-mono text-[#0f766e]">
									+ {key}
								</p>
							))}
						</div>
					)}
					{keysToRemove.length > 0 && (
						<div>
							<p className="m-0 mb-1 text-[#9ca3af]">Removing:</p>
							{keysToRemove.map((key) => (
								<p key={`remove-${key}`} className="m-0 break-all font-mono text-[#b91c1c]">
									- {key}
								</p>
							))}
						</div>
					)}
					<p className="m-0 text-[#6b7280]">
						<span className="text-[#9ca3af]">New threshold:</span> {threshold || '-'}
					</p>
					<p className="m-0 text-[#6b7280]">
						<span className="text-[#9ca3af]">Resulting signer count:</span> {resultingSignerCount ?? '-'}
					</p>
				</div>
			) : (
				<div className="mt-5 text-sm text-[#6b7280]">
					<p className="m-0 text-[#9ca3af]">New verification key:</p>
					<p className="m-0 mt-1 break-all font-mono text-[#111827]">{newVkHex || '-'}</p>
				</div>
			)}

			<div className="mt-6 border-t border-[#d8d9e7] pt-4">
				<p className="m-0 text-xs font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">Computed sighash</p>
				<p className="m-0 mt-2 break-all font-mono text-sm text-[#111827]">{sighashHex ?? '-'}</p>
			</div>
		</div>
	)
}
