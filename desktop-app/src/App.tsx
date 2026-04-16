import { useState } from 'react'
import { HwWalletConnect } from '@/components/HwWalletConnect'
import { createWalletAdapter } from '@/wallet'
import type { WalletAccountInfo } from '@/wallet/types'

const adapter = createWalletAdapter('trezor')

export default function App() {
	const [wallet, setWallet] = useState<WalletAccountInfo | null>(null)

	return (
		<main style={styles.main}>
			<div style={styles.stack}>
				<HwWalletConnect adapter={adapter} onConnected={setWallet} />
				{wallet && (
					<p style={styles.hint}>
						Active signer: <code>{wallet.derivationPath}</code>
					</p>
				)}
			</div>
		</main>
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
	stack: {
		display: 'flex',
		flexDirection: 'column',
		alignItems: 'center',
		gap: '1.25rem',
		maxWidth: '42rem',
		width: '100%',
	} as React.CSSProperties,
	hint: {
		marginTop: '0',
		color: '#555',
		fontSize: '0.82rem',
	} as React.CSSProperties,
}
