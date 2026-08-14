import { useCallback, useEffect, useRef, useState } from 'react'
import { verifyAddressOnDevice } from '@/api/admin-wallet'
import type { HwDeviceType } from '@/api/admin-wallet'
import { networkFromPath } from '@/domain/admin-wallet/model/network-from-path'
import type { HwWalletConnectState } from '@/domain/connect-wallet/model/hw-wallet-connect.types'
import { matchesDeviceAddress } from '@/lib/admin-id'
import type { WalletAccountInfo, WalletAdapter, WalletKind } from '@/wallet/types'

/** The connected device kind for verify dispatch, or null for software vendors. */
function hwDeviceType(vendor: WalletAdapter['vendor']): HwDeviceType | null {
	return vendor === 'trezor' || vendor === 'ledger' ? vendor : null
}

type Params = {
	adapter: WalletAdapter
	onConnected: (info: WalletAccountInfo | null) => void
}

type HookResult = {
	state: HwWalletConnectState
	actions: {
		/** Defaults to the standard wallet: the one that needs no passphrase. */
		connect: (kind?: WalletKind) => Promise<void>
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

	async function connect(kind: WalletKind = 'standard') {
		setLoading(true)
		setConnectViewState('loading')
		setError(null)

		try {
			const info = await adapter.connect(kind)
			const publicKeyHex = info.publicKeyHex ?? info.xpubOrFingerprint ?? ''
			const canonicalEntry = {
				index: 0,
				derivationPath: info.derivationPath,
				address: info.addressSample ?? 'Mnemonic signer',
				publicKeyHex,
			}

			setAccount(info)
			setSelectedEntry(canonicalEntry)
			setVerifyMessage(null)
			onConnected({
				...info,
				// The Admin ID (PRD 06 §3.b.ii.2). Both connect steps and the wallet panel read
				// this one field, so they cannot drift apart.
				addressSample: canonicalEntry.address,
				// Not the Admin ID, but still the signer's identity to the backend: the nonce
				// signature is checked against the canonical signer set by recovered public key.
				publicKeyHex: canonicalEntry.publicKeyHex,
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
		const deviceType = hwDeviceType(adapter.vendor)
		if (!deviceType) return

		setIsVerifyingAddress(true)
		setVerifyMessage(null)

		// Admin ID is BIP-84 / P2WPKH; dispatch to the connected device on its network.
		const result = await verifyAddressOnDevice({
			derivationPath: selectedEntry.derivationPath,
			deviceType,
			scriptType: 'p2wpkh',
			network: networkFromPath(selectedEntry.derivationPath),
		})

		setIsVerifyingAddress(false)

		if (!result.ok) {
			setVerifyMessage(`Verification failed: ${result.error}`)
			return
		}

		// Compare what the device drew against what this screen shows. Reporting success on
		// the mere fact that the device answered would confirm nothing: a device sitting on a
		// different wallet — a passphrase typed differently after the session was dropped —
		// returns a perfectly valid address for the wrong key.
		if (!matchesDeviceAddress(selectedEntry.address, result.data)) {
			setVerifyMessage(
				`Address mismatch: the device shows ${result.data}, this screen shows ${selectedEntry.address}. Do not use this key — reconnect and check the passphrase you entered on the device.`,
			)
			return
		}

		setVerifyMessage('Address confirmed on device.')
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
