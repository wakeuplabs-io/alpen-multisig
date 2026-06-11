import { zodResolver } from '@hookform/resolvers/zod'
import type { CurrentVk } from '@/api/asm-state'
import type { Proposal } from '@/api/proposals'
import { EyeGrayIcon, PencilWhiteIcon } from '@/assets/icons'
import { useEffect, useMemo, useState } from 'react'
import { FormProvider, useForm, useWatch } from 'react-hook-form'
import {
	isSessionExpiredReauthError,
	SESSION_EXPIRED_REAUTH_MESSAGE,
} from '@/domain/create-proposal/hooks/use-create-proposal'
import type { MultisigConfigSnapshot } from '../model/create-proposal.types'
import { buildCreateProposalFormSchema, type CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, numberInputClass, textInputClass } from '../model/create-proposal-form-styles'
import { ActionTypeCard, LabelWithTooltip } from './create-proposal-form-primitives'
import { CreateProposalPreview } from './create-proposal-preview'
import { OperatorSetUpdateFormFields } from './operator-set-update-form-fields'
import { SignerUpdateFormFields } from './signer-update-form-fields'
import { VkUpdateFormFields } from './vk-update-form-fields'

type Props = {
	authorityLabel: string
	authority: string
	multisigConfig: MultisigConfigSnapshot | null
	multisigConfigVersion: number
	isLoadingConfig: boolean
	nextSeqNo: number | null
	isLoadingSeqNo: boolean
	currentVk: CurrentVk | null
	isLoadingCurrentVk: boolean
	currentOperators: string[]
	isLoadingOperators: boolean
	isSubmitting: boolean
	error: string | null
	createdProposal: Proposal | null
	onCancel: () => void
	onPreviewValid: (data: CreateProposalFormValues) => Promise<string | null>
	onSubmitValid: (data: CreateProposalFormValues) => Promise<void>
	onReauthenticate: () => Promise<void>
}

const defaultFormValues: CreateProposalFormValues = {
	actionType: 'signer_update',
	seqNo: '',
	title: '',
	keysToAdd: [{ value: '' }],
	keysToRemove: [{ value: '' }],
	threshold: '2',
	vkTypeId: 'always_accept',
	newVkHex: '',
	operatorsToAdd: [{ value: '' }],
	operatorIndicesToRemove: [{ value: '' }],
}

export function CreateProposalForm({
	authorityLabel,
	authority,
	multisigConfig,
	multisigConfigVersion,
	isLoadingConfig,
	nextSeqNo,
	isLoadingSeqNo,
	currentVk,
	isLoadingCurrentVk,
	currentOperators,
	isLoadingOperators,
	isSubmitting,
	error,
	createdProposal,
	onCancel,
	onPreviewValid,
	onSubmitValid,
	onReauthenticate,
}: Props) {
	const [isPreviewMode, setIsPreviewMode] = useState(false)
	const [previewSighashHex, setPreviewSighashHex] = useState<string | null>(null)
	const [frozenAtPreview, setFrozenAtPreview] = useState<CreateProposalFormValues | null>(null)
	const [showReauthModal, setShowReauthModal] = useState(false)
	const [reauthError, setReauthError] = useState<string | null>(null)
	const [isReauthenticating, setIsReauthenticating] = useState(false)
	const [pendingAction, setPendingAction] = useState<'preview' | 'submit' | null>(null)

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
	const watchedValues = useWatch({ control })
	const actionType = watchedValues?.actionType
	const keysToAddWatched = watchedValues?.keysToAdd
	const keysToRemoveWatched = watchedValues?.keysToRemove
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
			keysToRemove: [{ value: '' }],
			threshold: String(multisigConfig.threshold),
		})
	}, [multisigConfigVersion, multisigConfig, reset, getValues])

	useEffect(() => {
		if (nextSeqNo === null) return
		const current = getValues('seqNo')
		if (current.trim() === '') {
			form.setValue('seqNo', String(nextSeqNo), { shouldValidate: false })
		}
	}, [nextSeqNo, form, getValues])

	useEffect(() => {
		if (!isPreviewMode || frozenAtPreview === null || watchedValues === undefined) {
			return
		}
		if (JSON.stringify(watchedValues) !== JSON.stringify(frozenAtPreview)) {
			setIsPreviewMode(false)
			setPreviewSighashHex(null)
			setFrozenAtPreview(null)
		}
	}, [watchedValues, isPreviewMode, frozenAtPreview])

	const previewData = frozenAtPreview ?? getValues()
	const previewAddingKeys = previewData.keysToAdd.map((row) => row.value.trim()).filter((value) => value.length > 0)
	const previewRemovingKeys = previewData.keysToRemove
		.map((row) => row.value.trim())
		.filter((value) => value.length > 0)
	async function handlePreviewClick() {
		const isValid = await trigger(undefined, { shouldFocus: true })
		if (!isValid) return
		try {
			const snapshot = getValues()
			const sighashHex = await onPreviewValid(snapshot)
			if (sighashHex === null) return
			setFrozenAtPreview(snapshot)
			setPreviewSighashHex(sighashHex)
			setIsPreviewMode(true)
		} catch (error) {
			if (!isSessionExpiredReauthError(error)) return
			setPendingAction('preview')
			setReauthError(null)
			setShowReauthModal(true)
		}
	}

	async function handleSubmitAttempt(data: CreateProposalFormValues) {
		if (frozenAtPreview === null || JSON.stringify(data) !== JSON.stringify(frozenAtPreview)) {
			setIsPreviewMode(false)
			setPreviewSighashHex(null)
			setFrozenAtPreview(null)
			return
		}
		try {
			await onSubmitValid(frozenAtPreview)
		} catch (error) {
			if (!isSessionExpiredReauthError(error)) return
			setPendingAction('submit')
			setReauthError(null)
			setShowReauthModal(true)
		}
	}

	async function handleReauthenticateAndRetry() {
		setReauthError(null)
		setIsReauthenticating(true)
		try {
			await onReauthenticate()
			setShowReauthModal(false)
			const actionToRetry = pendingAction
			setPendingAction(null)
			if (actionToRetry === 'preview') {
				await handlePreviewClick()
			} else if (actionToRetry === 'submit') {
				await handleSubmitAttempt(getValues())
			}
		} catch (error) {
			setReauthError(String(error))
		} finally {
			setIsReauthenticating(false)
		}
	}

	return (
		<FormProvider {...form}>
			<div className="w-full max-w-[760px]">
				<div className="mb-6">
					{isPreviewMode ? (
						<>
							<button
								type="button"
								className="mb-4 flex items-center gap-1.5 text-sm text-[#6b7280] hover:text-[#111827]"
								onClick={() => {
									setIsPreviewMode(false)
									setPreviewSighashHex(null)
									setFrozenAtPreview(null)
								}}
							>
								<span>←</span> Back to new proposal
							</button>
							<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2rem] font-normal leading-[1.15] text-[#0a0a0a]">
								Review &amp; Sign
							</h1>
							<p className="m-0 mt-2 text-sm text-[#6b7280]">
								You are signing the proposal you just drafted. Review the payload, then confirm on your Trezor. Nothing
								is sent until you sign.
							</p>
						</>
					) : (
						<>
							<button
								type="button"
								className="mb-4 flex items-center gap-1.5 text-sm text-[#6b7280] hover:text-[#111827]"
								onClick={onCancel}
							>
								← Back to proposals
							</button>
							<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2rem] font-normal leading-[1.15] text-[#0a0a0a]">
								Create {authorityLabel} proposal
							</h1>
							<p className="m-0 mt-2 text-sm text-[#6b7280]">
								Authority: <span className="font-semibold text-[#111827]">{authorityLabel}</span> · You will sign this
								proposal immediately after creation.
							</p>
						</>
					)}
				</div>

				<form
					className="rounded-2xl border border-[#e5e7eb] bg-white p-8"
					onSubmit={handleSubmit((data) => void handleSubmitAttempt(data))}
					noValidate
				>
					{isPreviewMode ? (
						<CreateProposalPreview
							title={previewData.title}
							actionType={previewData.actionType}
							seqNo={previewData.seqNo}
							keysToAdd={previewAddingKeys}
							keysToRemove={previewRemovingKeys}
							threshold={previewData.threshold}
							vkTypeId={previewData.vkTypeId}
							newVkHex={previewData.newVkHex}
							operatorsToAdd={previewData.operatorsToAdd.map((r) => r.value.trim()).filter((v) => v.length > 0)}
							operatorIndicesToRemove={previewData.operatorIndicesToRemove
								.map((r) => r.value.trim())
								.filter((v) => v.length > 0)}
							sighashHex={previewSighashHex}
							authorityLabel={authorityLabel}
							currentSigners={multisigConfig?.signers ?? []}
							currentThreshold={multisigConfig?.threshold ?? 0}
							createdProposal={createdProposal}
						/>
					) : (
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
									{authority !== 'alpen_admin' && (
										<ActionTypeCard
											title="Bridge Operator update"
											description="Add operators by key or remove by index."
											selected={actionType === 'operator_set_update'}
											onClick={() =>
												form.setValue('actionType', 'operator_set_update', { shouldValidate: true, shouldDirty: true })
											}
										/>
									)}
								</div>
							</div>

							<div className="max-w-[180px]">
								<LabelWithTooltip
									label="Sequence number"
									tooltip="The monotonically increasing sequence number for this proposal. Must match the expected next value on-chain."
								/>
								<div className="relative">
									<input
										type="number"
										min={0}
										className={numberInputClass}
										{...form.register('seqNo')}
										placeholder={isLoadingSeqNo ? 'Loading…' : '0'}
										disabled={isLoadingSeqNo}
									/>
									{isLoadingSeqNo && (
										<span className="absolute right-8 top-1/2 -translate-y-1/2 text-xs text-[#6b7280]">...</span>
									)}
								</div>
								{!isLoadingSeqNo && nextSeqNo !== null && !formState.dirtyFields.seqNo && (
									<p className="mt-1 text-xs text-[#6b7280]">Auto-detected from chain</p>
								)}
								{formState.errors.seqNo?.message && <p className={fieldErrorClass}>{formState.errors.seqNo.message}</p>}
							</div>

							<div>
								<label className="text-sm font-medium text-[#111827]">Title</label>
								<input
									type="text"
									className={textInputClass}
									{...form.register('title')}
									data-testid="e2e-create-proposal-title"
									placeholder="e.g. Rotate verification key (Q2 2026)"
								/>
								{formState.errors.title?.message && <p className={fieldErrorClass}>{formState.errors.title.message}</p>}
							</div>

							{actionType === 'signer_update' ? (
								<SignerUpdateFormFields
									isLoadingConfig={isLoadingConfig}
									currentSigners={multisigConfig?.signers ?? []}
									currentThreshold={multisigConfig?.threshold ?? 0}
								/>
							) : actionType === 'operator_set_update' ? (
								<OperatorSetUpdateFormFields
									isLoadingConfig={isLoadingConfig}
									currentOperators={currentOperators}
									isLoadingOperators={isLoadingOperators}
								/>
							) : (
								<VkUpdateFormFields currentVk={currentVk} isLoadingCurrentVk={isLoadingCurrentVk} />
							)}
						</div>
					)}

					{error && !isPreviewMode && (
						<div className="mt-6 rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3 text-sm text-[#991b1b]">
							{error}
						</div>
					)}

					<div className="mt-8 border-t border-[#e5e7eb] pt-5">
						<div className="flex items-center justify-end gap-3">
							{isPreviewMode ? (
								createdProposal ? (
									<button
										type="button"
										className="flex items-center gap-2 rounded-lg bg-[#0a0a0a] px-6 py-2.5 text-sm font-medium text-white hover:bg-[#1a1a1a]"
										onClick={onCancel}
									>
										Continue →
									</button>
								) : (
									<>
										{error && <p className="mr-auto text-sm text-[#b91c1c]">{error}</p>}
										<button
											type="submit"
											data-testid="e2e-create-proposal-sign-submit"
											className="flex items-center gap-2 rounded-lg bg-[#0a0a0a] px-5 py-2.5 text-sm font-medium text-white hover:bg-[#1a1a1a] disabled:cursor-not-allowed disabled:bg-[#9ca3af]"
											disabled={isSubmitting || isLoadingConfig || !formState.isValid}
										>
											<PencilWhiteIcon width={14} height={14} className="block shrink-0" />
											{isSubmitting ? 'Signing...' : 'Sign and Create Proposal'}
										</button>
									</>
								)
							) : (
								<>
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
										data-testid="e2e-create-proposal-preview"
										className="flex items-center gap-2 rounded-lg border border-[#0a0a0a] bg-white px-5 py-2.5 text-sm font-medium text-[#111827] hover:bg-[#f8f8fb] disabled:cursor-not-allowed disabled:opacity-50"
										disabled={isSubmitting || isLoadingConfig || !formState.isValid}
										onClick={() => void handlePreviewClick()}
									>
										<EyeGrayIcon width={15} height={15} className="block shrink-0" />
										Preview and Create
									</button>
								</>
							)}
						</div>
					</div>
				</form>
			</div>
			{showReauthModal && (
				<div className="fixed inset-0 z-50 flex items-center justify-center bg-[#111827]/40 p-4">
					<div className="w-full max-w-md rounded-2xl border border-[#e5e7eb] bg-white p-6 shadow-xl">
						<p className="m-0 text-lg font-semibold text-[#111827]">Session expired</p>
						<p className="m-0 mt-2 text-sm text-[#6b7280]">{SESSION_EXPIRED_REAUTH_MESSAGE}</p>
						{reauthError && <p className="m-0 mt-3 text-sm text-[#b91c1c]">{reauthError}</p>}
						<div className="mt-6 flex items-center justify-end gap-3">
							<button
								type="button"
								className="rounded-full border border-[#0a0a0a] bg-white px-5 py-2 text-sm font-medium text-[#0a0a0a] hover:bg-[#f8f8fb] disabled:opacity-50"
								onClick={() => {
									setPendingAction(null)
									setShowReauthModal(false)
								}}
								disabled={isReauthenticating}
							>
								Cancel
							</button>
							<button
								type="button"
								className="rounded-lg bg-[#0a0a0a] px-5 py-2 text-sm font-medium text-white hover:bg-[#1a1a1a] disabled:bg-[#9ca3af]"
								onClick={() => void handleReauthenticateAndRetry()}
								disabled={isReauthenticating}
							>
								{isReauthenticating ? 'Re-authenticating...' : 'Re-authenticate'}
							</button>
						</div>
					</div>
				</div>
			)}
		</FormProvider>
	)
}
