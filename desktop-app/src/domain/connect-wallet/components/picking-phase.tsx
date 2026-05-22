import { useCallback, useState, type MouseEvent } from 'react'
import { CheckEmeraldIcon, CopyClipboardIcon } from '@/assets/icons'
import { ScreenBottomBar } from '@/components/screen-bottom-bar'
import type { HwAddressEntry } from '@/wallet/types'

type Props = {
	addresses: HwAddressEntry[]
	selectedIndex: number | null
	onSelectIndex: (index: number) => void
	onBack: () => void
	onUseAddress: () => void
}

const GRID_COLS = 'grid-cols-[44px_minmax(0,140px)_minmax(0,1fr)_32px]'
const ROW_INNER_COLS = 'grid-cols-[44px_minmax(0,140px)_minmax(0,1fr)]'

export function PickingPhase({ addresses, selectedIndex, onSelectIndex, onBack, onUseAddress }: Props) {
	const selectedEntry = addresses.find((entry) => entry.index === selectedIndex) ?? null
	const [copiedIndex, setCopiedIndex] = useState<number | null>(null)

	const continueButtonClassName =
		selectedEntry === null
			? 'rounded-lg border border-[#0a0a0a] bg-[#a3a3a3] px-5 py-2 text-sm font-medium text-white'
			: 'rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-5 py-2 text-sm font-medium text-white transition hover:bg-[#2a2a2a]'

	const copyAddress = useCallback(async (event: MouseEvent<HTMLButtonElement>, entry: HwAddressEntry) => {
		event.stopPropagation()
		event.preventDefault()
		try {
			await navigator.clipboard.writeText(entry.address)
			setCopiedIndex(entry.index)
			window.setTimeout(() => {
				setCopiedIndex((current) => (current === entry.index ? null : current))
			}, 2000)
		} catch {
			// Clipboard may be unavailable (permissions / non-secure context); leave row selection unaffected.
		}
	}, [])

	return (
		<div className="mx-auto w-full max-w-150 pb-28">
			<div className="mb-3 flex items-center justify-between">
				<button
					type="button"
					className="inline-flex items-center gap-1 text-sm text-[#666] transition hover:text-[#0a0a0a]"
					onClick={onBack}
				>
					<span aria-hidden="true">←</span>
					Back
				</button>
				<p className="m-0 w-22 text-right text-[0.68rem] font-medium uppercase tracking-[0.22em] tabular-nums text-[#9ca3af]">
					Step 2 of 4
				</p>
			</div>

			<h1 className="m-0 font-['BIZ_UDPMincho'] text-[2.15rem] font-normal leading-[1.1] tracking-[-0.01em] text-[#0a0a0a]">
				Select your signer address
			</h1>
			<p className="mb-0 mt-3 text-[0.88rem] leading-[1.55] text-[#6b7280]">
				Choose which BIP-84 derived address represents you in this session. Your authority permissions are keyed to this
				address.
			</p>

			<div className="mt-5 overflow-hidden rounded-xl border border-[#e5e7eb] bg-white">
				<div
					className={`grid ${GRID_COLS} items-center border-b border-[#e5e7eb] bg-[#f8f8fb] px-4 py-3 text-[0.66rem] font-medium uppercase tracking-[0.12em] text-[#9ca3af]`}
				>
					<span>#</span>
					<span>Derivation Path</span>
					<span>Address</span>
					<span className="sr-only">Actions</span>
				</div>
				<div className="max-h-[330px] overflow-y-auto">
					{addresses.map((entry) => {
						const isSelected = selectedIndex === entry.index
						const rowBg = isSelected ? 'bg-[#f3f0ff] text-[#5b44c9]' : 'bg-white text-[#334155] hover:bg-[#fafafa]'
						const copyMuted = isSelected
							? 'text-[#5b44c9]/80 hover:text-[#5b44c9]'
							: 'text-[#94a3b8] hover:text-[#64748b]'
						const isCopied = copiedIndex === entry.index
						return (
							<div
								key={entry.index}
								className={`grid ${GRID_COLS} items-center border-b border-[#f3f4f6] transition last:border-b-0 ${rowBg}`}
							>
								<button
									type="button"
									data-testid={`e2e-picking-row-${entry.index}`}
									className={`col-span-3 grid ${ROW_INNER_COLS} items-center px-4 py-3 text-left outline-none transition ${rowBg}`}
									onClick={() => onSelectIndex(entry.index)}
								>
									<span className="font-mono text-xs">#{entry.index}</span>
									<span className="min-w-0 truncate font-mono text-xs">{entry.derivationPath}</span>
									<span className="min-w-0 font-mono text-xs">{entry.address}</span>
								</button>
								<div className="flex h-full items-center justify-end pr-1.5">
									<button
										type="button"
										className={`inline-flex size-7 shrink-0 items-center justify-center rounded-md border transition ${isSelected ? 'border-[#c4b5fd] bg-white/60' : 'border-transparent bg-transparent hover:border-[#e5e7eb] hover:bg-[#f8fafc]'} ${copyMuted} ${isCopied ? 'copy-address-feedback border-emerald-300/80 bg-emerald-50/90 text-emerald-700' : ''}`}
										aria-label={isCopied ? 'Address copied' : 'Copy full address'}
										title={isCopied ? 'Copied' : 'Copy address'}
										onClick={(event) => void copyAddress(event, entry)}
									>
										{isCopied ? (
											<CheckEmeraldIcon width={15} height={15} />
										) : (
											<CopyClipboardIcon width={14} height={14} />
										)}
									</button>
								</div>
							</div>
						)
					})}
				</div>
			</div>

			<ScreenBottomBar
				left={
					<>
						<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#9ca3af]">Signing as</p>
						<p className="m-0 mt-1 truncate font-mono text-xs text-[#334155]">
							{selectedEntry ? selectedEntry.address : 'Select an address from the list above'}
						</p>
					</>
				}
				actions={
					<button
						type="button"
						data-testid="e2e-picking-continue"
						className={continueButtonClassName}
						onClick={onUseAddress}
						disabled={selectedEntry === null}
					>
						Continue →
					</button>
				}
			/>
		</div>
	)
}
