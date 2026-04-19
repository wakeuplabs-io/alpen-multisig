import { useState, type CSSProperties } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { ScreenShell } from '@/screens/screen-shell'
import type { SignSighashResult } from '@/wallet/types'

export function SignPocScreen() {
	const navigate = useNavigate()
	const { wallet, clearSession, adapter } = useWalletSession()
	const [sighashHex, setSighashHex] = useState('')
	const [isSigning, setIsSigning] = useState(false)
	const [signError, setSignError] = useState<string | null>(null)
	const [signResult, setSignResult] = useState<SignSighashResult | null>(null)

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	function handleBackToWallet() {
		clearSession()
		navigate('/', { replace: true })
	}

	async function handleSignWithHw() {
		setIsSigning(true)
		setSignError(null)
		setSignResult(null)
		try {
			const result = await adapter.signSighash(sighashHex.trim())
			setSignResult(result)
		} catch (e) {
			setSignError(String(e))
		} finally {
			setIsSigning(false)
		}
	}

	return (
		<ScreenShell>
			<button type="button" style={styles.backLink} onClick={handleBackToWallet}>
				← Back to wallet connection
			</button>
			<section style={styles.panel}>
				<p style={styles.hint}>
					Active signer: <code>{wallet.derivationPath}</code>
				</p>
				<label style={styles.label}>
					SPS-65 digest (32-byte hex)
					<input
						style={styles.input}
						type="text"
						value={sighashHex}
						onChange={(e) => setSighashHex(e.target.value)}
						placeholder="64 hex chars"
						spellCheck={false}
					/>
				</label>
				<button
					type="button"
					style={styles.signButton}
					onClick={() => void handleSignWithHw()}
					disabled={isSigning || sighashHex.trim().length === 0}
				>
					{isSigning ? 'Waiting for device…' : 'Sign with hardware wallet'}
				</button>
				{signError && <p style={styles.error}>{signError}</p>}
				{signResult && (
					<div style={styles.resultBox}>
						<p style={styles.resultLabel}>Public key</p>
						<code style={styles.mono}>{signResult.publicKeyHex}</code>
						<p style={styles.resultLabel}>Signature</p>
						<code style={styles.mono}>{signResult.signatureHex}</code>
					</div>
				)}
			</section>
		</ScreenShell>
	)
}

const styles = {
	backLink: {
		alignSelf: 'flex-start',
		padding: '0.35rem 0',
		background: 'transparent',
		border: 'none',
		color: '#1d4ed8',
		cursor: 'pointer',
		fontSize: '0.88rem',
	} as CSSProperties,
	hint: {
		marginTop: '0',
		color: '#555',
		fontSize: '0.82rem',
	} as CSSProperties,
	panel: {
		width: '100%',
		background: '#fff',
		borderRadius: 12,
		padding: '1rem',
		border: '1px solid #e5e7eb',
	} as CSSProperties,
	label: {
		display: 'block',
		marginTop: '0.5rem',
		fontSize: '0.85rem',
		color: '#334155',
	} as CSSProperties,
	input: {
		width: '100%',
		marginTop: '0.4rem',
		padding: '0.55rem 0.6rem',
		fontFamily: 'ui-monospace, monospace',
		border: '1px solid #cbd5e1',
		borderRadius: 8,
	} as CSSProperties,
	signButton: {
		marginTop: '0.75rem',
		padding: '0.6rem 1rem',
		background: '#1d4ed8',
		color: '#fff',
		border: 'none',
		borderRadius: 8,
		cursor: 'pointer',
		fontSize: '0.9rem',
	} as CSSProperties,
	error: {
		marginTop: '0.5rem',
		fontSize: '0.85rem',
		color: '#b91c1c',
	} as CSSProperties,
	resultBox: {
		marginTop: '0.85rem',
		display: 'flex',
		flexDirection: 'column',
		gap: '0.4rem',
	} as CSSProperties,
	resultLabel: {
		margin: 0,
		fontSize: '0.78rem',
		color: '#64748b',
	} as CSSProperties,
	mono: {
		fontSize: '0.72rem',
		wordBreak: 'break-all',
		fontFamily: 'ui-monospace, monospace',
		background: '#f8fafc',
		padding: '0.4rem',
		borderRadius: 6,
	} as CSSProperties,
}
