import { useFormContext } from 'react-hook-form'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass } from '../model/create-proposal-form-styles'

export function VkUpdateFormFields() {
	const {
		register,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	return (
		<div>
			<label className="text-sm font-medium text-[#111827]">New verification key</label>
			<input
				type="text"
				className={monoInputClass}
				{...register('newVkHex')}
				placeholder="Verification key hex..."
				spellCheck={false}
			/>
			{errors.newVkHex?.message && <p className={fieldErrorClass}>{errors.newVkHex.message}</p>}
		</div>
	)
}
