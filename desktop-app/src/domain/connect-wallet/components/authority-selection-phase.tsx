import type { AuthRole } from '@/types'
import { ScreenBottomBar } from '@/components/screen-bottom-bar'

export type AuthorityOption = {
	id: string
	role: AuthRole | null
	label: string
	description: string
	signerSetSource: string
	availabilityLabel: string
	enabled: boolean
}

type Props = {
	selectedAuthorityId: string | null
	options: AuthorityOption[]
	isChecking?: boolean
	onSelectAuthority: (authorityId: string) => void
	onContinueToAuthenticate: () => void
	onBack: () => void
}

export function AuthoritySelectionPhase({
	selectedAuthorityId,
	options,
	isChecking = false,
	onSelectAuthority,
	onContinueToAuthenticate,
	onBack,
}: Props) {
	const selectedOption = options.find((option) => option.id === selectedAuthorityId) ?? null
	const canContinue = !isChecking && selectedOption?.enabled === true
	const continueButtonClassName = canContinue
		? 'rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-5 py-2 text-sm font-medium text-white transition hover:bg-[#2a2a2a]'
		: 'rounded-lg border border-[#0a0a0a] bg-[#a3a3a3] px-5 py-2 text-sm font-medium text-white'

	return (
		<div className="mx-auto w-full max-w-150 pb-28">
			<div className="mb-3 flex items-center justify-between">
				<button
					type="button"
					className="inline-flex items-center gap-1 text-sm text-[#666] transition hover:text-[#0a0a0a]"
					onClick={onBack}
				>
					<span aria-hidden="true">←</span>
					Back
				</button>
				<p className="m-0 w-22 text-right text-[0.68rem] font-medium uppercase tracking-[0.22em] tabular-nums text-[#9ca3af]">
					Step 2 of 3
				</p>
			</div>

			<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2.15rem] font-normal leading-[1.1] tracking-[-0.01em] text-[#0a0a0a]">
				Select authority
			</h1>
			<p className="mb-0 mt-3 text-[0.88rem] leading-[1.55] text-[#6b7280]">
				Choose which governance authority you are acting as. Alpen verifies this signer address before creating your
				session.
			</p>

			<div className="mt-5 space-y-3">
				{options.map((option) => {
					const isSelected = selectedAuthorityId === option.id
					const isDisabled = isChecking || !option.enabled
					const badgeLabel = isChecking && option.role !== null ? 'Checking…' : option.availabilityLabel
					const badgeClassName =
						isChecking && option.role !== null
							? 'border-[#e5e7eb] bg-white text-[#9ca3af]'
							: option.enabled
								? 'border-[#a7f3d0] bg-[#ecfdf5] text-[#059669]'
								: 'border-[#e5e7eb] bg-white text-[#6b7280]'
					return (
						<button
							key={option.id}
							type="button"
							className={`w-full rounded-xl border px-4 py-3 text-left transition ${
								isSelected
									? 'border-[#5b44c9] bg-[#f5f3ff]'
									: isDisabled
										? 'border-[#e5e7eb] bg-[#f8f8fb] opacity-75'
										: 'border-[#e5e7eb] bg-white hover:border-[#d1d5db]'
							}`}
							onClick={() => onSelectAuthority(option.id)}
							disabled={isDisabled}
						>
							<div className="flex items-start justify-between gap-3">
								<div>
									<p className="m-0 text-[1rem] font-medium text-[#111827]">{option.label}</p>
									<p className="m-0 mt-1 text-sm text-[#6b7280]">{option.description}</p>
									<p className="m-0 mt-1 text-xs text-[#9ca3af]">Signer set: {option.signerSetSource}</p>
								</div>
								<span
									className={`inline-flex rounded-full border px-2.5 py-1 text-[0.68rem] font-medium ${badgeClassName}`}
								>
									{badgeLabel}
								</span>
							</div>
						</button>
					)
				})}
			</div>

			<ScreenBottomBar
				left={
					<>
						<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#9ca3af]">Authority</p>
						<p className="m-0 mt-1 text-sm text-[#334155]">
							{selectedOption ? selectedOption.label : 'Select an authority from the list above'}
						</p>
					</>
				}
				actions={
					<button
						type="button"
						data-testid="e2e-authority-select-continue"
						className={continueButtonClassName}
						onClick={onContinueToAuthenticate}
						disabled={!canContinue}
					>
						Select {'->'}
					</button>
				}
			/>
		</div>
	)
}
