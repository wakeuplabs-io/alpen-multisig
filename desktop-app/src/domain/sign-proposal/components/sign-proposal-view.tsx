import { CopyClipboardIcon, PencilWhiteIcon, UsbSessionDefaultIcon } from '@/assets/icons'
import type { DecodedAction } from '@/api/signing'
import type { SignSighashResult, WalletVendor } from '@/wallet/types'

type SignProposalViewProps = {
	authorityLabel: string
	proposalIdLabel: string
	proposalTypeLabel: string
	proposalTitle: string
	decodedAction: DecodedAction | null
	sighashHex: string
	signResult: SignSighashResult | null
	isSigning: boolean
	error: string | null
	copyFeedbackVisible: boolean
	walletVendor: WalletVendor
	onCopySighash: () => void
	onSign: () => void
}

function vendorLabel(vendor: WalletVendor): string {
	switch (vendor) {
		case 'trezor':
			return 'Trezor'
		case 'ledger':
			return 'Ledger'
		case 'mnemonic':
			return 'Software Wallet'
		case 'mock':
			return 'Mock'
	}
}

function shortenHex(hex: string) {
	const cleanHex = hex.trim()
	if (cleanHex.length <= 42) {
		return cleanHex
	}
	return `${cleanHex.slice(0, 20)}...${cleanHex.slice(-20)}`
}

function MultisigUpdateDetails({ action }: { action: Extract<DecodedAction, { kind: 'multisig_update' }> }) {
	return (
		<div className="mt-5">
			<p className="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">
				Multisig configuration change
			</p>
			<div className="mt-2 grid gap-3">
				<div className="flex items-center justify-between rounded-lg border border-[#e5e7eb] bg-[#f8fafc] px-3 py-2.5">
					<span className="text-[12px] text-[#6b7280]">New threshold</span>
					<span className="font-mono text-[13px] font-semibold text-[#111827]">{action.newThreshold}</span>
				</div>

				{action.addKeys.length > 0 && (
					<div className="rounded-lg border border-[#bbf7d0] bg-[#f0fdf4] p-3">
						<p className="m-0 text-[10px] font-semibold uppercase tracking-[0.08em] text-[#16a34a]">
							Members to add · {action.addKeys.length}
						</p>
						<ul className="mt-2 flex flex-col gap-1.5 list-none m-0 p-0">
							{action.addKeys.map((key) => (
								<li key={key}>
									<code className="block break-all font-mono text-[11px] leading-5 text-[#166534]">{key}</code>
								</li>
							))}
						</ul>
					</div>
				)}

				{action.removeKeys.length > 0 && (
					<div className="rounded-lg border border-[#fecaca] bg-[#fef2f2] p-3">
						<p className="m-0 text-[10px] font-semibold uppercase tracking-[0.08em] text-[#dc2626]">
							Members to remove · {action.removeKeys.length}
						</p>
						<ul className="mt-2 flex flex-col gap-1.5 list-none m-0 p-0">
							{action.removeKeys.map((key) => (
								<li key={key}>
									<code className="block break-all font-mono text-[11px] leading-5 text-[#991b1b]">{key}</code>
								</li>
							))}
						</ul>
					</div>
				)}

				{action.addKeys.length === 0 && action.removeKeys.length === 0 && (
					<p className="m-0 text-[12px] text-[#9ca3af]">Threshold-only change — no members added or removed.</p>
				)}
			</div>
		</div>
	)
}

function UnknownActionDetails({ rawHex }: { rawHex: string }) {
	return (
		<div className="mt-5">
			<p className="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">Raw action payload</p>
			<div className="mt-2 rounded-lg border border-[#e5e7eb] bg-[#f8fafc] p-3">
				<code className="block break-all font-mono text-[12px] leading-5 text-[#6b7280]">{rawHex}</code>
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
	sighashHex,
	signResult,
	isSigning,
	error,
	copyFeedbackVisible,
	walletVendor,
	onCopySighash,
	onSign,
}: SignProposalViewProps) {
	const label = vendorLabel(walletVendor)
	return (
		<section className="w-full rounded-2xl border border-[#e5e7eb] bg-white p-6 shadow-[0_1px_3px_rgba(15,23,42,0.06)]">
			<div className="rounded-xl border border-[#f1f5f9] bg-[#fcfcff] p-4">
				<p className="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">Proposal</p>
				<h2 className="m-0 mt-2 font-['BIZ_UDPMincho'] text-[31px] leading-[1.12] text-[#0a0a0a]">{proposalTitle}</h2>
				<p className="m-0 mt-2 text-xs text-[#6b7280]">
					{proposalIdLabel} <span className="mx-1.5 text-[#d1d5db]">•</span> {authorityLabel}{' '}
					<span className="mx-1.5 text-[#d1d5db]">•</span> {proposalTypeLabel}
				</p>
			</div>

			{decodedAction === null ? null : decodedAction.kind === 'multisig_update' ? (
				<MultisigUpdateDetails action={decodedAction} />
			) : (
				<UnknownActionDetails rawHex={decodedAction.rawHex} />
			)}

			<div className="mt-5">
				<p className="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">
					SPS-65 Sighash (32 bytes)
				</p>
				<div className="mt-2 flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f8fafc] px-3 py-2.5">
					<code className="block min-w-0 flex-1 break-all font-mono text-[12px] leading-5 text-[#334155]">
						{sighashHex}
					</code>
					<button
						type="button"
						className="group inline-flex items-center gap-1.5 rounded-md border border-[#e5e7eb] bg-white px-2 py-1 text-[11px] font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#374151]"
						onClick={onCopySighash}
					>
						<CopyClipboardIcon width={14} height={14} className={copyFeedbackVisible ? 'copy-address-feedback' : ''} />
						Copy
					</button>
				</div>
			</div>

			<div className="mt-4 rounded-lg border border-[#e5e7eb] bg-[#fafaff] p-3.5">
				<div className="flex items-start gap-2.5">
					<div className="mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-[#ddd8ff] bg-[#f5f3ff] text-[#7c6fcd]">
						<UsbSessionDefaultIcon width={13} height={13} className="text-[#7c6fcd]" />
					</div>
					<div>
						<p className="m-0 text-sm font-medium text-[#111827]">Connect your {label} and confirm on device</p>
						<p className="m-0 mt-1 text-[12px] text-[#6b7280]">
							The sighash above appears on the device screen. Verify it matches before approving.
						</p>
					</div>
				</div>
			</div>

			<div className="mt-5 flex justify-end">
				<button
					type="button"
					data-testid="e2e-sign-proposal-submit"
					className="inline-flex items-center gap-1.5 rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-sm font-medium text-white transition hover:bg-[#232323] disabled:cursor-not-allowed disabled:opacity-60"
					onClick={onSign}
					disabled={isSigning}
				>
					<PencilWhiteIcon width={14} height={14} />
					{isSigning ? `Waiting for ${label}...` : `Sign with ${label}`}
				</button>
			</div>

			{error ? (
				<div className="mt-4 rounded-lg border border-[#fecaca] bg-[#fef2f2] px-3 py-2">
					<p className="m-0 text-[12px] text-[#991b1b]">{error}</p>
				</div>
			) : null}

			{signResult ? (
				<div className="mt-4 rounded-lg border border-[#bbf7d0] bg-[#f0fdf4] p-3">
					<p className="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#166534]">
						Signature collected
					</p>
					<code className="mt-2 block break-all font-mono text-[12px] leading-5 text-[#166534]">
						{shortenHex(signResult.signatureHex)}
					</code>
				</div>
			) : null}
		</section>
	)
}
