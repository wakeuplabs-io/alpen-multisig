import { useEffect, useState } from 'react'

import { ORCHESTRATOR_BASE_URL } from '@/api/orchestrator-auth'
import type { ConnectionMode, LocalNodeStatus, NodeConfig } from '../model/node-config.types'

type Props = {
	isOpen: boolean
	config: NodeConfig | null
	localNodeStatus: LocalNodeStatus | null
	isSaving: boolean
	onSave: (draft: NodeConfig) => void | Promise<void>
	onClose: () => void
	onRecheck: (config: NodeConfig) => void
}

type ModeOption = { value: ConnectionMode; label: string; description: string }

const MODE_OPTIONS: ModeOption[] = [
	{
		value: 'local',
		label: 'Local node',
		description: 'Connect to a Strata node running on this machine (default).',
	},
	{
		value: 'trusted',
		label: 'Trusted endpoint',
		description: 'Use the public RPC endpoint hosted at stratabtc.org.',
	},
	{
		value: 'custom',
		label: 'Custom',
		description: 'Enter your own RPC URLs.',
	},
]

function NodeStatusDot({ reachable }: { reachable: boolean | null }) {
	if (reachable === null) return null
	return (
		<span className={`inline-block h-2 w-2 rounded-full ${reachable ? 'bg-emerald-500' : 'bg-red-400'}`} aria-hidden />
	)
}

function StatusLine({ label, reachable, url }: { label: string; reachable: boolean | null; url: string }) {
	if (reachable === null) return null
	return (
		<span className="flex items-center gap-1.5 text-[11px] text-[#6b7280]">
			<NodeStatusDot reachable={reachable} />
			{label} ({url}): {reachable ? 'connected' : 'not found'}
		</span>
	)
}

export function NodeConfigModal({ isOpen, config, localNodeStatus, isSaving, onSave, onClose, onRecheck }: Props) {
	const [draft, setDraft] = useState<NodeConfig>({ mode: 'local' })

	useEffect(() => {
		if (config) setDraft(config)
	}, [config])

	if (!isOpen) return null

	const localUnreachable =
		draft.mode === 'local' &&
		localNodeStatus !== null &&
		!localNodeStatus.strataReachable &&
		!localNodeStatus.btcReachable

	function handleModeChange(mode: ConnectionMode) {
		const next = { ...draft, mode }
		setDraft(next)
		onRecheck(next)
	}

	function handleSubmit(e: React.FormEvent) {
		e.preventDefault()
		onSave(draft)
	}

	return (
		<div
			className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 px-4"
			onClick={(e) => {
				if (e.target === e.currentTarget) onClose()
			}}
		>
			<div className="flex max-h-[90vh] w-full max-w-md flex-col rounded-2xl border border-[#e5e7eb] bg-white shadow-xl">
				<div className="flex items-center justify-between border-b border-[#f3f4f6] px-6 py-4">
					<h2 className="text-[15px] font-semibold text-[#0a0a0a]">Node connection</h2>
					<button
						type="button"
						aria-label="Close"
						onClick={onClose}
						className="rounded-lg p-1 text-[#9ca3af] transition hover:bg-[#f3f4f6] hover:text-[#374151]"
					>
						<svg viewBox="0 0 24 24" fill="none" width={16} height={16} aria-hidden>
							<path d="M6 6l12 12M18 6 6 18" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
						</svg>
					</button>
				</div>

				<form onSubmit={handleSubmit} className="flex min-h-0 flex-1 flex-col">
					<div className="min-h-0 flex-1 space-y-2 overflow-y-auto px-6 py-5">
						{localNodeStatus && (
							<div className="mb-4 rounded-xl border border-[#e5e7eb] p-4">
								<div className="flex items-center justify-between">
									<p className="text-[11px] font-medium uppercase tracking-wide text-[#9ca3af]">Connection status</p>
									<NodeStatusDot reachable={localNodeStatus.strataReachable || localNodeStatus.btcReachable} />
								</div>
								<div className="mt-2 flex flex-col gap-0.5">
									<StatusLine
										label="Strata RPC"
										reachable={localNodeStatus.strataReachable}
										url={localNodeStatus.strataUrl}
									/>
									<StatusLine
										label="Bitcoin RPC"
										reachable={localNodeStatus.btcReachable}
										url={localNodeStatus.btcUrl}
									/>
									<StatusLine
										label="Electrum indexer"
										reachable={localNodeStatus.electrumReachable}
										url={localNodeStatus.electrumUrl}
									/>
									<StatusLine
										label="Orchestrator"
										reachable={localNodeStatus.orchestratorReachable}
										url={localNodeStatus.orchestratorUrl}
									/>
									<button
										type="button"
										onClick={() => onRecheck(draft)}
										className="mt-1 w-fit text-[11px] text-[#6b7280] underline underline-offset-2 hover:text-[#374151]"
									>
										Recheck
									</button>
								</div>
							</div>
						)}

						<p className="mb-3 text-[11px] font-medium uppercase tracking-wide text-[#9ca3af]">Node source</p>

						{MODE_OPTIONS.map((opt) => {
							const isSelected = draft.mode === opt.value
							return (
								<label
									key={opt.value}
									className={`flex cursor-pointer items-start gap-3 rounded-xl border p-4 transition ${
										isSelected ? 'border-[#0a0a0a] bg-[#fafafa]' : 'border-[#e5e7eb] hover:border-[#d1d5db]'
									}`}
								>
									<input
										type="radio"
										name="connectionMode"
										value={opt.value}
										checked={isSelected}
										onChange={() => handleModeChange(opt.value)}
										className="mt-0.5 accent-[#0a0a0a]"
									/>
									<div className="flex-1 min-w-0">
										<span className="text-[13px] font-medium text-[#0a0a0a]">{opt.label}</span>
										<p className="mt-0.5 text-[12px] text-[#6b7280]">{opt.description}</p>
									</div>
								</label>
							)
						})}

						{draft.mode === 'local' && (
							<div className="mt-1 space-y-3 rounded-xl border border-[#e5e7eb] p-4">
								<div>
									<label className="mb-1 block text-[12px] font-medium text-[#374151]">Orchestrator URL</label>
									<input
										type="url"
										placeholder={ORCHESTRATOR_BASE_URL}
										value={draft.customOrchestratorUrl ?? ''}
										onChange={(e) => setDraft((prev) => ({ ...prev, customOrchestratorUrl: e.target.value }))}
										className="w-full rounded-lg border border-[#e5e7eb] px-3 py-2 text-[13px] text-[#0a0a0a] outline-none focus:border-[#0a0a0a] focus:ring-1 focus:ring-[#0a0a0a]"
									/>
									<p className="mt-1 text-[11px] text-[#9ca3af]">
										Backend coordination API. Leave empty to use the default ({ORCHESTRATOR_BASE_URL}).
									</p>
								</div>
							</div>
						)}

						{draft.mode === 'custom' && (
							<div className="mt-1 space-y-3 rounded-xl border border-[#e5e7eb] p-4">
								<div>
									<label className="mb-1 block text-[12px] font-medium text-[#374151]">Strata RPC URL</label>
									<input
										type="url"
										placeholder="http://127.0.0.1:8080"
										value={draft.customStrataRpcUrl ?? ''}
										onChange={(e) => setDraft((prev) => ({ ...prev, customStrataRpcUrl: e.target.value }))}
										className="w-full rounded-lg border border-[#e5e7eb] px-3 py-2 text-[13px] text-[#0a0a0a] outline-none focus:border-[#0a0a0a] focus:ring-1 focus:ring-[#0a0a0a]"
									/>
								</div>
								<div>
									<label className="mb-1 block text-[12px] font-medium text-[#374151]">Bitcoin RPC URL</label>
									<input
										type="url"
										placeholder="http://127.0.0.1:18443"
										value={draft.customBtcRpcUrl ?? ''}
										onChange={(e) => setDraft((prev) => ({ ...prev, customBtcRpcUrl: e.target.value }))}
										className="w-full rounded-lg border border-[#e5e7eb] px-3 py-2 text-[13px] text-[#0a0a0a] outline-none focus:border-[#0a0a0a] focus:ring-1 focus:ring-[#0a0a0a]"
									/>
								</div>
								<div>
									<label className="mb-1 block text-[12px] font-medium text-[#374151]">Electrum URL</label>
									<input
										type="text"
										placeholder="tcp://127.0.0.1:60401"
										value={draft.customElectrumUrl ?? ''}
										onChange={(e) => setDraft((prev) => ({ ...prev, customElectrumUrl: e.target.value }))}
										data-testid="e2e-node-config-electrum-url"
										className="w-full rounded-lg border border-[#e5e7eb] px-3 py-2 text-[13px] text-[#0a0a0a] outline-none focus:border-[#0a0a0a] focus:ring-1 focus:ring-[#0a0a0a]"
									/>
									<p className="mt-1 text-[11px] text-[#9ca3af]">
										Indexer used for Admin Wallet sync (tcp:// or ssl://). Leave empty for the local default.
									</p>
								</div>
								<div className="grid grid-cols-2 gap-3">
									<div>
										<label className="mb-1 block text-[12px] font-medium text-[#374151]">RPC User</label>
										<input
											type="text"
											placeholder="rpcuser"
											value={draft.customBtcRpcUser ?? ''}
											onChange={(e) => setDraft((prev) => ({ ...prev, customBtcRpcUser: e.target.value }))}
											className="w-full rounded-lg border border-[#e5e7eb] px-3 py-2 text-[13px] text-[#0a0a0a] outline-none focus:border-[#0a0a0a] focus:ring-1 focus:ring-[#0a0a0a]"
										/>
									</div>
									<div>
										<label className="mb-1 block text-[12px] font-medium text-[#374151]">RPC Password</label>
										<input
											type="password"
											placeholder="rpcpass"
											value={draft.customBtcRpcPass ?? ''}
											onChange={(e) => setDraft((prev) => ({ ...prev, customBtcRpcPass: e.target.value }))}
											className="w-full rounded-lg border border-[#e5e7eb] px-3 py-2 text-[13px] text-[#0a0a0a] outline-none focus:border-[#0a0a0a] focus:ring-1 focus:ring-[#0a0a0a]"
										/>
									</div>
								</div>
							</div>
						)}

						{draft.mode === 'trusted' && (
							<div className="flex items-start gap-2 rounded-xl border border-amber-200 bg-amber-50 p-3 text-[12px] text-amber-800">
								<svg viewBox="0 0 24 24" fill="none" width={14} height={14} className="mt-0.5 shrink-0" aria-hidden>
									<path d="M12 4L3 19h18L12 4Z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
									<path d="M12 10v4M12 16.5v.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
								</svg>
								<span>
									Trusted Electrum indexer is not yet available. Admin Wallet sync will fail until the public endpoint
									is published. Use Local or Custom mode with your own electrs instance.
								</span>
							</div>
						)}

						{localUnreachable && (
							<div className="flex items-start gap-2 rounded-xl border border-amber-200 bg-amber-50 p-3 text-[12px] text-amber-800">
								<svg viewBox="0 0 24 24" fill="none" width={14} height={14} className="mt-0.5 shrink-0" aria-hidden>
									<path d="M12 4L3 19h18L12 4Z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
									<path d="M12 10v4M12 16.5v.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
								</svg>
								<span>Local node not detected. Start your node or switch to a trusted endpoint to continue.</span>
							</div>
						)}
					</div>

					<div className="border-t border-[#f3f4f6] px-6 py-4">
						<button
							type="submit"
							disabled={isSaving}
							className="w-full rounded-xl bg-[#0a0a0a] py-2.5 text-[13px] font-medium text-white transition hover:bg-[#1f1f1f] disabled:opacity-60"
						>
							{isSaving ? 'Saving…' : 'Save'}
						</button>
					</div>
				</form>
			</div>
		</div>
	)
}
