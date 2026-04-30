import type { HwAddressEntry } from '@/wallet/types'
import { truncateAddr } from '@/domain/connect-wallet/utils/hw-wallet-connect-utils'

type Props = {
	addresses: HwAddressEntry[]
	selectedIndex: number | null
	onSelectIndex: (index: number) => void
	onBack: () => void
	onUseAddress: () => void
	onDisconnect: () => void
}

export function PickingPhase({ addresses, selectedIndex, onSelectIndex, onBack, onUseAddress, onDisconnect }: Props) {
	const selectedEntry = addresses.find((entry) => entry.index === selectedIndex) ?? null

	return (
		<div className="w-full max-w-[760px] pb-28">
			<div className="mb-3 flex items-center justify-between">
				<button
					type="button"
					className="inline-flex items-center gap-1 text-sm text-[#666] transition hover:text-[#0a0a0a]"
					onClick={onBack}
				>
					<span aria-hidden="true">←</span>
					Back
				</button>
				<p className="m-0 text-[0.68rem] font-medium uppercase tracking-[0.22em] text-[#9ca3af]">Step 2 of 4</p>
			</div>

			<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2.15rem] font-normal leading-[1.1] tracking-[-0.01em] text-[#0a0a0a]">
				Select your signer address
			</h1>
			<p className="mb-0 mt-3 text-[0.88rem] leading-[1.55] text-[#6b7280]">
				Choose which BIP-86 derived address represents you in this session. Your authority permissions are keyed
				to this address.
			</p>

			<div className="mt-5 overflow-hidden rounded-xl border border-[#e5e7eb] bg-white">
				<div className="grid grid-cols-[56px_220px_1fr] border-b border-[#e5e7eb] bg-[#f8f8fb] px-4 py-3 text-[0.66rem] font-medium uppercase tracking-[0.12em] text-[#9ca3af]">
					<span>#</span>
					<span>Derivation Path</span>
					<span>Address</span>
				</div>
				<div className="max-h-[330px] overflow-y-auto">
					{addresses.map((entry) => {
						const isSelected = selectedIndex === entry.index
						return (
							<button
								key={entry.index}
								type="button"
								className={`grid w-full grid-cols-[56px_220px_1fr] items-center border-b border-[#f3f4f6] px-4 py-3 text-left transition last:border-b-0 ${
									isSelected ? 'bg-[#f3f0ff] text-[#5b44c9]' : 'bg-white text-[#334155] hover:bg-[#fafafa]'
								}`}
								onClick={() => onSelectIndex(entry.index)}
							>
								<span className="font-mono text-xs">#{entry.index}</span>
								<span className="font-mono text-xs">{entry.derivationPath}</span>
								<span className="font-mono text-xs">{truncateAddr(entry.address)}</span>
							</button>
						)
					})}
				</div>
			</div>

			<div className="fixed inset-x-0 bottom-0 z-20 border-t border-[#e5e7eb] bg-[#fbfbfd]/95 px-4 py-3 backdrop-blur-sm">
				<div className="mx-auto flex w-full max-w-[1000px] items-center justify-between rounded-xl border border-[#e5e7eb] bg-[#fbfbfd] px-4 py-3">
					<div>
						<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#9ca3af]">Signing as</p>
						<p className="m-0 mt-1 font-mono text-xs text-[#334155]">
							{selectedEntry ? selectedEntry.address : 'Select an address from the list above'}
						</p>
					</div>
					<div className="flex items-center gap-2">
						<button
							type="button"
							className="rounded-lg border border-[#d1d5db] bg-white px-3 py-2 text-sm font-medium text-[#4b5563] transition hover:bg-[#f7f7f8]"
							onClick={onDisconnect}
						>
							Disconnect
						</button>
						<button
							type="button"
							className="rounded-lg border border-[#0a0a0a] bg-[#a3a3a3] px-5 py-2 text-sm font-medium text-white transition disabled:cursor-not-allowed disabled:opacity-70 enabled:bg-[#0a0a0a]"
							onClick={onUseAddress}
							disabled={selectedEntry === null}
						>
							Continue →
						</button>
					</div>
				</div>
			</div>
		</div>
	)
}
