import type { CSSProperties } from 'react'
import { useNavigate } from 'react-router-dom'
import { HwWalletConnect } from '@/components/HwWalletConnect'
import { useWalletSession } from '@/hooks/use-wallet-session'
import { ScreenShell } from '@/screens/screen-shell'

export function WalletConnectScreen() {
	const navigate = useNavigate()
	const { wallet, setConnectedWallet, adapter } = useWalletSession()

	return (
		<ScreenShell>
			<HwWalletConnect adapter={adapter} onConnected={setConnectedWallet} />
			{wallet !== null && (
				<button type="button" style={styles.continueBtn} onClick={() => navigate('/authorities')}>
					Continue to authority selection
				</button>
			)}
		</ScreenShell>
	)
}

const styles = {
	continueBtn: {
		padding: '0.55rem 1rem',
		background: '#1d4ed8',
		color: '#fff',
		border: 'none',
		borderRadius: 8,
		cursor: 'pointer',
		fontSize: '0.9rem',
	} as CSSProperties,
}
