import { useCallback } from 'react'
import { useSearchParams } from 'react-router-dom'

export type WalletPanelSection = 'addresses' | 'receive' | 'transactions' | 'send'

const VALID_SECTIONS: ReadonlySet<string> = new Set<WalletPanelSection>([
	'addresses',
	'receive',
	'transactions',
	'send',
])

type UseWalletPanelStateReturn = {
	isOpen: boolean
	expandedSection: WalletPanelSection | null
	open(section?: WalletPanelSection): void
	close(): void
	setExpandedSection(section: WalletPanelSection | null): void
}

export function useWalletPanelState(): UseWalletPanelStateReturn {
	const [params, setSearchParams] = useSearchParams()

	const isOpen = params.get('wallet') === 'open'

	const rawSection = params.get('walletSection')
	const expandedSection: WalletPanelSection | null =
		rawSection !== null && VALID_SECTIONS.has(rawSection) ? (rawSection as WalletPanelSection) : null

	const open = useCallback(
		(section?: WalletPanelSection) => {
			setSearchParams((prev) => {
				const next = new URLSearchParams(prev)
				next.set('wallet', 'open')
				if (section != null) {
					next.set('walletSection', section)
				} else {
					next.delete('walletSection')
				}
				return next
			})
		},
		[setSearchParams],
	)

	const close = useCallback(() => {
		setSearchParams((prev) => {
			const next = new URLSearchParams(prev)
			next.delete('wallet')
			next.delete('walletSection')
			return next
		})
	}, [setSearchParams])

	const setExpandedSection = useCallback(
		(section: WalletPanelSection | null) => {
			if (!isOpen) return
			setSearchParams((prev) => {
				const next = new URLSearchParams(prev)
				if (section != null) {
					next.set('walletSection', section)
				} else {
					next.delete('walletSection')
				}
				return next
			})
		},
		[isOpen, setSearchParams],
	)

	return { isOpen, expandedSection, open, close, setExpandedSection }
}
