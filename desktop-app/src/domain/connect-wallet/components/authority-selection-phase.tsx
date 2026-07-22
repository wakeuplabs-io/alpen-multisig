import type { ReactNode } from 'react'
import type { AuthRole } from '@/types'
import { AuthorityShieldIcon, AuthorityLayersIcon, AuthorityServerIcon } from '@/assets/icons'
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

const AUTHORITY_ICONS: Record<string, ReactNode> = {
	'alpen-administrator': <AuthorityShieldIcon width={22} height={22} />,
	'strata-administrator': <AuthorityLayersIcon width={22} height={22} />,
	'strata-sequencer-manager': <AuthorityServerIcon width={22} height={22} />,
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
		? 'rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-5 py-2 text-body font-medium text-white transition hover:bg-[#2a2a2a]'
		: 'rounded-lg border border-[#0a0a0a] bg-[#a3a3a3] px-5 py-2 text-body font-medium text-white'

	return (
		<div className="mx-auto w-full max-w-150 pb-28">
			<div className="mb-3 flex items-center justify-between">
				<button
					type="button"
					className="inline-flex items-center gap-1 text-body text-[#666] transition hover:text-[#0a0a0a]"
					onClick={onBack}
				>
					<span aria-hidden="true">←</span>
					Back
				</button>
				<p className="m-0 w-22 text-right text-[0.68rem] font-medium uppercase tracking-[0.22em] tabular-nums text-[#9ca3af]">
					Step 2 of 3
				</p>
			</div>

			<h1 className="m-0 font-display text-[2.15rem] font-normal leading-[1.1] tracking-[-0.01em] text-[#0a0a0a]">
				Select multisig
			</h1>
			<p className="mb-0 mt-3 text-[0.88rem] leading-[1.55] text-[#6b7280]">
				Choose which multisig you are acting as. The application will verify that your selected Admin ID is a registered
				signer before advancing.
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

					const iconBgClass = isSelected
						? 'bg-accent-surface text-accent'
						: isDisabled
							? 'bg-[#f3f4f6] text-[#d1d5db]'
							: 'bg-[#f3f4f6] text-[#9ca3af]'

					return (
						<button
							key={option.id}
							type="button"
							className={`w-full rounded-xl border px-5 py-4 text-left transition ${
								isSelected
									? 'border-accent bg-accent-surface'
									: isDisabled
										? 'border-[#e5e7eb] bg-bg-base opacity-75'
										: 'border-[#e5e7eb] bg-white hover:border-[#d1d5db]'
							}`}
							onClick={() => onSelectAuthority(option.id)}
							disabled={isDisabled}
						>
							<div className="flex items-start gap-4">
								<div className={`flex h-11 w-11 shrink-0 items-center justify-center rounded-xl ${iconBgClass}`}>
									{AUTHORITY_ICONS[option.id] ?? <AuthorityShieldIcon width={22} height={22} />}
								</div>
								<div className="min-w-0 flex-1">
									<div className="flex items-start justify-between gap-3">
										<p
											className={`m-0 text-[1rem] font-semibold ${isDisabled && !isSelected ? 'text-[#9ca3af]' : 'text-[#111827]'}`}
										>
											{option.label}
										</p>
										<span
											className={`inline-flex shrink-0 items-center gap-1 rounded-full border px-2.5 py-0.5 text-[0.68rem] font-medium ${badgeClassName}`}
										>
											{option.enabled && !isChecking && (
												<svg
													width="12"
													height="12"
													viewBox="0 0 24 24"
													fill="none"
													stroke="currentColor"
													strokeWidth="2.5"
													strokeLinecap="round"
													aria-hidden="true"
												>
													<path d="M5 12l5 5L20 7" />
												</svg>
											)}
											{badgeLabel}
										</span>
									</div>
									<p
										className={`m-0 mt-1 text-body ${isDisabled && !isSelected ? 'text-[#d1d5db]' : 'text-[#6b7280]'}`}
									>
										{option.description}
									</p>
									<p
										className={`m-0 mt-1.5 text-label ${isDisabled && !isSelected ? 'text-[#d1d5db]' : 'text-[#9ca3af]'}`}
									>
										Signer set · {option.signerSetSource}
									</p>
								</div>
							</div>
						</button>
					)
				})}
			</div>

			<ScreenBottomBar
				left={
					<>
						<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#9ca3af]">Multisig</p>
						<p className="m-0 mt-1 text-body text-[#334155]">
							{selectedOption ? selectedOption.label : 'Select a multisig from the list above'}
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
