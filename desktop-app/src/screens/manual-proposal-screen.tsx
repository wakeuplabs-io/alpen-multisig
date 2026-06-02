import { Navigate, useLocation, useNavigate } from 'react-router-dom'
import { LogOutMutedIcon } from '@/assets/icons'
import { ManualImportForm } from '@/domain/manual-proposal/components/manual-import-form'
import { ManualSignCollect } from '@/domain/manual-proposal/components/manual-sign-collect'
import { useManualProposal } from '@/domain/manual-proposal/hooks/use-manual-proposal'
import type { ManualBundleJson } from '@/domain/manual-proposal/model/manual-proposal.types'
import { useSession } from '@/hooks/use-session'
import { ScreenShell } from '@/screens/screen-shell'
import { CopyButton } from '@/components/copy-button'

const STEP_LABELS = ['Import', 'Sign & Collect', 'Broadcast']

type LocationState = { prefill?: ManualBundleJson }

export function ManualProposalScreen() {
	const navigate = useNavigate()
	const location = useLocation()
	const { wallet, sessionTimeLabel, disconnectSession } = useSession()
	const prefill = (location.state as LocationState | null)?.prefill ?? null
	const manual = useManualProposal(prefill)

	async function handleBack() {
		await disconnectSession()
	}

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	const stepIndex = manual.step === 'import' ? 0 : manual.step === 'sign-collect' ? 1 : 2

	return (
		<ScreenShell
			headerContent={
				<>
					<span className="inline-flex items-center gap-2 rounded-full border border-[#e5e7eb] bg-[#f8f8fb] px-3 py-1.25 text-[12px]">
						<span className="font-mono text-[11px] font-medium text-[#111827]">Session · {sessionTimeLabel}</span>
					</span>
					<button
						type="button"
						className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-[12px] font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleBack()}
					>
						<LogOutMutedIcon width={12} height={12} className="block shrink-0" />
						Disconnect
					</button>
				</>
			}
		>
			<div className="mx-auto w-full max-w-190">
				<button
					type="button"
					className="inline-flex items-center gap-1.5 text-sm text-[#6b7280] transition hover:text-[#111827]"
					onClick={() => navigate('/proposals')}
				>
					← Back to proposals
				</button>

				<h1 className="m-0 mt-3 font-['BIZ_UDPMincho'] text-[44px] leading-[1.05] tracking-[-0.01em] text-[#0a0a0a]">
					Manual execution
				</h1>
				<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
					Sign and broadcast a proposal without connecting to the orchestrator.
				</p>

				{/* Step indicator */}
				<div className="mt-6 flex items-center gap-2">
					{STEP_LABELS.map((label, i) => (
						<div key={label} className="flex items-center gap-2">
							<div
								className={`flex items-center gap-1.5 ${i === stepIndex ? 'text-[#111827]' : i < stepIndex ? 'text-[#059669]' : 'text-[#9ca3af]'}`}
							>
								<span
									className={`flex h-5 w-5 items-center justify-center rounded-full text-[11px] font-semibold ${i === stepIndex ? 'bg-[#111827] text-white' : i < stepIndex ? 'bg-[#059669] text-white' : 'bg-[#f3f4f6] text-[#9ca3af]'}`}
								>
									{i < stepIndex ? '✓' : i + 1}
								</span>
								<span className="text-[12px] font-medium">{label}</span>
							</div>
							{i < STEP_LABELS.length - 1 && <span className="text-[#d1d5db]">—</span>}
						</div>
					))}
				</div>

				<div className="mt-6">
					{/* Step 1: Import */}
					{manual.step === 'import' && (
						<div className="rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
							<h2 className="m-0 mb-5 text-[15px] font-semibold text-[#111827]">Enter proposal data</h2>
							<ManualImportForm
								errors={manual.importErrors}
								isValidating={manual.isValidating}
								onLoadJson={(file) => void manual.handleLoadFromJson(file)}
							/>
						</div>
					)}

					{/* Step 2: Sign & Collect */}
					{manual.step === 'sign-collect' && manual.importData && (
						<div className="space-y-4">
							<ManualSignCollect
								importData={manual.importData}
								decodedData={manual.decodedData}
								localSignatures={manual.localSignatures}
								requiredSignatures={manual.requiredSignatures}
								hasQuorum={manual.hasQuorum}
								isSigning={manual.isSigning}
								signError={manual.signError}
								onSign={() => void manual.handleSign()}
								onBroadcast={() => void manual.handleAdvanceToBroadcast()}
							/>
						</div>
					)}

					{/* Step 3: Broadcast */}
					{manual.step === 'broadcast' && (
						<div className="space-y-4">
							{/* Preparing */}
							{manual.broadcastPhase === 'preparing' && (
								<div className="animate-pulse rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
									<div className="h-5 w-48 rounded bg-[#f3f4f6]" />
									<div className="mt-3 h-4 w-64 rounded bg-[#f3f4f6]" />
								</div>
							)}

							{/* Confirming */}
							{manual.broadcastPhase === 'confirming' && manual.broadcastBundle && (
								<div className="rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
									<h2 className="m-0 mb-4 text-[15px] font-semibold text-[#111827]">Confirm broadcast</h2>
									<div className="space-y-3 text-[13px]">
										<div className="flex justify-between">
											<span className="text-[#6b7280]">Commit address</span>
											<span className="font-mono text-[12px] text-[#111827]">
												{manual.broadcastBundle.commitAddress.slice(0, 20)}…
											</span>
										</div>
										<div className="flex justify-between">
											<span className="text-[#6b7280]">Commit amount</span>
											<span className="font-medium text-[#111827]">
												{manual.broadcastBundle.commitAmountSats.toLocaleString()} sats
											</span>
										</div>
										<div className="flex justify-between">
											<span className="text-[#6b7280]">Estimated fee</span>
											<span className="font-medium text-[#111827]">
												{manual.broadcastBundle.estimatedFeeSats.toLocaleString()} sats
											</span>
										</div>
									</div>
									<div className="mt-5 flex gap-3">
										<button
											type="button"
											className="rounded-xl border border-[#e5e7eb] bg-white px-4 py-2.5 text-sm font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
											onClick={manual.handleBackToSignCollect}
										>
											Back
										</button>
										<button
											type="button"
											className="flex-1 rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-sm font-medium text-white transition hover:bg-black"
											onClick={() => void manual.handleConfirmBroadcast()}
										>
											Confirm & broadcast
										</button>
									</div>
								</div>
							)}

							{/* Broadcasting */}
							{manual.broadcastPhase === 'broadcasting' && (
								<div className="rounded-xl border border-[#bfdbfe] bg-[#eff6ff] px-4 py-5 shadow-sm">
									<p className="m-0 text-[13px] font-medium text-[#1d4ed8]">Broadcasting commit + reveal…</p>
									<p className="m-0 mt-1 text-[12px] text-[#3b82f6]">This may take a moment. Do not close the app.</p>
								</div>
							)}

							{/* Done */}
							{manual.broadcastPhase === 'done' && (
								<div className="space-y-4">
									<div className="rounded-xl border border-[#a7f3d0] bg-[#ecfdf5] px-4 py-4 shadow-sm">
										<p className="m-0 text-[13px] font-semibold text-[#065f46]">✓ Broadcast successful</p>
									</div>

									{manual.commitTxid && (
										<div className="rounded-xl border border-[#e5e7eb] bg-white p-5 shadow-sm">
											<p className="m-0 mb-1.5 text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">
												Commit TXID
											</p>
											<div className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
												<span className="min-w-0 flex-1 truncate font-mono text-[12px] text-[#111827]">
													{manual.commitTxid}
												</span>
												<CopyButton text={manual.commitTxid} />
											</div>
										</div>
									)}

									{manual.revealTxid && (
										<div className="rounded-xl border border-[#e5e7eb] bg-white p-5 shadow-sm">
											<p className="m-0 mb-1.5 text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">
												Reveal TXID
											</p>
											<div className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
												<span className="min-w-0 flex-1 truncate font-mono text-[12px] text-[#111827]">
													{manual.revealTxid}
												</span>
												<CopyButton text={manual.revealTxid} />
											</div>
										</div>
									)}

									<button
										type="button"
										className="w-full rounded-xl border border-[#e5e7eb] bg-white px-4 py-2.5 text-sm font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
										onClick={() => navigate('/proposals')}
									>
										Back to proposals
									</button>
								</div>
							)}

							{/* Error */}
							{manual.broadcastPhase === 'error' && (
								<div className="space-y-3">
									<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
										<p className="m-0 text-[13px] font-medium text-[#dc2626]">Broadcast failed</p>
										{manual.broadcastError && (
											<p className="m-0 mt-1 text-[12px] text-[#991b1b]">{manual.broadcastError}</p>
										)}
									</div>
									<button
										type="button"
										className="w-full rounded-xl border border-[#e5e7eb] bg-white px-4 py-2.5 text-sm font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
										onClick={manual.handleBackToSignCollect}
									>
										Back to signing
									</button>
								</div>
							)}
						</div>
					)}
				</div>
			</div>
		</ScreenShell>
	)
}
