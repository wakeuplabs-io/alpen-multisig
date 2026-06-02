import { useFormContext, useWatch } from 'react-hook-form'
import type { CurrentVk } from '@/api/asm-state'
import {
	VK_PREDICATE_TYPE_LABELS,
	VK_PREDICATE_TYPES,
	type CreateProposalFormValues,
	type VkPredicateType,
} from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass } from '../model/create-proposal-form-styles'

type Props = {
	currentVk: CurrentVk | null
	isLoadingCurrentVk: boolean
}

type PredicateTypeMeta = {
	label: string
	description: string
	conditionLabel: string | null
	conditionPlaceholder: string | null
}

const PREDICATE_META: Record<VkPredicateType, PredicateTypeMeta> = {
	always_accept: {
		label: 'AlwaysAccept',
		description: 'Any proof passes. Use for testing only.',
		conditionLabel: null,
		conditionPlaceholder: null,
	},
	never_accept: {
		label: 'NeverAccept',
		description: 'All proofs rejected. Disables checkpointing.',
		conditionLabel: null,
		conditionPlaceholder: null,
	},
	bip340_schnorr: {
		label: 'Bip340Schnorr',
		description: 'Schnorr signature check (32-byte x-only pubkey).',
		conditionLabel: 'X-only public key (32 bytes)',
		conditionPlaceholder: '64 hex chars...',
	},
	sp1_groth16: {
		label: 'Sp1Groth16',
		description: 'ZK proof verification (356-byte gnark VK).',
		conditionLabel: 'Groth16 verifying key (356 bytes)',
		conditionPlaceholder: '712 hex chars...',
	},
}

function truncateHex(hex: string, chars = 16): string {
	if (hex.length <= chars * 2) return hex
	return `${hex.slice(0, chars)}…${hex.slice(-chars)}`
}

export function VkUpdateFormFields({ currentVk, isLoadingCurrentVk }: Props) {
	const {
		register,
		control,
		setValue,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	const selectedType = useWatch({ control, name: 'vkTypeId' }) ?? 'always_accept'
	const meta = PREDICATE_META[selectedType]
	const needsCondition = meta.conditionLabel !== null

	function handleTypeSelect(type: VkPredicateType) {
		setValue('vkTypeId', type, { shouldValidate: true, shouldDirty: true })
		setValue('newVkHex', '', { shouldValidate: false })
	}

	return (
		<div className="flex flex-col gap-5">
			{/* Current VK */}
			<div>
				<p className="mb-3 text-sm font-medium text-[#6b7280]">Current verification key</p>
				{isLoadingCurrentVk ? (
					<div className="h-12 animate-pulse rounded-lg bg-[#f3f4f6]" />
				) : currentVk === null ? (
					<div className="rounded-xl border border-[#e5e7eb] bg-[#f9fafb] px-4 py-3 text-sm text-[#9ca3af]">
						Could not load current VK from chain.
					</div>
				) : (
					<div className="rounded-xl border border-[#d97706] bg-[#fffbeb] p-3">
						<div className="flex items-center gap-3 rounded-lg border border-[#e5e7eb] bg-white px-4 py-3">
							<span className="shrink-0 rounded-md bg-[#fef3c7] px-2 py-0.5 font-mono text-xs font-medium text-[#92400e]">
								{currentVk.typeName}
							</span>
							{currentVk.conditionHex.length > 0 && (
								<span
									className="min-w-0 flex-1 break-all font-mono text-sm text-[#374151]"
									title={currentVk.conditionHex}
								>
									{truncateHex(currentVk.conditionHex)}
								</span>
							)}
						</div>
					</div>
				)}
			</div>

			{/* Type selector */}
			<div>
				<p className="mb-3 text-sm font-medium text-[#111827]">New predicate type</p>
				<div className="grid grid-cols-2 gap-2">
					{VK_PREDICATE_TYPES.map((type) => {
						const m = PREDICATE_META[type]
						const isSelected = selectedType === type
						return (
							<button
								key={type}
								type="button"
								onClick={() => handleTypeSelect(type)}
								className={`rounded-xl border px-4 py-3 text-left transition-colors ${
									isSelected
										? 'border-[#d97706] bg-[#fffbeb]'
										: 'border-[#e5e7eb] bg-white hover:border-[#d97706] hover:bg-[#fffbeb]/40'
								}`}
							>
								<p className={`font-mono text-sm font-semibold ${isSelected ? 'text-[#92400e]' : 'text-[#111827]'}`}>
									{m.label}
								</p>
								<p className="mt-0.5 text-xs text-[#6b7280]">{m.description}</p>
							</button>
						)
					})}
				</div>
				{errors.vkTypeId?.message && <p className={fieldErrorClass}>{errors.vkTypeId.message}</p>}
			</div>

			{/* Condition input — only for types that need it */}
			{needsCondition && (
				<div>
					<label className="text-sm font-medium text-[#111827]">{meta.conditionLabel}</label>
					<input
						type="text"
						className={monoInputClass}
						{...register('newVkHex')}
						placeholder={meta.conditionPlaceholder ?? ''}
						spellCheck={false}
					/>
					{errors.newVkHex?.message && <p className={fieldErrorClass}>{errors.newVkHex.message}</p>}
				</div>
			)}

			{/* Hint for no-condition types */}
			{!needsCondition && (
				<div className="rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-4 py-3 text-sm text-[#6b7280]">
					<span className="font-medium text-[#374151]">{VK_PREDICATE_TYPE_LABELS[selectedType]}</span> requires no
					condition bytes — no key input needed.
				</div>
			)}
		</div>
	)
}
