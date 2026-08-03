import { CopyButton } from '@/components/copy-button'
import type { Proposal } from '@/api/proposals'
import type { WalletVendor } from '@/wallet/types'
import { deviceCopy } from '@/lib/device-copy'
import type { DeviceSigningDisplay } from '@/lib/device-signing-display'
import { DeviceSigningHint } from '@/components/device-signing-hint'
import { CheckCircleEmeraldIcon, UsbTridentIcon } from '@/assets/icons'
import {
	countSignersAfterUpdate,
	normalizeSignerKey,
	VK_PREDICATE_TYPE_LABELS,
	type VkPredicateType,
} from '../model/create-proposal.schema'

type Props = {
	title: string
	actionType: 'signer_update' | 'vk_update' | 'operator_set_update' | 'sequencer_key_update'
	seqNo: string
	keysToAdd: string[]
	keysToRemove: string[]
	threshold: string
	vkTypeId: VkPredicateType
	newVkHex: string
	operatorsToAdd: string[]
	operatorIndicesToRemove: string[]
	newSequencerKeyHex: string
	sighashHex: string | null
	/** What the connected device shows for this action — nothing for software signers. */
	deviceDisplay: DeviceSigningDisplay
	authorityLabel: string
	/** Signer connected in this session — drives the device-specific confirmation copy. */
	walletVendor: WalletVendor
	currentSigners: string[]
	currentThreshold: number
	createdProposal: Proposal | null
}

export function CreateProposalPreview({
	title,
	actionType,
	seqNo,
	keysToAdd,
	keysToRemove,
	threshold,
	vkTypeId,
	newVkHex,
	operatorsToAdd,
	operatorIndicesToRemove,
	newSequencerKeyHex,
	sighashHex,
	deviceDisplay,
	authorityLabel,
	walletVendor,
	currentSigners,
	currentThreshold,
	createdProposal,
}: Props) {
	const signerCopy = deviceCopy(walletVendor)
	const actionTypeLabel =
		actionType === 'signer_update'
			? 'Signer update'
			: actionType === 'operator_set_update'
				? 'Bridge Operator update'
				: actionType === 'sequencer_key_update'
					? 'Sequencer key update'
					: 'Verification key update'

	const removeNorm = new Set(
		keysToRemove
			.map((k) => k.trim())
			.filter((k) => k.length > 0)
			.map(normalizeSignerKey),
	)
	const keysToRemoveRows = keysToRemove.map((k) => ({ value: k }))
	const keysToAddRows = keysToAdd.map((k) => ({ value: k }))
	const afterSignerCount = countSignersAfterUpdate(currentSigners, keysToRemoveRows, keysToAddRows)
	const beforeSignerCount = new Set(currentSigners.map((s) => normalizeSignerKey(s))).size

	const tableRows = [
		...currentSigners.map((s) => ({
			before: s,
			after: removeNorm.has(normalizeSignerKey(s)) ? null : s,
		})),
		...keysToAdd.filter((k) => k.trim().length > 0).map((k) => ({ before: null, after: k })),
	]

	const newThreshold = Number(threshold)
	const thresholdChanged = newThreshold !== currentThreshold || afterSignerCount !== beforeSignerCount

	const signatureHex = createdProposal?.signatures[0]?.signatureHex ?? null

	if (createdProposal !== null) {
		return (
			<div className="flex flex-col gap-6" data-testid="e2e-proposal-signature-success">
				<div className="flex items-start gap-4 rounded-xl border border-[#a7f3d0] bg-[#ecfdf5] p-4">
					<div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-[#a7f3d0] bg-white">
						<CheckCircleEmeraldIcon width={20} height={20} />
					</div>
					<div>
						<p className="m-0 font-semibold text-[#065f46]">Signature collected</p>
						<p className="m-0 mt-0.5 text-body text-[#6b7280]">Your signature has been submitted to the backend.</p>
					</div>
				</div>

				{signatureHex && (
					<div>
						<p className="m-0 mb-2 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
							Your Signature
						</p>
						<div className="flex items-center gap-3 rounded-lg border border-[#e5e7eb] px-4 py-3">
							<span className="min-w-0 flex-1 break-all font-mono text-body text-[#111827]">{signatureHex}</span>
							<CopyButton text={signatureHex} />
						</div>
					</div>
				)}
			</div>
		)
	}

	return (
		<div className="flex flex-col gap-6">
			<div>
				<p className="m-0 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">Proposal</p>
				<p className="m-0 mt-2 font-display text-[1.75rem] font-normal leading-tight text-[#0a0a0a]">{title || '—'}</p>
				<p className="m-0 mt-2 text-body text-[#6b7280]">
					#{seqNo} · {authorityLabel} · {actionTypeLabel}
				</p>
			</div>

			<div className="border-t border-[#e5e7eb]" />

			{actionType === 'operator_set_update' ? (
				<div className="flex flex-col gap-4">
					{operatorsToAdd.length > 0 && (
						<div>
							<p className="m-0 mb-2 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
								Operators to Add ({operatorsToAdd.length})
							</p>
							<div className="flex flex-col gap-1">
								{operatorsToAdd.map((key, i) => (
									<div key={i} className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] px-3 py-2">
										<span className="shrink-0 text-label font-medium text-[#059669]">+</span>
										<span className="min-w-0 flex-1 break-all font-mono text-label text-[#374151]">{key}</span>
									</div>
								))}
							</div>
						</div>
					)}
					{operatorIndicesToRemove.length > 0 && (
						<div>
							<p className="m-0 mb-2 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
								Operator Indices to Remove ({operatorIndicesToRemove.length})
							</p>
							<div className="flex flex-col gap-1">
								{operatorIndicesToRemove.map((idx, i) => (
									<div key={i} className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] px-3 py-2">
										<span className="shrink-0 text-label font-medium text-emphasis-soft">–</span>
										<span className="text-body text-[#374151]">Index {idx}</span>
									</div>
								))}
							</div>
						</div>
					)}
				</div>
			) : actionType === 'sequencer_key_update' ? (
				<div>
					<p className="m-0 mb-2 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
						New Sequencer Public Key
					</p>
					<div className="rounded-lg border border-[#e5e7eb] px-4 py-3">
						<span className="break-all font-mono text-body text-[#111827]">{newSequencerKeyHex.trim() || '—'}</span>
					</div>
				</div>
			) : actionType === 'signer_update' ? (
				<div>
					<p className="m-0 mb-3 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
						Signer Set Change
					</p>
					<div className="overflow-hidden rounded-xl border border-[#e5e7eb]">
						<div className="grid grid-cols-2">
							<div className="border-b border-r border-[#e5e7eb] bg-[#f9fafb] px-4 py-2.5">
								<span className="text-label font-semibold uppercase tracking-widest text-[#9ca3af]">Before</span>
							</div>
							<div className="border-b border-[#e5e7eb] bg-[#f9fafb] px-4 py-2.5">
								<span className="text-label font-semibold uppercase tracking-widest text-[#9ca3af]">After</span>
							</div>
						</div>
						{tableRows.map((row, i) => (
							<div key={i} className="grid grid-cols-2 border-b border-[#e5e7eb] last:border-b-0">
								<div className="border-r border-[#e5e7eb] px-4 py-3">
									<span className="break-all font-mono text-body text-[#374151]">{row.before ?? ''}</span>
								</div>
								<div className="px-4 py-3">
									<span className="break-all font-mono text-body text-[#374151]">{row.after ?? ''}</span>
								</div>
							</div>
						))}
						<div className="grid grid-cols-2">
							<div className="border-r border-[#e5e7eb] px-4 py-3">
								<span className="text-body text-[#9ca3af]">
									Threshold{' '}
									<span className="font-semibold text-[#374151]">
										{currentThreshold} of {beforeSignerCount}
									</span>
								</span>
							</div>
							<div className="px-4 py-3">
								<span className={`text-body ${thresholdChanged ? 'text-[#059669]' : 'text-[#9ca3af]'}`}>
									Threshold{' '}
									<span className="font-semibold">
										{threshold} of {afterSignerCount}
									</span>
								</span>
							</div>
						</div>
					</div>
				</div>
			) : (
				<div>
					<p className="m-0 mb-2 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
						New Verification Key
					</p>
					<div className="flex flex-col gap-2 rounded-lg border border-[#e5e7eb] px-4 py-3">
						<span className="shrink-0 self-start rounded-md bg-highlight-surface-alt px-2 py-0.5 font-mono text-label font-medium text-emphasis">
							{VK_PREDICATE_TYPE_LABELS[vkTypeId]}
						</span>
						{newVkHex.trim().length > 0 && (
							<span className="break-all font-mono text-body text-[#111827]">{newVkHex.trim()}</span>
						)}
					</div>
				</div>
			)}

			<div>
				<p className="m-0 mb-2 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
					SPS-65 Sighash (32 bytes)
				</p>
				<div className="flex items-center gap-3 rounded-lg border border-[#e5e7eb] px-4 py-3">
					<span className="min-w-0 flex-1 break-all font-mono text-body text-[#111827]">{sighashHex ?? '—'}</span>
					{sighashHex && <CopyButton text={sighashHex} />}
				</div>
			</div>

			<div className="rounded-xl border border-accent-border bg-highlight-surface p-4">
				<div className="flex items-start gap-4">
					<div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg border border-[#e5e7eb] bg-white">
						<UsbTridentIcon width={24} height={24} className="text-emphasis-soft" />
					</div>
					<div>
						<p className="m-0 font-semibold text-[#111827]">
							{signerCopy.isHardware ? 'Confirm on device' : 'Confirm before signing'}
						</p>
						<p className="m-0 mt-1 text-body text-[#6b7280]">{signerCopy.verifyHint}</p>
					</div>
				</div>
				{deviceDisplay.kind !== 'none' && (
					<div className="mt-3">
						<DeviceSigningHint display={deviceDisplay} />
					</div>
				)}
			</div>
		</div>
	)
}
