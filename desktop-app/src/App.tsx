import { useState } from 'react'
import { createWalletAdapter } from '@/wallet'
import type { WalletAccountInfo, SignTestPayloadResult } from '@/wallet'

type Status = 'idle' | 'connecting' | 'connected' | 'signing' | 'error'

const adapter = createWalletAdapter('trezor')

export default function App() {
	const [status, setStatus] = useState<Status>('idle')
	const [walletInfo, setWalletInfo] = useState<WalletAccountInfo | null>(null)
	const [signResult, setSignResult] = useState<SignTestPayloadResult | null>(null)
	const [error, setError] = useState<string | null>(null)

	async function handleConnect() {
		setStatus('connecting')
		setError(null)
		try {
			const info = await adapter.connect()
			setWalletInfo(info)
			setStatus('connected')
		} catch (e) {
			setError(String(e))
			setStatus('error')
		}
	}

	async function handleSign() {
		setStatus('signing')
		setError(null)
		try {
			const result = await adapter.signTestPayload('hello from alpen multisig')
			setSignResult(result)
			setStatus('connected')
		} catch (e) {
			setError(String(e))
			setStatus('error')
		}
	}

	function handleDisconnect() {
		adapter.disconnect()
		setWalletInfo(null)
		setSignResult(null)
		setError(null)
		setStatus('idle')
	}

	return (
		<main style={styles.main}>
			<section style={styles.card}>
				<h1 style={styles.title}>Trezor POC</h1>
				<p style={styles.subtitle}>Hardware wallet discovery test</p>

				{status === 'idle' || status === 'error' ? (
					<button style={styles.btn} onClick={handleConnect}>
						Connect Trezor
					</button>
				) : status === 'connecting' ? (
					<p style={styles.muted}>Connecting…</p>
				) : (
					<>
						<div style={styles.infoBox}>
							<Row label="Device" value={walletInfo?.deviceLabel} />
							<Row label="Path" value={walletInfo?.derivationPath} />
							<Row label="Address" value={walletInfo?.addressSample} />
							<Row label="Public key" value={walletInfo?.xpubOrFingerprint} />
						</div>

						<div style={{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
							<button style={styles.btn} onClick={handleSign} disabled={status === 'signing'}>
								{status === 'signing' ? 'Signing… (confirm on device)' : 'Sign test payload'}
							</button>
							<button style={{ ...styles.btn, ...styles.btnSecondary }} onClick={handleDisconnect}>
								Disconnect
							</button>
						</div>

						{signResult && (
							<div style={{ ...styles.infoBox, marginTop: '1rem', wordBreak: 'break-all' }}>
								<Row label="Signature" value={signResult.signatureHex} />
								{signResult.note && <p style={{ ...styles.muted, marginTop: '0.5rem' }}>{signResult.note}</p>}
							</div>
						)}
					</>
				)}

				{error && <p style={styles.errorText}>{error}</p>}
			</section>
		</main>
	)
}

function Row({ label, value }: { label: string; value?: string | null }) {
	return (
		<div style={{ display: 'flex', gap: '0.5rem', fontSize: '0.85rem' }}>
			<span style={{ color: '#888', minWidth: 90 }}>{label}</span>
			<span style={{ fontFamily: 'monospace' }}>{value ?? '—'}</span>
		</div>
	)
}

const styles = {
	main: {
		minHeight: '100vh',
		display: 'grid',
		placeItems: 'center',
		fontFamily: 'Inter, system-ui, sans-serif',
		padding: '2rem',
		background: '#f5f5f5',
	} as React.CSSProperties,
	card: {
		background: '#fff',
		borderRadius: 12,
		padding: '2rem',
		width: '100%',
		maxWidth: 480,
		boxShadow: '0 2px 12px rgba(0,0,0,0.08)',
	} as React.CSSProperties,
	title: { margin: 0, fontSize: '1.4rem' } as React.CSSProperties,
	subtitle: { marginTop: '0.25rem', color: '#666', fontSize: '0.9rem' } as React.CSSProperties,
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
		flexDirection: 'column',
		gap: '0.4rem',
	} as React.CSSProperties,
	muted: { color: '#888', fontSize: '0.85rem' } as React.CSSProperties,
	errorText: { color: '#c0392b', fontSize: '0.85rem', marginTop: '0.75rem' } as React.CSSProperties,
}
