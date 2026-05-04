import { MinusHoverIcon, MinusMutedIcon } from '@/assets/icons'
import { useFieldArray, useFormContext } from 'react-hook-form'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass, numberInputClass } from '../model/create-proposal-form-styles'
import { LabelWithTooltip } from './create-proposal-form-primitives'

type Props = {
	isLoadingConfig: boolean
}

export function SignerUpdateFormFields({ isLoadingConfig }: Props) {
	const {
		register,
		control,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	const keysToAddArray = useFieldArray({ control, name: 'keysToAdd' })
	const keysToRemoveArray = useFieldArray({ control, name: 'keysToRemove' })

	return (
		<>
			<div>
				<label className="text-sm font-medium text-[#111827]">Keys to add</label>
				<div className="mt-1.5 flex flex-col gap-2">
					{keysToAddArray.fields.map((field, index) => (
						<div key={field.id} className="flex items-start gap-2">
							<div className="min-w-0 flex-1">
								<input
									type="text"
									className={monoInputClass}
									{...register(`keysToAdd.${index}.value`)}
									placeholder="bc1p..."
									spellCheck={false}
								/>
							</div>
							{keysToAddArray.fields.length > 1 && (
								<button
									type="button"
									className="group/remove-key relative mt-2 shrink-0 rounded-md p-1.5 text-[#9ca3af] hover:bg-[#f3f4f6] hover:text-[#374151]"
									onClick={() => keysToAddArray.remove(index)}
									aria-label="Remove key"
								>
									<span className="relative inline-flex h-3.5 w-3.5">
										<MinusMutedIcon
											width={14}
											height={14}
											className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 transition-opacity group-hover/remove-key:opacity-0"
										/>
										<MinusHoverIcon
											width={14}
											height={14}
											className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 opacity-0 transition-opacity group-hover/remove-key:opacity-100"
										/>
									</span>
								</button>
							)}
						</div>
					))}
				</div>
				{typeof errors.keysToAdd?.message === 'string' && (
					<p className={fieldErrorClass}>{errors.keysToAdd.message}</p>
				)}
				<button
					type="button"
					className="mt-2 flex items-center gap-1 text-sm font-medium text-[#5b44c9] hover:text-[#4736a3]"
					onClick={() => keysToAddArray.append({ value: '' })}
				>
					<span className="text-base leading-none">+</span> Add another
				</button>
			</div>

			<div>
				<label className="text-sm font-medium text-[#111827]">Keys to remove</label>
				<div className="mt-1.5 flex flex-col gap-2">
					{isLoadingConfig ? (
						<div className="h-10 animate-pulse rounded-lg bg-[#f3f4f6]" />
					) : (
						keysToRemoveArray.fields.map((field, index) => (
							<div key={field.id} className="flex items-start gap-2">
								<div className="min-w-0 flex-1">
									<input
										type="text"
										className={monoInputClass}
										{...register(`keysToRemove.${index}.value`)}
										placeholder="bc1p..."
										spellCheck={false}
									/>
								</div>
								{keysToRemoveArray.fields.length > 1 && (
									<button
										type="button"
										className="group/remove-key relative mt-2 shrink-0 rounded-md p-1.5 text-[#9ca3af] hover:bg-[#f3f4f6] hover:text-[#374151]"
										onClick={() => keysToRemoveArray.remove(index)}
										aria-label="Remove key"
									>
										<span className="relative inline-flex h-3.5 w-3.5">
											<MinusMutedIcon
												width={14}
												height={14}
												className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 transition-opacity group-hover/remove-key:opacity-0"
											/>
											<MinusHoverIcon
												width={14}
												height={14}
												className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 opacity-0 transition-opacity group-hover/remove-key:opacity-100"
											/>
										</span>
									</button>
								)}
							</div>
						))
					)}
				</div>
				{typeof errors.keysToRemove?.message === 'string' && (
					<p className={fieldErrorClass}>{errors.keysToRemove.message}</p>
				)}
				<button
					type="button"
					className="mt-2 flex items-center gap-1 text-sm font-medium text-[#5b44c9] hover:text-[#4736a3]"
					onClick={() => keysToRemoveArray.append({ value: '' })}
				>
					<span className="text-base leading-none">+</span> Add another
				</button>
			</div>

			<div className="max-w-[160px]">
				<LabelWithTooltip
					label="New threshold"
					tooltip="Minimum number of signers required to approve a proposal."
				/>
				<input
					type="number"
					min={1}
					max={255}
					className={numberInputClass}
					{...register('threshold')}
				/>
				{errors.threshold?.message && <p className={fieldErrorClass}>{errors.threshold.message}</p>}
			</div>
		</>
	)
}
