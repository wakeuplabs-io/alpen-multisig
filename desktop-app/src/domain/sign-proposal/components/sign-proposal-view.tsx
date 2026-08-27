import { AlertTriangleIcon, PencilWhiteIcon, UsbSessionDefaultIcon } from '@/assets/icons'
import type { DecodedAction } from '@/api/signing'
import { vkPredicateLabelFromTypeId } from '@/lib/vk-predicate'
import type { DeviceSigningDisplay } from '@/lib/device-signing-display'
import { DeviceSigningHint } from '@/components/device-signing-hint'
import { deviceCopy } from '@/lib/device-copy'
import { multisigUpdateChanges } from '../model/multisig-update-changes'
import type { SignSighashResult, WalletVendor } from '@/wallet/types'

type SignProposalViewProps = {
	authorityLabel: string
	proposalIdLabel: string
	proposalTypeLabel: string
	proposalTitle: string
	decodedAction: DecodedAction | null
	/** On-chain threshold for this authority, or null while unknown — decides whether
	 * the proposal's threshold counts as a change (#423). */
	currentThreshold: number | null
	/** What the connected device displays for this signature (Ledger hash / Trezor text). */
	deviceDisplay: DeviceSigningDisplay
	signResult: SignSighashResult | null
	isSigning: boolean
	error: string | null
	walletVendor: WalletVendor
	onSign: () => void
}

function shortenHex(hex: string) {
	const cleanHex = hex.trim()
	if (cleanHex.length <= 42) {
		return cleanHex
	}
	return `${cleanHex.slice(0, 20)}...${cleanHex.slice(-20)}`
}

function MultisigUpdateDetails({
	action,
	currentThreshold,
}: {
	action: Extract<DecodedAction, { kind: 'multisig_update' }>
	currentThreshold: number | null
}) {
	const changes = multisigUpdateChanges(action, currentThreshold)
	return (
		<div className="mt-5">
			<p className="m-0 text-mono-sm font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">
				Multisig configuration change
			</p>
			<div className="mt-2 grid gap-3">
				{changes.showThreshold && (
					<div className="flex items-center justify-between rounded-lg border border-[#e5e7eb] bg-[#f8fafc] px-3 py-2.5">
						<span className="text-label text-[#6b7280]">New threshold</span>
						<span className="font-mono text-body-sm font-semibold text-[#111827]">{changes.newThreshold}</span>
					</div>
				)}

				{changes.addKeys.length > 0 && (
					<div className="rounded-lg border border-[#bbf7d0] bg-[#f0fdf4] p-3">
						<p className="m-0 text-[10px] font-semibold uppercase tracking-[0.08em] text-[#16a34a]">
							Members to add · {changes.addKeys.length}
						</p>
						<ul className="mt-2 flex flex-col gap-1.5 list-none m-0 p-0">
							{changes.addKeys.map((key) => (
								<li key={key}>
									<code className="block break-all font-mono text-mono-sm leading-5 text-[#166534]">{key}</code>
								</li>
							))}
						</ul>
					</div>
				)}

				{changes.removeKeys.length > 0 && (
					<div className="rounded-lg border border-[#e5e7eb] bg-[#f9fafb] p-3">
						<p className="m-0 text-[10px] font-semibold uppercase tracking-[0.08em] text-emphasis">
							Members to remove · {changes.removeKeys.length}
						</p>
						<ul className="mt-2 flex flex-col gap-1.5 list-none m-0 p-0">
							{changes.removeKeys.map((key) => (
								<li key={key}>
									<code className="block break-all font-mono text-mono-sm leading-5 text-emphasis-soft line-through">
										{key}
									</code>
								</li>
							))}
						</ul>
					</div>
				)}

				{changes.showThreshold && changes.addKeys.length === 0 && changes.removeKeys.length === 0 && (
					<p className="m-0 text-label text-[#9ca3af]">Threshold-only change — no members added or removed.</p>
				)}

				{!changes.hasAnyChange && (
					<p className="m-0 text-label text-[#9ca3af]">
						This proposal does not change the signer set or the threshold.
					</p>
				)}
			</div>
		</div>
	)
}

function VkUpdateDetails({ action }: { action: Extract<DecodedAction, { kind: 'vk_update' }> }) {
	return (
		<div className="mt-5">
			<p className="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">New verification key</p>
			<div className="mt-2 flex flex-col gap-2 rounded-lg border border-[#e5e7eb] px-3 py-2.5">
				<span className="shrink-0 self-start rounded-md bg-highlight-surface-alt px-2 py-0.5 font-mono text-[11px] font-medium text-emphasis">
					{vkPredicateLabelFromTypeId(action.typeId)}
				</span>
				{action.conditionHex.length > 0 && (
					<code className="block break-all font-mono text-[12px] leading-5 text-[#111827]">{action.conditionHex}</code>
				)}
			</div>
		</div>
	)
}

function Defcon1Details() {
	return (
		<div className="mt-5 rounded-lg border border-danger-border bg-danger-surface p-3.5">
			<p className="m-0 flex items-center gap-2 text-body font-semibold text-danger-deep">
				<AlertTriangleIcon width={15} height={15} className="shrink-0 text-danger" />
				Irreversible
			</p>
			<p className="m-0 mt-2 text-label text-danger-deep">
				Approving this authorizes the bridge safe harbour to activate immediately once the proposal is broadcast. The
				action carries no payload — its identity is the signal — and it cannot be cancelled.
			</p>
		</div>
	)
}

function UnknownActionDetails({ rawHex }: { rawHex: string }) {
	return (
		<div className="mt-5">
			<p className="m-0 text-mono-sm font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">Raw action payload</p>
			<div className="mt-2 rounded-lg border border-[#e5e7eb] bg-[#f8fafc] p-3">
				<code className="block break-all font-mono text-label leading-5 text-[#6b7280]">{rawHex}</code>
			</div>
		</div>
	)
}

export function SignProposalView({
	authorityLabel,
	proposalIdLabel,
	proposalTypeLabel,
	proposalTitle,
	decodedAction,
	currentThreshold,
	deviceDisplay,
	signResult,
	isSigning,
	error,
	walletVendor,
	onSign,
}: SignProposalViewProps) {
	const { label, isHardware } = deviceCopy(walletVendor)
	const isDestructive = decodedAction?.kind === 'defcon_1'
	return (
		<section className="w-full rounded-2xl border border-[#e5e7eb] bg-white p-6 shadow-[0_1px_3px_rgba(15,23,42,0.06)]">
			<div className="rounded-xl border border-[#f1f5f9] bg-bg-surface p-4">
				<p className="m-0 text-mono-sm font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">Proposal</p>
				<h2 className="m-0 mt-2 font-display text-[31px] leading-[1.12] text-[#0a0a0a]">{proposalTitle}</h2>
				<p className="m-0 mt-2 text-label text-[#6b7280]">
					{proposalIdLabel} <span className="mx-1.5 text-[#d1d5db]">•</span> {authorityLabel}{' '}
					<span className="mx-1.5 text-[#d1d5db]">•</span> {proposalTypeLabel}
				</p>
			</div>

			{decodedAction === null ? null : decodedAction.kind === 'multisig_update' ? (
				<MultisigUpdateDetails action={decodedAction} currentThreshold={currentThreshold} />
			) : decodedAction.kind === 'vk_update' ? (
				<VkUpdateDetails action={decodedAction} />
			) : decodedAction.kind === 'defcon_1' ? (
				<Defcon1Details />
			) : (
				<UnknownActionDetails rawHex={decodedAction.rawHex} />
			)}

			<div className="mt-4 rounded-lg border border-[#e5e7eb] bg-bg-surface p-3.5">
				<div className="flex items-start gap-2.5">
					<div className="mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-accent-border bg-accent-surface text-accent">
						<UsbSessionDefaultIcon width={13} height={13} className="text-accent" />
					</div>
					<div className="min-w-0 flex-1">
						<p className="m-0 text-body font-medium text-[#111827]">
							{isHardware ? `Connect your ${label} and confirm on device` : `Sign with your ${label}`}
						</p>
						<p className="m-0 mt-1 text-label text-[#6b7280]">Review the action details above before approving.</p>
					</div>
				</div>
				{deviceDisplay.kind !== 'none' && (
					<div className="mt-3">
						<DeviceSigningHint display={deviceDisplay} />
					</div>
				)}
			</div>

			<div className="mt-5 flex justify-end">
				<button
					type="button"
					data-testid="e2e-sign-proposal-submit"
					className={`inline-flex items-center gap-1.5 rounded-lg border px-4 py-2 text-body font-medium text-white transition disabled:cursor-not-allowed disabled:opacity-60 ${
						isDestructive
							? 'border-danger bg-danger hover:bg-danger-strong'
							: 'border-[#0a0a0a] bg-[#0a0a0a] hover:bg-[#232323]'
					}`}
					onClick={onSign}
					disabled={isSigning}
				>
					<PencilWhiteIcon width={14} height={14} />
					{isSigning ? `Waiting for ${label}...` : `Sign with ${label}`}
				</button>
			</div>

			{error ? (
				<div className="mt-4 rounded-lg border border-danger-border bg-danger-surface px-3 py-2">
					<p className="m-0 text-label text-danger-deep">{error}</p>
				</div>
			) : null}

			{signResult ? (
				<div className="mt-4 rounded-lg border border-[#bbf7d0] bg-[#f0fdf4] p-3">
					<p className="m-0 text-mono-sm font-semibold uppercase tracking-[0.08em] text-[#166534]">
						Signature collected
					</p>
					<code className="mt-2 block break-all font-mono text-label leading-5 text-[#166534]">
						{shortenHex(signResult.signatureHex)}
					</code>
				</div>
			) : null}
		</section>
	)
}
