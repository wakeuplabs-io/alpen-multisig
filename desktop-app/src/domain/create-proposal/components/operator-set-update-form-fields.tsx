import { TrashIcon, UndoIcon } from '@/assets/icons'
import { useState } from 'react'
import { useFieldArray, useFormContext, useWatch } from 'react-hook-form'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass } from '../model/create-proposal-form-styles'

type Props = {
	isLoadingConfig: boolean
	currentOperators: string[]
	isLoadingOperators: boolean
}

export function OperatorSetUpdateFormFields({ isLoadingConfig, currentOperators, isLoadingOperators }: Props) {
	const {
		control,
		formState: { errors },
		setValue,
	} = useFormContext<CreateProposalFormValues>()

	const [newOperatorInput, setNewOperatorInput] = useState('')

	const operatorsToAddArray = useFieldArray({ control, name: 'operatorsToAdd' })
	const operatorIndicesToRemoveArray = useFieldArray({ control, name: 'operatorIndicesToRemove' })
	const operatorsToAdd = useWatch({ control, name: 'operatorsToAdd' }) ?? []
	const operatorIndicesToRemove = useWatch({ control, name: 'operatorIndicesToRemove' }) ?? []

	const pendingRemovalSet = new Set(operatorIndicesToRemove.map((r) => r.value.trim()).filter((v) => v.length > 0))

	const visibleAddedOperators = operatorsToAdd
		.map((r, i) => ({ value: r.value.trim(), index: i }))
		.filter((r) => r.value.length > 0)

	function handleRemoveCurrent(operatorIndex: number) {
		const idxStr = String(operatorIndex)
		const emptySlot = operatorIndicesToRemove.findIndex((r) => r.value.trim() === '')
		if (emptySlot !== -1) {
			setValue(`operatorIndicesToRemove.${emptySlot}.value`, idxStr, { shouldValidate: true })
		} else {
			operatorIndicesToRemoveArray.append({ value: idxStr })
		}
	}

	function handleUndoRemove(operatorIndex: number) {
		const idxStr = String(operatorIndex)
		const matchIdx = operatorIndicesToRemove.findIndex((r) => r.value.trim() === idxStr)
		if (matchIdx === -1) return
		if (operatorIndicesToRemoveArray.fields.length > 1) {
			operatorIndicesToRemoveArray.remove(matchIdx)
		} else {
			setValue('operatorIndicesToRemove.0.value', '', { shouldValidate: true })
		}
	}

	function handleRemoveAdded(fieldIndex: number) {
		if (operatorsToAddArray.fields.length > 1) {
			operatorsToAddArray.remove(fieldIndex)
		} else {
			setValue('operatorsToAdd.0.value', '', { shouldValidate: true })
		}
	}

	function handleAddOperator() {
		const trimmed = newOperatorInput.trim()
		if (!trimmed) return
		const emptyIdx = operatorsToAdd.findIndex((r) => r.value.trim() === '')
		if (emptyIdx !== -1) {
			setValue(`operatorsToAdd.${emptyIdx}.value`, trimmed, { shouldValidate: true })
		} else {
			operatorsToAddArray.append({ value: trimmed })
		}
		setNewOperatorInput('')
	}

	const addErrors = errors.operatorsToAdd as Record<number, { value?: { message?: string } } | undefined> | undefined
	const isLoading = isLoadingConfig || isLoadingOperators

	return (
		<>
			{/* Current operator set — interactive removal */}
			<div>
				<p className="mb-3 text-sm font-medium text-[#6b7280]">Current Operators</p>
				{isLoading ? (
					<div className="space-y-2">
						{[1, 2, 3].map((i) => (
							<div key={i} className="h-12 animate-pulse rounded-lg bg-[#f3f4f6]" />
						))}
					</div>
				) : (
					<div className="rounded-xl border border-[#d97706] bg-[#fffbeb] p-3">
						<div className="flex flex-col gap-2">
							{currentOperators.map((pubkey, idx) => {
								const isPendingRemoval = pendingRemovalSet.has(String(idx))
								return (
									<div
										key={idx}
										className="flex items-center gap-3 rounded-lg border border-[#e5e7eb] bg-white px-4 py-3"
									>
										<span
											className={`shrink-0 text-xs font-medium tabular-nums ${isPendingRemoval ? 'text-[#dc2626]' : 'text-[#9ca3af]'}`}
										>
											#{idx}
										</span>
										<span
											className={`min-w-0 flex-1 break-all font-mono text-xs ${isPendingRemoval ? 'text-[#dc2626] line-through' : 'text-[#374151]'}`}
										>
											{pubkey}
										</span>
										{isPendingRemoval ? (
											<button
												type="button"
												className="shrink-0 rounded-md border border-[#3b82f6] p-1 text-[#3b82f6] hover:bg-[#eff6ff]"
												onClick={() => handleUndoRemove(idx)}
												aria-label="Undo remove operator"
											>
												<UndoIcon width={14} height={14} />
											</button>
										) : (
											<button
												type="button"
												className="shrink-0 text-[#c2773b] hover:text-[#92400e]"
												onClick={() => handleRemoveCurrent(idx)}
												aria-label="Remove operator"
											>
												<TrashIcon width={16} height={16} />
											</button>
										)}
									</div>
								)
							})}
							{/* Newly added operators shown in the same list */}
							{visibleAddedOperators.map(({ value, index }) => {
								const itemError = addErrors?.[index]?.value?.message
								return (
									<div key={index} className="flex flex-col gap-1">
										<div
											className={`flex items-center gap-3 rounded-lg border bg-white px-4 py-3 ${itemError ? 'border-[#dc2626]' : 'border-[#e5e7eb]'}`}
										>
											<span
												className={`shrink-0 text-xs font-medium ${itemError ? 'text-[#dc2626]' : 'text-[#d97706]'}`}
											>
												+new
											</span>
											<span
												className={`min-w-0 flex-1 break-all font-mono text-xs ${itemError ? 'text-[#dc2626]' : 'text-[#374151]'}`}
											>
												{value}
											</span>
											<button
												type="button"
												className="shrink-0 text-[#c2773b] hover:text-[#92400e]"
												onClick={() => handleRemoveAdded(index)}
												aria-label="Remove added operator"
											>
												<TrashIcon width={16} height={16} />
											</button>
										</div>
										{itemError && <p className={fieldErrorClass}>{itemError}</p>}
									</div>
								)
							})}
							{currentOperators.length === 0 && visibleAddedOperators.length === 0 && (
								<div className="py-4 text-center text-sm text-[#9ca3af]">No operators registered.</div>
							)}
						</div>
					</div>
				)}
				{typeof errors.operatorsToAdd?.message === 'string' && (
					<p className={fieldErrorClass}>{errors.operatorsToAdd.message}</p>
				)}
				{typeof errors.operatorIndicesToRemove?.message === 'string' && (
					<p className={fieldErrorClass}>{errors.operatorIndicesToRemove.message}</p>
				)}
			</div>

			{/* Add new operator */}
			<div>
				<label className="text-sm font-medium text-[#6b7280]">Add new operator (x-only pubkey hex)</label>
				<div className="mt-1.5 flex gap-2">
					<input
						type="text"
						className={`min-w-0 flex-1 ${monoInputClass}`}
						placeholder="x-only pubkey hex (64 chars)"
						value={newOperatorInput}
						onChange={(e) => setNewOperatorInput(e.target.value)}
						onKeyDown={(e) => {
							if (e.key === 'Enter') {
								e.preventDefault()
								handleAddOperator()
							}
						}}
						spellCheck={false}
					/>
					<button
						type="button"
						className="shrink-0 rounded-lg border border-[#0a0a0a] px-4 py-2.5 text-sm font-medium text-[#0a0a0a] hover:bg-[#f8f8fb]"
						onClick={handleAddOperator}
					>
						+ Add
					</button>
				</div>
			</div>
		</>
	)
}
