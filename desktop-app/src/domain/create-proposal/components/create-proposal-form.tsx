import { zodResolver } from '@hookform/resolvers/zod'
import type { Proposal } from '@/api/proposals'
import { EyeGrayIcon, PencilWhiteIcon } from '@/assets/icons'
import { useEffect, useMemo } from 'react'
import { FormProvider, useForm, useWatch } from 'react-hook-form'
import type { MultisigConfigSnapshot } from '../model/create-proposal.types'
import { buildCreateProposalFormSchema, type CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, numberInputClass, textInputClass } from '../model/create-proposal-form-styles'
import { ActionTypeCard, LabelWithTooltip } from './create-proposal-form-primitives'
import { SignerUpdateFormFields } from './signer-update-form-fields'
import { VkUpdateFormFields } from './vk-update-form-fields'

type Props = {
	authorityLabel: string
	multisigConfig: MultisigConfigSnapshot | null
	multisigConfigVersion: number
	isLoadingConfig: boolean
	isSubmitting: boolean
	error: string | null
	createdProposal: Proposal | null
	onCancel: () => void
	onSubmitValid: (data: CreateProposalFormValues) => Promise<void>
}

const defaultFormValues: CreateProposalFormValues = {
	actionType: 'signer_update',
	seqNo: '',
	title: '',
	keysToAdd: [{ value: '' }],
	keysToRemove: [{ value: '' }],
	threshold: '2',
	newVkHex: '',
}

export function CreateProposalForm({
	authorityLabel,
	multisigConfig,
	multisigConfigVersion,
	isLoadingConfig,
	isSubmitting,
	error,
	createdProposal,
	onCancel,
	onSubmitValid,
}: Props) {
	const createProposalSchema = useMemo(
		() =>
			buildCreateProposalFormSchema({
				currentMultisigSigners: multisigConfig?.signers ?? null,
			}),
		[multisigConfig],
	)

	const resolver = useMemo(() => zodResolver(createProposalSchema), [createProposalSchema])

	const form = useForm<CreateProposalFormValues, unknown, CreateProposalFormValues>({
		resolver,
		defaultValues: defaultFormValues,
		mode: 'all',
		reValidateMode: 'onChange',
	})

	const { handleSubmit, reset, formState, getValues, control, trigger } = form
	const actionType = useWatch({ control, name: 'actionType' })
	const keysToAddWatched = useWatch({ control, name: 'keysToAdd' })
	const keysToRemoveWatched = useWatch({ control, name: 'keysToRemove' })
	const signerKeysDigest =
		actionType === 'signer_update'
			? [
					multisigConfigVersion,
					JSON.stringify((keysToAddWatched ?? []).map((r) => r.value)),
					JSON.stringify((keysToRemoveWatched ?? []).map((r) => r.value)),
				].join('|')
			: ''

	useEffect(() => {
		if (actionType !== 'signer_update' || signerKeysDigest === '') return
		void trigger('threshold')
	}, [actionType, signerKeysDigest, trigger])

	useEffect(() => {
		if (multisigConfigVersion === 0 || multisigConfig === null) return
		const current = getValues()
		reset({
			...current,
			keysToRemove:
				multisigConfig.signers.length > 0
					? multisigConfig.signers.map((s) => ({ value: s }))
					: [{ value: '' }],
			threshold: String(multisigConfig.threshold),
		})
	}, [multisigConfigVersion, multisigConfig, reset, getValues])

	return (
		<FormProvider {...form}>
			<div className="w-full max-w-[760px]">
				<div className="mb-6">
					<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2rem] font-normal leading-[1.15] text-[#0a0a0a]">
						Create {authorityLabel} proposal
					</h1>
					<p className="m-0 mt-2 text-sm text-[#6b7280]">
						Authority: <span className="font-semibold text-[#111827]">{authorityLabel}</span> · You will sign this
						proposal immediately after creation.
					</p>
				</div>

				<form
					className="rounded-2xl border border-[#e5e7eb] bg-white p-8"
					onSubmit={handleSubmit((data) => void onSubmitValid(data))}
					noValidate
				>
					<div className="flex flex-col gap-6">
						<div>
							<p className="mb-3 text-sm font-medium text-[#111827]">Action type</p>
							<div className="grid grid-cols-2 gap-3">
								<ActionTypeCard
									title="Verification key update"
									description="Rotate the Alpen VK."
									selected={actionType === 'vk_update'}
									onClick={() =>
										form.setValue('actionType', 'vk_update', { shouldValidate: true, shouldDirty: true })
									}
								/>
								<ActionTypeCard
									title="Signer update"
									description="Add / remove signers or change threshold."
									selected={actionType === 'signer_update'}
									onClick={() =>
										form.setValue('actionType', 'signer_update', { shouldValidate: true, shouldDirty: true })
									}
								/>
							</div>
						</div>

						<div className="max-w-[180px]">
							<LabelWithTooltip
								label="Sequence number"
								tooltip="The monotonically increasing sequence number for this proposal. Must match the expected next value on-chain."
							/>
							<input
								type="number"
								min={0}
								className={numberInputClass}
								{...form.register('seqNo')}
								placeholder="0"
							/>
							{formState.errors.seqNo?.message && (
								<p className={fieldErrorClass}>{formState.errors.seqNo.message}</p>
							)}
						</div>

						<div>
							<label className="text-sm font-medium text-[#111827]">Title</label>
							<input
								type="text"
								className={textInputClass}
								{...form.register('title')}
								placeholder="e.g. Rotate verification key (Q2 2026)"
							/>
							{formState.errors.title?.message && (
								<p className={fieldErrorClass}>{formState.errors.title.message}</p>
							)}
						</div>

						{actionType === 'signer_update' ? (
							<SignerUpdateFormFields isLoadingConfig={isLoadingConfig} />
						) : (
							<VkUpdateFormFields />
						)}
					</div>

					{error && (
						<div className="mt-6 rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3 text-sm text-[#991b1b]">
							{error}
						</div>
					)}

					{createdProposal && (
						<div className="mt-6 rounded-xl border border-[#bbf7d0] bg-[#f0fdf4] px-4 py-3">
							<p className="m-0 text-sm font-medium text-[#166534]">Proposal created successfully</p>
							<p className="m-0 mt-1 font-mono text-xs text-[#15803d]">{createdProposal.actionId}</p>
						</div>
					)}

					<div className="mt-8 border-t border-[#e5e7eb] pt-5">
						<div className="flex items-center justify-end gap-3">
							<button
								type="button"
								className="rounded-full border border-[#0a0a0a] bg-white px-6 py-2.5 text-sm font-medium text-[#0a0a0a] hover:bg-[#f8f8fb]"
								onClick={onCancel}
								disabled={isSubmitting}
							>
								Cancel
							</button>
							<button
								type="button"
								className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-white px-5 py-2.5 text-sm font-medium text-[#374151] hover:bg-[#f8f8fb] disabled:cursor-not-allowed disabled:opacity-50"
								disabled={isSubmitting}
								onClick={() => void trigger(undefined, { shouldFocus: true })}
							>
								<EyeGrayIcon width={15} height={15} className="block shrink-0" />
								Preview
							</button>
							<button
								type="submit"
								className="flex items-center gap-2 rounded-lg bg-[#0a0a0a] px-5 py-2.5 text-sm font-medium text-white hover:bg-[#1a1a1a] disabled:cursor-not-allowed disabled:bg-[#9ca3af]"
								disabled={isSubmitting || isLoadingConfig || !formState.isValid}
							>
								<PencilWhiteIcon width={14} height={14} className="block shrink-0" />
								{isSubmitting ? 'Signing...' : 'Create & sign'}
							</button>
						</div>
					</div>
				</form>
			</div>
		</FormProvider>
	)
}
