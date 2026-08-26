import { useFormContext, useWatch } from 'react-hook-form'
import { AlertTriangleIcon } from '@/assets/icons'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import { useDefcon1ActionHex } from '../hooks/use-defcon-1-action-hex'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputClass } from '../model/create-proposal-form-styles'
import { DEFCON_1_CONFIRMATION } from '../model/validators/defcon-1'

/** `seqNo` is a free-text field; the signing message can only be resolved for a real number. */
function parseSeqNo(raw: string | undefined): number | null {
	const trimmed = (raw ?? '').trim()
	return /^\d+$/.test(trimmed) ? Number(trimmed) : null
}

export function Defcon1FormFields() {
	const {
		control,
		register,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	const actionHex = useDefcon1ActionHex()
	const seqNo = parseSeqNo(useWatch({ control, name: 'seqNo' }))
	// Rendered, never written: the four canonical lines come from the same Rust renderer the
	// device signs over, so they cannot drift from what the signer is about to confirm.
	const { message } = useDeviceSigningMessage(seqNo, actionHex)

	return (
		<div className="flex flex-col gap-5">
			<div className="rounded-xl border border-danger-border bg-danger-surface p-4">
				<p className="m-0 flex items-center gap-2 text-body font-semibold text-danger-deep">
					<AlertTriangleIcon width={16} height={16} className="shrink-0 text-danger" />
					Irreversible
				</p>
				<p className="m-0 mt-2 text-body text-danger-deep">
					Defcon 1 activates the bridge safe harbour immediately. It takes effect in the block that carries it, it
					cannot be cancelled, and there is no way to undo it.
				</p>
			</div>

			<div>
				<label className="text-body font-medium text-[#111827]">Signing message</label>
				<pre
					className="mt-1.5 m-0 whitespace-pre-wrap break-all rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5 font-mono text-body text-[#111827]"
					data-testid="e2e-defcon-1-signing-message"
				>
					{message ?? 'Enter a sequence number to resolve the signing message.'}
				</pre>
				<p className="mt-1 text-label text-[#6b7280]">This is exactly what you will see on your signer screen.</p>
			</div>

			<div>
				<label className="text-body font-medium text-[#111827]">
					Type <span className="font-mono font-semibold text-danger-deep">{DEFCON_1_CONFIRMATION}</span> to confirm
				</label>
				<input
					type="text"
					className={monoInputClass}
					{...register('defconConfirm')}
					data-testid="e2e-defcon-1-confirm"
					placeholder={DEFCON_1_CONFIRMATION}
					autoComplete="off"
					spellCheck={false}
				/>
				{errors.defconConfirm?.message && <p className={fieldErrorClass}>{errors.defconConfirm.message}</p>}
			</div>
		</div>
	)
}
