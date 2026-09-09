import { useFormContext, useWatch } from 'react-hook-form'
import { SafeHarbourNote } from '@/components/safe-harbour-note'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import type { SafeHarbourStatus } from '@/api/asm-state'
import { useSafeHarbourActionHex } from '../hooks/use-safe-harbour-action-hex'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass } from '../model/create-proposal-form-styles'

type Props = {
	safeHarbour: SafeHarbourStatus | null
	isLoadingSafeHarbour: boolean
}

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
	// Only resolved once the address is well formed: the builder is an IPC round trip, and while the
	// field is invalid its own error is what the signer needs to read.
	const resolvableAddress = errors.newSafeHarbourAddress === undefined ? address : ''
	const { actionHex, error: actionHexError } = useSafeHarbourActionHex(resolvableAddress)
	const { message } = useDeviceSigningMessage(seqNo, actionHex)

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
					aria-invalid={errors.newSafeHarbourAddress !== undefined}
				/>
				{errors.newSafeHarbourAddress?.message ? (
					<p role="alert" className={fieldErrorClass}>
						{errors.newSafeHarbourAddress.message}
					</p>
				) : (
					<p className="mt-1 text-label text-emphasis-soft">
						Must be a taproot address on this network. Every bridge output would sweep here.
					</p>
				)}
			</div>

			<div>
				<p id="safe-harbour-signing-message-label" className="m-0 text-body font-medium text-emphasis">
					Signing message
				</p>
				{actionHexError === null ? (
					<pre
						aria-labelledby="safe-harbour-signing-message-label"
						className="m-0 mt-1.5 overflow-x-auto whitespace-pre rounded-lg border border-[#e5e7eb] bg-bg-surface px-3 py-2.5 font-mono text-body text-emphasis"
						data-testid="e2e-safe-harbour-signing-message"
					>
						{message ?? placeholder}
					</pre>
				) : (
					<p
						role="alert"
						className="mt-1.5 rounded-lg border border-danger-border bg-danger-surface px-3 py-2.5 text-body text-danger-deep"
					>
						The signing message could not be resolved, so there is nothing to compare against your signer. Reconnect and
						try again. ({actionHexError})
					</p>
				)}
				<p className="mt-1 text-label text-emphasis-soft">
					This is exactly what you will see on your signer screen. The destination appears there as a descriptor, not as
					an address — compare that line.
				</p>
			</div>
		</div>
	)
}
