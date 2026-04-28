import { useState } from 'react'
import { tauriCall } from '@/api/tauri-bridge'
import type { HwAddressEntry, WalletAccountInfo, WalletAdapter } from '@/wallet/types'

type Phase = 'connect' | 'picking' | 'selected'

type Props = {
	adapter: WalletAdapter
	onConnected: (info: WalletAccountInfo | null) => void
}

export function HwWalletConnect({ adapter, onConnected }: Props) {
	const [phase, setPhase] = useState<Phase>('connect')
	const [loading, setLoading] = useState(false)
	const [account, setAccount] = useState<WalletAccountInfo | null>(null)
	const [addresses, setAddresses] = useState<HwAddressEntry[]>([])
	const [selectedIndex, setSelectedIndex] = useState<number | null>(null)
	const [selectedEntry, setSelectedEntry] = useState<HwAddressEntry | null>(null)
	const [isVerifyingAddress, setIsVerifyingAddress] = useState(false)
	const [verifyMessage, setVerifyMessage] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)

	async function handleConnect() {
		if (!adapter.listAddresses) {
			setError('This wallet does not support address listing.')
			return
		}
		setLoading(true)
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
			setPhase('picking')
		} catch (e) {
			setError(String(e))
		} finally {
			setLoading(false)
		}
	}

	function handleUseAddress() {
		if (selectedIndex === null || !account) return
		const entry = addresses[selectedIndex]
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

	function handleChangeAddress() {
		setVerifyMessage(null)
		setPhase('picking')
	}

	async function handleVerifyOnDevice() {
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

	function handleDisconnect() {
		adapter.disconnect()
		setPhase('connect')
		setAccount(null)
		setAddresses([])
		setSelectedIndex(null)
		setSelectedEntry(null)
		setVerifyMessage(null)
		setError(null)
		onConnected(null)
	}

	return (
		<section style={s.card}>
			<h1 style={s.title}>Connect hardware wallet</h1>
			<p style={s.subtitle}>Trezor · list and select signer</p>
			{phase === 'connect' && (
				<>
					<button style={s.btn} onClick={handleConnect} disabled={loading}>
						{loading ? 'Reading addresses from device…' : 'Connect Trezor'}
					</button>
					{error && <p style={s.errorText}>{error}</p>}
				</>
			)}
			{phase === 'picking' && (
				<>
					<p style={s.helper}>Select the address you want to use as signer.</p>
					<div style={s.list}>
						{addresses.map((entry) => {
							const isSelected = selectedIndex === entry.index
							return (
								<button
									key={entry.index}
									type="button"
									style={{ ...s.row, ...(isSelected ? s.rowSelected : {}) }}
									onClick={() => setSelectedIndex(entry.index)}
								>
									<span style={s.index}>#{entry.index}</span>
									<span style={s.path}>{entry.derivationPath}</span>
									<span style={s.addr}>{truncateAddr(entry.address)}</span>
								</button>
							)
						})}
					</div>
					<div style={s.rowActions}>
						<button
							style={{ ...s.btn, ...(selectedIndex === null ? s.btnDisabled : {}) }}
							onClick={handleUseAddress}
							disabled={selectedIndex === null}
						>
							Use this address
						</button>
						<button style={{ ...s.btn, ...s.btnSecondary }} onClick={handleDisconnect}>
							Disconnect
						</button>
					</div>
				</>
			)}
			{phase === 'selected' && selectedEntry && account && (
				<>
					<div style={s.infoBox}>
						<Label>Device</Label>
						<Mono>{account.deviceLabel}</Mono>
						<Label style={{ marginTop: '0.5rem' }}>Selected path</Label>
						<Mono>{selectedEntry.derivationPath}</Mono>
						<Label style={{ marginTop: '0.5rem' }}>Selected address</Label>
						<Mono>{selectedEntry.address}</Mono>
						<Label style={{ marginTop: '0.5rem' }}>Signer public key (for ASM keys)</Label>
						<Mono>{selectedEntry.publicKeyHex}</Mono>
						<Label style={{ marginTop: '0.5rem' }}>ASM config snippet (copy/paste)</Label>
						<Mono>{buildAsmConfigSnippet(selectedEntry.publicKeyHex)}</Mono>
					</div>
					<div style={s.rowActions}>
						<button style={s.btn} onClick={handleVerifyOnDevice} disabled={isVerifyingAddress}>
							{isVerifyingAddress ? 'Check device…' : 'Verify key/path on device'}
						</button>
						<button style={s.btn} onClick={handleChangeAddress}>
							Change address
						</button>
						<button style={{ ...s.btn, ...s.btnSecondary }} onClick={handleDisconnect}>
							Disconnect
						</button>
					</div>
					{isVerifyingAddress && (
						<p style={s.helper}>
							Check your Trezor screen and confirm the selected path/public key shown by the device.
						</p>
					)}
					{verifyMessage && (
						<p style={verifyMessage.startsWith('Verification failed') ? s.errorText : s.successText}>{verifyMessage}</p>
					)}
				</>
			)}
		</section>
	)
}

function truncateAddr(addr: string): string {
	if (addr.length <= 16) return addr
	return `${addr.slice(0, 8)}…${addr.slice(-6)}`
}

function buildAsmConfigSnippet(publicKeyHex: string): string {
	return `{
  "authorities": {
    "strata_administrator": {
      "config": {
        "keys": ["${publicKeyHex}"],
        "threshold": 1
      }
    }
  }
}`
}

function Label({ children, style }: { children: React.ReactNode; style?: React.CSSProperties }) {
	return <span style={{ ...s.label, ...style }}>{children}</span>
}

function Mono({ children }: { children: React.ReactNode }) {
	return <span style={s.mono}>{children}</span>
}

// ── Styles ────────────────────────────────────────────────────────────────────

const s = {
	card: {
		background: '#fff',
		borderRadius: 12,
		padding: '2rem',
		width: '100%',
		maxWidth: 560,
		boxShadow: '0 2px 12px rgba(0,0,0,0.08)',
		fontFamily: 'Inter, system-ui, sans-serif',
	} as React.CSSProperties,
	title: { margin: 0, fontSize: '1.4rem' } as React.CSSProperties,
	subtitle: { marginTop: '0.25rem', color: '#666', fontSize: '0.85rem' } as React.CSSProperties,
	helper: { marginTop: '1rem', color: '#666', fontSize: '0.85rem' } as React.CSSProperties,
	errorText: { color: '#c0392b', fontSize: '0.85rem', marginTop: '0.75rem' } as React.CSSProperties,
	successText: { color: '#1d7a34', fontSize: '0.85rem', marginTop: '0.75rem' } as React.CSSProperties,
	btn: {
		marginTop: '1rem',
		padding: '0.6rem 1.2rem',
		borderRadius: 8,
		border: '1px solid #222',
		background: '#222',
		color: '#fff',
		cursor: 'pointer',
		fontSize: '0.9rem',
	} as React.CSSProperties,
	btnSecondary: {
		background: '#fff',
		color: '#222',
	} as React.CSSProperties,
	btnDisabled: {
		opacity: 0.45,
		cursor: 'not-allowed',
	} as React.CSSProperties,
	list: {
		marginTop: '0.75rem',
		display: 'flex',
		flexDirection: 'column' as const,
		gap: '0.4rem',
		maxHeight: '20rem',
		overflowY: 'auto' as const,
	} as React.CSSProperties,
	row: {
		width: '100%',
		display: 'grid',
		gridTemplateColumns: '3rem 1fr auto',
		alignItems: 'center',
		gap: '0.5rem',
		padding: '0.55rem 0.65rem',
		borderRadius: 8,
		border: '1px solid #ddd',
		background: '#fff',
		cursor: 'pointer',
		textAlign: 'left' as const,
	} as React.CSSProperties,
	rowSelected: {
		border: '1px solid #2563eb',
		background: '#eff6ff',
	} as React.CSSProperties,
	index: {
		color: '#666',
		fontSize: '0.82rem',
		fontFamily: 'monospace',
	} as React.CSSProperties,
	path: {
		fontFamily: 'monospace',
		fontSize: '0.8rem',
		color: '#444',
	} as React.CSSProperties,
	addr: {
		fontFamily: 'monospace',
		fontSize: '0.82rem',
		color: '#222',
	} as React.CSSProperties,
	rowActions: {
		display: 'flex',
		gap: '0.5rem',
		flexWrap: 'wrap' as const,
	} as React.CSSProperties,
	infoBox: {
		marginTop: '1rem',
		padding: '0.75rem 1rem',
		background: '#f9f9f9',
		borderRadius: 8,
		display: 'flex',
		flexDirection: 'column' as const,
		gap: '0.25rem',
	} as React.CSSProperties,
	label: {
		color: '#888',
		fontSize: '0.78rem',
	} as React.CSSProperties,
	mono: {
		fontFamily: 'monospace',
		fontSize: '0.85rem',
		wordBreak: 'break-all' as const,
	} as React.CSSProperties,
}
