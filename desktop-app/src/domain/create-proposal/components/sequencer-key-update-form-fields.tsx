import { useFormContext } from 'react-hook-form'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass } from '../model/create-proposal-form-styles'

export function SequencerKeyUpdateFormFields() {
	const {
		register,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	return (
		<div className="flex flex-col gap-5">
			<div>
				<label className="text-body font-medium text-[#111827]">New sequencer public key</label>
				<input
					type="text"
					className={monoInputClass}
					{...register('newSequencerKeyHex')}
					placeholder="x-only pubkey hex (32 bytes, 64 hex chars)..."
					spellCheck={false}
				/>
				{errors.newSequencerKeyHex?.message && <p className={fieldErrorClass}>{errors.newSequencerKeyHex.message}</p>}
			</div>
		</div>
	)
}
