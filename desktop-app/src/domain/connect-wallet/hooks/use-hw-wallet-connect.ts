import { useCallback, useEffect, useRef, useState } from 'react'
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
		goBackToConnect: () => void
		verifyOnDevice: () => Promise<void>
		disconnect: () => void
	}
}

export function useHwWalletConnect({ adapter, onConnected }: Params): HookResult {
	const [phase, setPhase] = useState<HwWalletConnectState['phase']>('connect')
	const [loading, setLoading] = useState(false)
	const [account, setAccount] = useState<WalletAccountInfo | null>(null)
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
		setLoading(true)
		setConnectViewState('loading')
		setError(null)

		try {
			const info = await adapter.connect()
			const canonicalEntry = {
				index: 0,
				derivationPath: info.derivationPath,
				address: info.addressSample ?? 'Mnemonic signer',
				publicKeyHex: info.xpubOrFingerprint ?? '',
			}

			setAccount(info)
			setSelectedEntry(canonicalEntry)
			setVerifyMessage(null)
			onConnected({
				...info,
				addressSample: canonicalEntry.address,
				xpubOrFingerprint: canonicalEntry.publicKeyHex,
			})
			setConnectViewState('success')
			successTransitionTimeoutRef.current = window.setTimeout(() => {
				setPhase('selected')
			}, 400)
		} catch (e) {
			setError(String(e))
			setConnectViewState('idle')
		} finally {
			setLoading(false)
		}
	}

	function goBackToConnect() {
		setPhase('connect')
		setConnectViewState('idle')
		setSelectedEntry(null)
		setVerifyMessage(null)
		setError(null)
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

	const disconnect = useCallback(() => {
		adapter.disconnect()
		setPhase('connect')
		setAccount(null)
		setSelectedEntry(null)
		setConnectViewState('idle')
		setVerifyMessage(null)
		setError(null)
		onConnected(null)
	}, [adapter, onConnected])

	return {
		state: {
			phase,
			loading,
			account,
			selectedEntry,
			connectViewState,
			isVerifyingAddress,
			verifyMessage,
			error,
		},
		actions: {
			connect,
			goBackToConnect,
			verifyOnDevice,
			disconnect,
		},
	}
}
