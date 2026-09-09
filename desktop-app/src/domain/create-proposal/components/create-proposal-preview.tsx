import { CopyButton } from '@/components/copy-button'
import type { Proposal } from '@/api/proposals'
import type { WalletVendor } from '@/wallet/types'
import { deviceCopy } from '@/lib/device-copy'
import type { DeviceSigningDisplay } from '@/lib/device-signing-display'
import { DeviceSigningHint } from '@/components/device-signing-hint'
import { CheckCircleEmeraldIcon, UsbTridentIcon } from '@/assets/icons'
import { DefconCallout } from '@/components/defcon-callout'
import { SignerSetChangeTable } from '@/domain/signer-set-change/components/signer-set-change-table'
import { buildSignerSetChange } from '@/domain/signer-set-change/model/build-signer-set-change'
import { actionTypeTitle } from '../model/action-type-config'
import { isSignerUpdateActionType } from '../model/action-type-predicates'
import type { ActionType } from '../model/create-proposal.types'
import { VK_PREDICATE_TYPE_LABELS, type VkPredicateType } from '../model/create-proposal.schema'
import { removesCurrentMembers } from '../model/validators/signer-update'

type Props = {
	title: string
	actionType: ActionType
	seqNo: string
	keysToAdd: string[]
	keysToRemove: string[]
	threshold: string
	vkTypeId: VkPredicateType
	newVkHex: string
	operatorsToAdd: string[]
	operatorIndicesToRemove: string[]
	newSequencerKeyHex: string
	newSafeHarbourAddress: string
	/** The bridge's destination this rotation replaces, and whether it is already frozen. */
	currentSafeHarbour: { address: string; addressHex: string; activated: boolean } | null
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
	newSafeHarbourAddress,
	currentSafeHarbour,
	deviceDisplay,
	authorityLabel,
	walletVendor,
	currentSigners,
	currentThreshold,
	createdProposal,
}: Props) {
	const signerCopy = deviceCopy(walletVendor)
	const actionTypeLabel = actionTypeTitle(actionType)
	const keysToRemoveRows = keysToRemove.map((k) => ({ value: k }))
	const signerSetChange = buildSignerSetChange({
		signers: currentSigners,
		threshold: currentThreshold,
		addKeys: keysToAdd.filter((key) => key.trim().length > 0),
		removeKeys: keysToRemove.filter((key) => key.trim().length > 0),
		newThreshold: Number(threshold),
		isEnacted: false,
	})

	// AC 11: states the consequence, does not block it — Constraint 5 keeps this off the danger
	// palette. Only meaningful for the council's own rotation: an administrator signer update
	// removing an administrator signer has no bearing on who may authorize the Council's actions.
	const showsCouncilMembershipLossCallout =
		actionType === 'council_signer_update' && removesCurrentMembers(currentSigners, keysToRemoveRows)

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

			{actionType === 'defcon_1' || actionType === 'defcon_3' ? (
				<DefconCallout level={actionType} />
			) : actionType === 'operator_set_update' ? (
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
			) : actionType === 'safe_harbour_address_update' ? (
				<div>
					<p className="m-0 mb-2 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
						New Sweep Destination
					</p>
					<div className="flex flex-col gap-1 rounded-lg border border-[#e5e7eb] px-4 py-3">
						<span className="break-all font-mono text-body text-[#111827]">{newSafeHarbourAddress.trim() || '—'}</span>
						{currentSafeHarbour !== null && (
							<span className="text-label text-[#6b7280]">
								Replacing <span className="font-mono">{currentSafeHarbour.address}</span>
							</span>
						)}
					</div>
					{currentSafeHarbour?.activated === true && (
						<div className="mt-4 rounded-xl border border-accent-border bg-highlight-surface p-4">
							<p className="m-0 text-body font-semibold text-[#111827]">Safe harbour is already active</p>
							<p className="m-0 mt-2 text-body text-[#6b7280]">
								The destination is frozen once safe harbour is active. This update will be accepted on chain and change
								nothing, and it will not report as Enacted.
							</p>
						</div>
					)}
				</div>
			) : isSignerUpdateActionType(actionType) ? (
				<div>
					<p className="m-0 mb-3 text-label font-semibold uppercase tracking-[0.12em] text-[#9ca3af]">
						Signer Set Change
					</p>
					<div className="overflow-hidden rounded-xl border border-[#e5e7eb]">
						<SignerSetChangeTable change={signerSetChange} />
					</div>
					{showsCouncilMembershipLossCallout && (
						<div className="mt-4 rounded-xl border border-accent-border bg-highlight-surface p-4">
							<p className="m-0 text-body font-semibold text-[#111827]">Removes a current Council member</p>
							<p className="m-0 mt-2 text-body text-[#6b7280]">
								This rotation removes at least one current Security Council member. Once enacted, that signer can no
								longer authorize Council actions, including Defcon 1 and Defcon 3.
							</p>
						</div>
					)}
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
