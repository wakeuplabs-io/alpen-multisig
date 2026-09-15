import { useFormContext, useWatch } from 'react-hook-form'
import { SafeHarborNote } from '@/components/safe-harbor-note'
import type { SafeHarborStatus } from '@/api/asm-state'
import { useSafeHarborActionHex } from '../hooks/use-safe-harbor-action-hex'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass } from '../model/create-proposal-form-styles'

type Props = {
	safeHarbor: SafeHarborStatus | null
	isLoadingSafeHarbor: boolean
}

/**
 * Below this length a string cannot be a taproot address yet, so a rejection of it is a *not yet*
 * rather than a verdict. Deliberately loose: its only job is to keep red off a field mid-edit.
 */
const MIN_PLAUSIBLE_ADDRESS_LENGTH = 40

/**
 * The destination the bridge sweeps to, and the exact value the signer's device will display for
 * it.
 *
 * The address is what an operator holds; the BOSD descriptor is what the device shows. The
 * conversion between them is this application's, which makes it the one step of this action where a
 * defect would be invisible — so the descriptor is printed under the address, decoded by the same Rust
 * codec that builds the action, rather than composed in TypeScript or left for the signer to take on
 * trust.
 *
 * The full signing message is not here: nothing is signed on this screen. It is on the preview and
 * the sign view, where the signature is given (V4 Phase 4 §2).
 */
export function SafeHarborAddressFormFields({ safeHarbor, isLoadingSafeHarbor }: Props) {
	const {
		control,
		register,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	const address = useWatch({ control, name: 'newSafeHarborAddress' }) ?? ''
	// Attempted whenever the Zod rules pass. The earlier version read as if that guard filtered
	// invalid addresses; it never could, because the address is validated in Rust and the validator
	// has no opinion about it.
	const { descriptorHex, error: buildError } = useSafeHarborActionHex(
		errors.newSafeHarborAddress === undefined ? address : '',
	)

	// The builder's rejection answers "is this a destination I can use", which is the question the
	// field asks — so it belongs under the field, and only once the signer has typed something long
	// enough to be a whole address. Before that it is a *not yet*, and red is for errors.
	const looksComplete = address.trim().length >= MIN_PLAUSIBLE_ADDRESS_LENGTH
	const addressError = errors.newSafeHarborAddress?.message ?? (looksComplete ? (buildError ?? undefined) : undefined)

	return (
		<div className="flex flex-col gap-5">
			{/* Stated, never enforced: the chain accepts this rotation and discards it, so the signer
			    is told what it will do rather than stopped from doing it. */}
			{safeHarbor?.activated === true && (
				<SafeHarborNote>
					The destination is frozen once safe harbor is active, so this update will be accepted on chain and change
					nothing. It will not report as Enacted.
				</SafeHarborNote>
			)}

			<div>
				<p className="mb-3 text-body font-medium text-[#6b7280]">Current destination</p>
				{isLoadingSafeHarbor ? (
					<div className="h-12 animate-pulse rounded-lg bg-[#f3f4f6]" />
				) : safeHarbor === null ? (
					<div className="rounded-xl border border-[#e5e7eb] bg-[#f9fafb] px-4 py-3 text-body text-[#9ca3af]">
						Could not load the current safe harbor destination from chain.
					</div>
				) : (
					<div className="rounded-xl border border-accent-border bg-highlight-surface p-3">
						<div className="flex flex-col gap-1 rounded-lg border border-[#e5e7eb] bg-white px-4 py-3">
							<span className="break-all font-mono text-body text-[#374151]">{safeHarbor.address}</span>
							<span className="break-all font-mono text-label text-[#9ca3af]">{safeHarbor.addressHex}</span>
						</div>
					</div>
				)}
			</div>

			<div>
				<label htmlFor="safe-harbor-address" className="text-body font-medium text-[#111827]">
					New destination address
				</label>
				<input
					id="safe-harbor-address"
					type="text"
					className={monoInputClass}
					{...register('newSafeHarborAddress')}
					data-testid="e2e-safe-harbor-address"
					placeholder="Taproot address (bc1p… / bcrt1p…)"
					autoComplete="off"
					spellCheck={false}
					aria-invalid={addressError !== undefined}
				/>
				{addressError !== undefined ? (
					<p role="alert" className={fieldErrorClass}>
						{addressError}
					</p>
				) : (
					<p className="mt-1 text-label text-emphasis-soft">
						Must be a taproot address on this network. Every bridge output would sweep here.
					</p>
				)}
				{/* Only for an address the builder accepted: a descriptor under a rejected address would
				    read as a value the device could show. */}
				{addressError === undefined && descriptorHex !== null && (
					<div className="mt-3" data-testid="e2e-safe-harbor-descriptor">
						<p className="m-0 text-body font-medium text-emphasis">Your signer will display</p>
						<code className="mt-1.5 block break-all rounded-lg border border-[#e5e7eb] bg-bg-surface px-3 py-2.5 font-mono text-body text-emphasis">
							{descriptorHex}
						</code>
						<p className="mt-1 text-label text-emphasis-soft">
							The destination appears on the device as this descriptor, not as an address — compare that line.
						</p>
					</div>
				)}
			</div>
		</div>
	)
}
