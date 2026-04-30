import { useEffect, useRef, useState } from 'react'
import { tauriCall } from '@/api/tauri-bridge'
import type { HwWalletConnectState } from '@/domain/connect-wallet/model/hw-wallet-connect.types'
import type { WalletAccountInfo, WalletAdapter } from '@/wallet/types'

type Params = {
	adapter: WalletAdapter
	onConnected: (info: WalletAccountInfo | null) => void
}

type HookResult = {
	state: HwWalletConnectState
	actions: {
		connect: () => Promise<void>
		selectAddressIndex: (index: number) => void
		goBackToConnect: () => void
		useAddress: () => void
		changeAddress: () => void
		verifyOnDevice: () => Promise<void>
		disconnect: () => void
	}
}

export function useHwWalletConnect({ adapter, onConnected }: Params): HookResult {
	const [phase, setPhase] = useState<HwWalletConnectState['phase']>('connect')
	const [loading, setLoading] = useState(false)
	const [account, setAccount] = useState<WalletAccountInfo | null>(null)
	const [addresses, setAddresses] = useState<HwWalletConnectState['addresses']>([])
	const [selectedIndex, setSelectedIndex] = useState<number | null>(null)
	const [selectedEntry, setSelectedEntry] = useState<HwWalletConnectState['selectedEntry']>(null)
	const [connectViewState, setConnectViewState] = useState<HwWalletConnectState['connectViewState']>('idle')
	const [isVerifyingAddress, setIsVerifyingAddress] = useState(false)
	const [verifyMessage, setVerifyMessage] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)
	const successTransitionTimeoutRef = useRef<number | null>(null)

	useEffect(() => {
		return () => {
			if (successTransitionTimeoutRef.current !== null) {
				window.clearTimeout(successTransitionTimeoutRef.current)
			}
		}
	}, [])

	async function connect() {
		if (!adapter.listAddresses) {
			setError('This wallet does not support address listing.')
			return
		}

		setLoading(true)
		setConnectViewState('loading')
		setError(null)

		try {
			const info = await adapter.connect()
			const entries = await adapter.listAddresses(20)

			setAccount(info)
			setAddresses(entries)
			setSelectedIndex(null)
			setSelectedEntry(null)
			setVerifyMessage(null)
			onConnected(null)
			setConnectViewState('success')

			successTransitionTimeoutRef.current = window.setTimeout(() => {
				setPhase('picking')
			}, 900)
		} catch (e) {
			setError(String(e))
			setConnectViewState('idle')
		} finally {
			setLoading(false)
		}
	}

	function useAddress() {
		if (selectedIndex === null || !account) return
		const entry = addresses.find((addressEntry) => addressEntry.index === selectedIndex)
		if (!entry) return

		adapter.setDerivationPath?.(entry.derivationPath)
		setSelectedEntry(entry)
		setVerifyMessage(null)
		setPhase('selected')
		onConnected({
			...account,
			derivationPath: entry.derivationPath,
			addressSample: entry.address,
			xpubOrFingerprint: entry.publicKeyHex,
		})
	}

	function goBackToConnect() {
		setPhase('connect')
		setConnectViewState('idle')
		setSelectedIndex(null)
		setSelectedEntry(null)
		setVerifyMessage(null)
		setError(null)
	}

	function changeAddress() {
		setVerifyMessage(null)
		setPhase('picking')
	}

	async function verifyOnDevice() {
		if (!selectedEntry) return

		setIsVerifyingAddress(true)
		setVerifyMessage(null)

		const result = await tauriCall<null>('verify_address_on_device', {
			derivationPath: selectedEntry.derivationPath,
		})

		setIsVerifyingAddress(false)

		if (!result.ok) {
			setVerifyMessage(`Verification failed: ${result.error}`)
			return
		}

		setVerifyMessage('Path/public key confirmed on device.')
	}

	function disconnect() {
		adapter.disconnect()
		setPhase('connect')
		setAccount(null)
		setAddresses([])
		setSelectedIndex(null)
		setSelectedEntry(null)
		setConnectViewState('idle')
		setVerifyMessage(null)
		setError(null)
		onConnected(null)
	}

	return {
		state: {
			phase,
			loading,
			account,
			addresses,
			selectedIndex,
			selectedEntry,
			connectViewState,
			isVerifyingAddress,
			verifyMessage,
			error,
		},
		actions: {
			connect,
			selectAddressIndex: setSelectedIndex,
			goBackToConnect,
			useAddress,
			changeAddress,
			verifyOnDevice,
			disconnect,
		},
	}
}
