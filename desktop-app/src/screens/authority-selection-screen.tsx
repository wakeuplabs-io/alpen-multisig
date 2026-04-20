import { useEffect, useState, type CSSProperties } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import { tauriCall } from '@/api/tauri-bridge'
import { AUTHORITY_LABELS, type Authority, type AuthorityEligibility } from '@/types'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { ScreenShell } from '@/screens/screen-shell'

const MOCK_ENABLED = import.meta.env.DEV && import.meta.env.VITE_AUTHORITY_SELECTION_MOCK === 'true'
const MOCK_PROFILE = (import.meta.env.VITE_AUTHORITY_SELECTION_MOCK_PROFILE ?? 'eligible').toLowerCase()

function mockAuthoritiesForProfile(): AuthorityEligibility[] {
	if (MOCK_PROFILE === 'empty') {
		return []
	}
	if (MOCK_PROFILE === 'mixed') {
		return [
			{ authority: 'strata_admin', eligible: true },
			{ authority: 'security_council', eligible: false },
		]
	}
	return [{ authority: 'strata_admin', eligible: true }]
}

export function AuthoritySelectionScreen() {
	const navigate = useNavigate()
	const { wallet, selectedAuthority, setSelectedAuthority } = useWalletSession()
	const [checking, setChecking] = useState(false)
	const [eligibility, setEligibility] = useState<AuthorityEligibility[]>([])
	const [selectingAuthority, setSelectingAuthority] = useState<Authority | null>(null)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		if (wallet?.xpubOrFingerprint === undefined) {
			return
		}

		if (MOCK_ENABLED) {
			const mockAuthorities = mockAuthoritiesForProfile()
			setEligibility(mockAuthorities)
			if (
				selectedAuthority !== null &&
				!mockAuthorities.some((item) => item.authority === selectedAuthority && item.eligible)
			) {
				setSelectedAuthority(null)
				void tauriCall('set_selected_authority', { authority: null })
			}
			setError(null)
			setChecking(false)
			return
		}

		setChecking(true)
		setError(null)
		void tauriCall<AuthorityEligibility[]>('list_selectable_authorities', {
			signerPubkeyHex: wallet.xpubOrFingerprint,
		}).then((result) => {
			setChecking(false)
			if (!result.ok) {
				setError(result.error)
				setEligibility([])
				return
			}
			setEligibility(result.data)
			if (
				selectedAuthority !== null &&
				!result.data.some((item) => item.authority === selectedAuthority && item.eligible)
			) {
				setSelectedAuthority(null)
				void tauriCall('set_selected_authority', { authority: null })
			}
		})
	}, [wallet?.xpubOrFingerprint, selectedAuthority, setSelectedAuthority])

	if (wallet === null) {
		return <Navigate to="/" replace />
	}

	async function handleSelectAuthority(authority: Authority) {
		setSelectingAuthority(authority)
		const result = await tauriCall<null>('set_selected_authority', { authority })
		if (!result.ok) {
			setSelectingAuthority(null)
			setError(result.error)
			return
		}
		setSelectedAuthority(authority)
		setSelectingAuthority(null)
		navigate('/dev/sign')
	}

	return (
		<ScreenShell>
			<section style={styles.panel}>
				<h1 style={styles.title}>Select authority</h1>
				<p style={styles.subtitle}>Only authorities where this signer is in the canonical ASM set are available.</p>
				{checking && <p style={styles.helper}>Checking canonical signer membership…</p>}
				{error && <p style={styles.error}>{error}</p>}
				{MOCK_ENABLED && <p style={styles.helper}>Mock mode active (dev only).</p>}
				{!checking && !error && eligibility.length === 0 && (
					<p style={styles.helper}>No selectable authorities for this signer.</p>
				)}
				{eligibility.length > 0 && (
					<div style={styles.list}>
						{eligibility.map((item) => (
							<button
								key={item.authority}
								type="button"
								style={{ ...styles.row, ...(item.eligible ? styles.rowEligible : styles.rowDisabled) }}
								onClick={() => void handleSelectAuthority(item.authority)}
								disabled={!item.eligible || selectingAuthority !== null}
							>
								<span>{AUTHORITY_LABELS[item.authority]}</span>
								<span style={styles.status}>
									{item.eligible
										? selectingAuthority === item.authority
											? 'Selecting...'
											: 'Eligible'
										: 'Not signer'}
								</span>
							</button>
						))}
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
