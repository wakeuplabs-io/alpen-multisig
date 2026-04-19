import { useEffect, useState, type CSSProperties } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import { tauriCall } from '@/api/tauri-bridge'
import { AUTHORITY_LABELS, type Authority } from '@/types'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { ScreenShell } from '@/screens/screen-shell'

type AuthorityEligibility = {
	authority: Authority
	eligible: boolean
}

export function AuthoritySelectionScreen() {
	const navigate = useNavigate()
	const { wallet, selectedAuthority, setSelectedAuthority } = useWalletSession()
	const [checking, setChecking] = useState(false)
	const [eligibility, setEligibility] = useState<AuthorityEligibility | null>(null)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		if (wallet?.xpubOrFingerprint === undefined) {
			return
		}

		setChecking(true)
		setError(null)
		void tauriCall<AuthorityEligibility>('check_strata_admin_signer', {
			signerPubkeyHex: wallet.xpubOrFingerprint,
		}).then((result) => {
			setChecking(false)
			if (!result.ok) {
				setError(result.error)
				setEligibility(null)
				return
			}
			setEligibility(result.data)
			if (!result.data.eligible && selectedAuthority === result.data.authority) {
				setSelectedAuthority(null)
			}
		})
	}, [wallet?.xpubOrFingerprint, selectedAuthority, setSelectedAuthority])

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	function handleSelectAuthority(authority: Authority) {
		setSelectedAuthority(authority)
		navigate('/dev/sign')
	}

	return (
		<ScreenShell>
			<section style={styles.panel}>
				<h1 style={styles.title}>Select authority</h1>
				<p style={styles.subtitle}>Only authorities where this signer is in the canonical ASM set are available.</p>
				{checking && <p style={styles.helper}>Checking canonical signer membership…</p>}
				{error && <p style={styles.error}>{error}</p>}
				{eligibility !== null && (
					<div style={styles.list}>
						<button
							type="button"
							style={{ ...styles.row, ...(eligibility.eligible ? styles.rowEligible : styles.rowDisabled) }}
							onClick={() => handleSelectAuthority(eligibility.authority)}
							disabled={!eligibility.eligible}
						>
							<span>{AUTHORITY_LABELS[eligibility.authority]}</span>
							<span style={styles.status}>{eligibility.eligible ? 'Eligible' : 'Not signer'}</span>
						</button>
					</div>
				)}
			</section>
		</ScreenShell>
	)
}

const styles = {
	panel: {
		width: '100%',
		maxWidth: '42rem',
		background: '#fff',
		borderRadius: 12,
		padding: '1.25rem',
		border: '1px solid #e5e7eb',
	} as CSSProperties,
	title: {
		marginTop: 0,
		marginBottom: '0.3rem',
	} as CSSProperties,
	subtitle: {
		marginTop: 0,
		color: '#555',
		fontSize: '0.9rem',
	} as CSSProperties,
	helper: {
		color: '#334155',
		fontSize: '0.85rem',
	} as CSSProperties,
	error: {
		color: '#b91c1c',
		fontSize: '0.85rem',
	} as CSSProperties,
	list: {
		marginTop: '0.8rem',
		display: 'flex',
		flexDirection: 'column',
		gap: '0.5rem',
	} as CSSProperties,
	row: {
		display: 'flex',
		width: '100%',
		justifyContent: 'space-between',
		padding: '0.75rem 0.85rem',
		border: '1px solid #cbd5e1',
		borderRadius: 8,
		background: '#fff',
		fontSize: '0.9rem',
		cursor: 'pointer',
	} as CSSProperties,
	rowEligible: {
		border: '1px solid #1d4ed8',
	} as CSSProperties,
	rowDisabled: {
		opacity: 0.6,
		cursor: 'not-allowed',
	} as CSSProperties,
	status: {
		fontSize: '0.8rem',
		color: '#64748b',
	} as CSSProperties,
}
