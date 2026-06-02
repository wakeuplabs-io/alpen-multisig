import { useRef, useState } from 'react'
import { ImportJsonIcon } from '@/assets/icons'
import type { ManualImportErrors } from '@/domain/manual-proposal/model/manual-proposal.types'

type Props = {
	isValidating: boolean
	errors: ManualImportErrors
	onLoadJson: (file: File) => void
}

export function ManualImportForm({ isValidating, errors, onLoadJson }: Props) {
	const fileInputRef = useRef<HTMLInputElement>(null)
	const [isDragging, setIsDragging] = useState(false)

	const errorMessage = errors.actionHex ?? errors.seqNo ?? errors.authority

	function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
		const file = e.target.files?.[0]
		if (file) onLoadJson(file)
		e.target.value = ''
	}

	function handleDrop(e: React.DragEvent<HTMLDivElement>) {
		e.preventDefault()
		setIsDragging(false)
		const file = e.dataTransfer.files?.[0]
		if (file) onLoadJson(file)
	}

	return (
		<div className="space-y-4">
			<input
				ref={fileInputRef}
				type="file"
				accept=".json,application/json"
				className="hidden"
				onChange={handleFileChange}
			/>

			<div
				role="button"
				tabIndex={0}
				aria-label="Load proposal from JSON file"
				onClick={() => !isValidating && fileInputRef.current?.click()}
				onKeyDown={(e) => e.key === 'Enter' && !isValidating && fileInputRef.current?.click()}
				onDragOver={(e) => {
					e.preventDefault()
					setIsDragging(true)
				}}
				onDragLeave={() => setIsDragging(false)}
				onDrop={handleDrop}
				className={`flex cursor-pointer flex-col items-center justify-center gap-3 rounded-xl border-2 border-dashed px-6 py-12 text-center transition
					${isDragging ? 'border-[#111827] bg-[#f3f4f6]' : 'border-[#e5e7eb] bg-[#f9fafb] hover:border-[#9ca3af] hover:bg-[#f3f4f6]'}
					${isValidating ? 'cursor-not-allowed opacity-60' : ''}
					${errorMessage ? 'border-[#fca5a5] bg-[#fef2f2]' : ''}`}
			>
				{isValidating ? (
					<>
						<div className="h-6 w-6 animate-spin rounded-full border-2 border-[#9ca3af] border-t-[#111827]" />
						<p className="m-0 text-[13px] font-medium text-[#6b7280]">Validating…</p>
					</>
				) : (
					<>
						<ImportJsonIcon width={24} height={24} className="text-[#9ca3af]" />
						<div>
							<p className="m-0 text-[13px] font-medium text-[#374151]">Drop your JSON file here</p>
							<p className="m-0 mt-0.5 text-[12px] text-[#9ca3af]">or click to browse</p>
						</div>
					</>
				)}
			</div>

			{errorMessage && <p className="text-[12px] text-[#dc2626]">{errorMessage}</p>}
		</div>
	)
}
