import { TrashIcon, UndoIcon } from '@/assets/icons'
import { useState } from 'react'
import { useFieldArray, useFormContext, useWatch } from 'react-hook-form'
import { normalizeSignerKey, type CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, numberInputClass } from '../model/create-proposal-form-styles'
import { LabelWithTooltip } from './create-proposal-form-primitives'

type Props = {
	isLoadingConfig: boolean
	currentSigners: string[]
	currentThreshold: number
}

export function SignerUpdateFormFields({ isLoadingConfig, currentSigners }: Props) {
	const {
		register,
		control,
		formState: { errors },
		setValue,
	} = useFormContext<CreateProposalFormValues>()

	const [newSignerInput, setNewSignerInput] = useState('')

	const keysToAddArray = useFieldArray({ control, name: 'keysToAdd' })
	const keysToRemoveArray = useFieldArray({ control, name: 'keysToRemove' })
	const keysToAdd = useWatch({ control, name: 'keysToAdd' }) ?? []
	const keysToRemove = useWatch({ control, name: 'keysToRemove' }) ?? []

	const removedSet = new Set(
		keysToRemove
			.map((r) => r.value.trim())
			.filter((v) => v.length > 0)
			.map(normalizeSignerKey),
	)
	const visibleAddedSigners = keysToAdd
		.map((r, i) => ({ value: r.value.trim(), index: i }))
		.filter((r) => r.value.length > 0)

	function handleRemoveCurrent(signer: string) {
		const emptyIdx = keysToRemove.findIndex((r) => r.value.trim() === '')
		if (emptyIdx !== -1) {
			setValue(`keysToRemove.${emptyIdx}.value`, signer, { shouldValidate: true })
		} else {
			keysToRemoveArray.append({ value: signer })
		}
	}

	function handleUndoRemove(signer: string) {
		const target = normalizeSignerKey(signer)
		const matchIdx = keysToRemove.findIndex((r) => normalizeSignerKey(r.value) === target)
		if (matchIdx === -1) return
		if (keysToRemoveArray.fields.length > 1) {
			keysToRemoveArray.remove(matchIdx)
		} else {
			setValue('keysToRemove.0.value', '', { shouldValidate: true })
		}
	}

	function handleRemoveAdded(fieldIndex: number) {
		if (keysToAddArray.fields.length > 1) {
			keysToAddArray.remove(fieldIndex)
		} else {
			setValue('keysToAdd.0.value', '', { shouldValidate: true })
		}
	}

	function handleAddSigner() {
		const trimmed = newSignerInput.trim()
		if (!trimmed) return
		const emptyIdx = keysToAdd.findIndex((r) => r.value.trim() === '')
		if (emptyIdx !== -1) {
			setValue(`keysToAdd.${emptyIdx}.value`, trimmed, { shouldValidate: true })
		} else {
			keysToAddArray.append({ value: trimmed })
		}
		setNewSignerInput('')
	}

	const hasNoSigners = currentSigners.length === 0 && visibleAddedSigners.length === 0

	return (
		<>
			<div>
				<p className="mb-3 text-sm font-medium text-[#6b7280]">Update Signers</p>
				{isLoadingConfig ? (
					<div className="space-y-2">
						{[1, 2, 3].map((i) => (
							<div key={i} className="h-12 animate-pulse rounded-lg bg-[#f3f4f6]" />
						))}
					</div>
				) : (
					<div className="rounded-xl border border-[#d97706] bg-[#fffbeb] p-3">
						<div className="flex flex-col gap-2">
							{currentSigners.map((signer) => {
								const isPendingRemoval = removedSet.has(normalizeSignerKey(signer))
								return (
									<div
										key={signer}
										className="flex items-center gap-3 rounded-lg border border-[#e5e7eb] bg-white px-4 py-3"
									>
										{isPendingRemoval ? (
											<span className="shrink-0 text-sm font-medium text-[#dc2626]">–</span>
										) : (
											<span className="h-4 w-4 shrink-0 rounded-full border border-[#d1d5db]" />
										)}
										<span
											className={`min-w-0 flex-1 break-all font-mono text-sm ${isPendingRemoval ? 'text-[#dc2626] line-through' : 'text-[#374151]'}`}
										>
											{signer}
										</span>
										{isPendingRemoval ? (
											<button
												type="button"
												className="shrink-0 rounded-md border border-[#3b82f6] p-1 text-[#3b82f6] hover:bg-[#eff6ff]"
												onClick={() => handleUndoRemove(signer)}
												aria-label="Undo remove signer"
											>
												<UndoIcon width={14} height={14} />
											</button>
										) : (
											<button
												type="button"
												className="shrink-0 text-[#c2773b] hover:text-[#92400e]"
												onClick={() => handleRemoveCurrent(signer)}
												aria-label="Remove signer"
											>
												<TrashIcon width={16} height={16} />
											</button>
										)}
									</div>
								)
							})}
							{visibleAddedSigners.map(({ value, index }) => (
								<div
									key={index}
									className="flex items-center gap-3 rounded-lg border border-[#e5e7eb] bg-white px-4 py-3"
								>
									<span className="h-4 w-4 shrink-0 rounded-full border border-[#d97706]" />
									<span className="min-w-0 flex-1 break-all font-mono text-sm text-[#374151]">{value}</span>
									<button
										type="button"
										className="shrink-0 text-[#c2773b] hover:text-[#92400e]"
										onClick={() => handleRemoveAdded(index)}
										aria-label="Remove added signer"
									>
										<TrashIcon width={16} height={16} />
									</button>
								</div>
							))}
							{hasNoSigners && <div className="py-4 text-center text-sm text-[#9ca3af]">No signers remaining.</div>}
						</div>
					</div>
				)}
				{typeof errors.keysToAdd?.message === 'string' && <p className={fieldErrorClass}>{errors.keysToAdd.message}</p>}
			</div>

			<div>
				<label className="text-sm font-medium text-[#6b7280]">Add new signer (compressed pubkey hex)</label>
				<div className="mt-1.5 flex gap-2">
					<input
						type="text"
						className="min-w-0 flex-1 rounded-lg border border-[#e5e7eb] px-3 py-2.5 font-mono text-sm text-[#111827] placeholder:text-[#9ca3af] focus:border-[#d97706] focus:outline-none focus:ring-1 focus:ring-[#d97706]"
						placeholder="02… or 03… (33-byte hex)"
						value={newSignerInput}
						onChange={(e) => setNewSignerInput(e.target.value)}
						onKeyDown={(e) => {
							if (e.key === 'Enter') {
								e.preventDefault()
								handleAddSigner()
							}
						}}
						spellCheck={false}
					/>
					<button
						type="button"
						className="shrink-0 rounded-lg border border-[#0a0a0a] px-4 py-2.5 text-sm font-medium text-[#0a0a0a] hover:bg-[#f8f8fb]"
						onClick={handleAddSigner}
					>
						+ Add
					</button>
				</div>
			</div>

			<div className="max-w-40">
				<LabelWithTooltip label="New threshold" tooltip="Minimum number of signers required to approve a proposal." />
				<input type="number" min={1} max={255} className={numberInputClass} {...register('threshold')} />
				{errors.threshold?.message && <p className={fieldErrorClass}>{errors.threshold.message}</p>}
			</div>
		</>
	)
}
