import { useState } from 'react'
import type { WalletAccountInfo, WalletAdapter } from '@/wallet/types'

type Props = {
	adapter: WalletAdapter
	onConnected: (info: WalletAccountInfo | null) => void
}

export function HwWalletConnect({ adapter, onConnected }: Props) {
	const [loading, setLoading] = useState(false)
	const [account, setAccount] = useState<WalletAccountInfo | null>(null)
	const [error, setError] = useState<string | null>(null)

	async function handleConnect() {
		setLoading(true)
		setError(null)
		try {
			const info = await adapter.connect()
			setAccount(info)
			onConnected(info)
		} catch (e) {
			setError(String(e))
		} finally {
			setLoading(false)
		}
	}

	function handleDisconnect() {
		adapter.disconnect()
		setAccount(null)
		setError(null)
		onConnected(null)
	}

	return (
		<section style={s.card}>
			<h1 style={s.title}>Connect hardware wallet</h1>
			<p style={s.subtitle}>Trezor · basic connect/sign flow</p>
			{!account && (
				<>
					<button style={s.btn} onClick={handleConnect} disabled={loading}>
						{loading ? 'Connecting…' : 'Connect Trezor'}
					</button>
					{error && <p style={s.errorText}>{error}</p>}
				</>
			)}
			{account && (
				<>
					<div style={s.infoBox}>
						<Label>Device</Label>
						<Mono>{account.deviceLabel}</Mono>
						<Label style={{ marginTop: '0.5rem' }}>Derivation path</Label>
						<Mono>{account.derivationPath}</Mono>
						{account.addressSample && (
							<>
								<Label style={{ marginTop: '0.5rem' }}>Address</Label>
								<Mono>{account.addressSample}</Mono>
							</>
						)}
					</div>
					<button style={{ ...s.btn, ...s.btnSecondary }} onClick={handleDisconnect}>
						Disconnect
					</button>
				</>
			)}
		</section>
	)
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
	errorText: { color: '#c0392b', fontSize: '0.85rem', marginTop: '0.75rem' } as React.CSSProperties,
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
