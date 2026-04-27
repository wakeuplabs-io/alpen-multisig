import { useState, type CSSProperties } from 'react'
import { useNavigate } from 'react-router-dom'
import { HwWalletConnect } from '@/components/HwWalletConnect'
import { useAuthSession } from '@/hooks/use-auth-session'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { ScreenShell } from '@/screens/screen-shell'

export function WalletConnectScreen() {
	const navigate = useNavigate()
	const { wallet, setConnectedWallet, adapter } = useWalletSession()
	const { isAuthenticated, authenticate, selectedRole, setSelectedRole } = useAuthSession()
	const [authError, setAuthError] = useState<string | null>(null)
	const [authOkMessage, setAuthOkMessage] = useState<string | null>(null)
	const [isAuthenticating, setIsAuthenticating] = useState(false)

	async function handleAuthenticate() {
		setAuthError(null)
		setAuthOkMessage(null)
		setIsAuthenticating(true)
		try {
			await authenticate((challengeHex: string) => adapter.signSighash(challengeHex))
			setAuthOkMessage('OK: autenticado correctamente.')
		} catch (e) {
			const message = String(e)
			if (message.toLowerCase().includes('not a member')) {
				setAuthError('No tiene permisos para el rol seleccionado.')
			} else {
				setAuthError(message)
			}
		} finally {
			setIsAuthenticating(false)
		}
	}

	return (
		<ScreenShell>
			<HwWalletConnect adapter={adapter} onConnected={setConnectedWallet} />
			{wallet !== null && (
				<>
					<label style={styles.label}>
						Authority role
						<select
							style={styles.select}
							value={selectedRole}
							onChange={(e) => {
								setSelectedRole(e.target.value as typeof selectedRole)
								setAuthError(null)
								setAuthOkMessage(null)
							}}
						>
							<option value="strata_administrator">Strata Administrator</option>
							<option value="strata_sequencer_manager">Strata Sequencer Manager</option>
						</select>
					</label>
					<button
						type="button"
						style={styles.continueBtn}
						onClick={() => void handleAuthenticate()}
						disabled={isAuthenticating}
					>
						{isAuthenticating ? 'Authenticating…' : 'Authenticate signer'}
					</button>
					{authOkMessage && <p style={styles.success}>{authOkMessage}</p>}
					{authError && <p style={styles.error}>{authError}</p>}
					<button
						type="button"
						style={{ ...styles.continueBtn, ...(isAuthenticated ? {} : styles.disabled) }}
						onClick={() => navigate('/dev/sign')}
						disabled={!isAuthenticated}
					>
						Continue to SPS-65 signing (PoC)
					</button>
				</>
			)}
		</ScreenShell>
	)
}

const styles = {
	label: {
		display: 'block',
		marginTop: '0.75rem',
		fontSize: '0.85rem',
		color: '#334155',
	} as CSSProperties,
	select: {
		display: 'block',
		marginTop: '0.35rem',
		padding: '0.45rem 0.5rem',
		borderRadius: 8,
		border: '1px solid #cbd5e1',
	} as CSSProperties,
	continueBtn: {
		display: 'block',
		marginTop: '0.75rem',
		padding: '0.55rem 1rem',
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
	success: {
		marginTop: '0.5rem',
		fontSize: '0.85rem',
		color: '#166534',
	} as CSSProperties,
	disabled: {
		opacity: 0.5,
		cursor: 'not-allowed',
	} as CSSProperties,
}
