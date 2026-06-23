import { useCallback, useState } from 'react'

export type WalletPanelSection = 'addresses' | 'receive' | 'transactions' | 'send'

type UseWalletPanelStateReturn = {
	isOpen: boolean
	expandedSection: WalletPanelSection | null
	open(section?: WalletPanelSection): void
	close(): void
	setExpandedSection(section: WalletPanelSection | null): void
}

export function useWalletPanelState(): UseWalletPanelStateReturn {
	const [isOpen, setIsOpen] = useState(false)
	const [expandedSection, setExpandedSectionState] = useState<WalletPanelSection | null>(null)

	const open = useCallback((section?: WalletPanelSection) => {
		setIsOpen(true)
		setExpandedSectionState(section ?? null)
	}, [])

	const close = useCallback(() => {
		setIsOpen(false)
		setExpandedSectionState(null)
	}, [])

	const setExpandedSection = useCallback(
		(section: WalletPanelSection | null) => {
			if (!isOpen) return
			setExpandedSectionState(section)
		},
		[isOpen],
	)

	return { isOpen, expandedSection, open, close, setExpandedSection }
}
