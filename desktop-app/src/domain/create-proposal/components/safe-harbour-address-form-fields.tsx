import { useFormContext, useWatch } from 'react-hook-form'
import { SafeHarbourNote } from '@/components/safe-harbour-note'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import type { SafeHarbourStatus } from '@/api/asm-state'
import { useSafeHarbourActionHex } from '../hooks/use-safe-harbour-action-hex'
import { SigningMessagePanel } from './signing-message-panel'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass } from '../model/create-proposal-form-styles'

type Props = {
	safeHarbour: SafeHarbourStatus | null
	isLoadingSafeHarbour: boolean
}

/**
 * Below this length a string cannot be a taproot address yet, so a rejection of it is a *not yet*
 * rather than a verdict. Deliberately loose: its only job is to keep red off a field mid-edit.
 */
const MIN_PLAUSIBLE_ADDRESS_LENGTH = 40

/** `seqNo` is a free-text field; the signing message can only be resolved for a real number. */
function parseSeqNo(raw: string | undefined): number | null {
	const trimmed = (raw ?? '').trim()
	return /^\d+$/.test(trimmed) ? Number(trimmed) : null
}

/**
 * The destination the bridge sweeps to, and the exact value the signer's device will display for
 * it.
 *
 * The address is what an operator holds; the BOSD descriptor is what the device shows. The
 * conversion between them is this application's, which makes it the one step of this action where a
 * defect would be invisible — so the descriptor is rendered here, from the same Rust renderer the
 * device signs over, rather than composed in TypeScript or left for the signer to take on trust.
 */
export function SafeHarbourAddressFormFields({ safeHarbour, isLoadingSafeHarbour }: Props) {
	const {
		control,
		register,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	const address = useWatch({ control, name: 'newSafeHarbourAddress' }) ?? ''
	const seqNo = parseSeqNo(useWatch({ control, name: 'seqNo' }))
	// Attempted whenever the Zod rules pass. The earlier version read as if that guard filtered
	// invalid addresses; it never could, because the address is validated in Rust and the validator
	// has no opinion about it.
	const { actionHex, error: buildError } = useSafeHarbourActionHex(
		errors.newSafeHarbourAddress === undefined ? address : '',
	)
	const { message } = useDeviceSigningMessage(seqNo, actionHex)

	// The builder's rejection answers "is this a destination I can use", which is the question the
	// field asks — so it belongs under the field, and only once the signer has typed something long
	// enough to be a whole address. Before that it is a *not yet*, and red is for errors.
	const looksComplete = address.trim().length >= MIN_PLAUSIBLE_ADDRESS_LENGTH
	const addressError = errors.newSafeHarbourAddress?.message ?? (looksComplete ? (buildError ?? undefined) : undefined)

	const placeholder =
		address.trim().length === 0
			? 'Enter a destination address to resolve the signing message.'
			: seqNo === null
				? 'Enter a sequence number to resolve the signing message.'
				: 'Resolving…'

	return (
		<div className="flex flex-col gap-5">
			{/* Stated, never enforced: the chain accepts this rotation and discards it, so the signer
			    is told what it will do rather than stopped from doing it. */}
			{safeHarbour?.activated === true && (
				<SafeHarbourNote>
					The destination is frozen once safe harbour is active, so this update will be accepted on chain and change
					nothing. It will not report as Enacted.
				</SafeHarbourNote>
			)}

			<div>
				<p className="mb-3 text-body font-medium text-[#6b7280]">Current destination</p>
				{isLoadingSafeHarbour ? (
					<div className="h-12 animate-pulse rounded-lg bg-[#f3f4f6]" />
				) : safeHarbour === null ? (
					<div className="rounded-xl border border-[#e5e7eb] bg-[#f9fafb] px-4 py-3 text-body text-[#9ca3af]">
						Could not load the current safe harbour destination from chain.
					</div>
				) : (
					<div className="rounded-xl border border-accent-border bg-highlight-surface p-3">
						<div className="flex flex-col gap-1 rounded-lg border border-[#e5e7eb] bg-white px-4 py-3">
							<span className="break-all font-mono text-body text-[#374151]">{safeHarbour.address}</span>
							<span className="break-all font-mono text-label text-[#9ca3af]">{safeHarbour.addressHex}</span>
						</div>
					</div>
				)}
			</div>

			<div>
				<label htmlFor="safe-harbour-address" className="text-body font-medium text-[#111827]">
					New destination address
				</label>
				<input
					id="safe-harbour-address"
					type="text"
					className={monoInputClass}
					{...register('newSafeHarbourAddress')}
					data-testid="e2e-safe-harbour-address"
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
			</div>

			{/* The panel never carries the address's rejection: an unfinished address leaves it waiting,
			    not shouting. Red here is reserved for a failure that finishing the field cannot fix,
			    which is why it is `null` while the input is still the thing that is wrong. */}
			<SigningMessagePanel
				message={message}
				placeholder={placeholder}
				error={null}
				testId="e2e-safe-harbour-signing-message"
				labelId="safe-harbour-signing-message-label"
				hint="This is exactly what you will see on your signer screen. The destination appears there as a descriptor, not as an address — compare that line."
			/>
		</div>
	)
}
