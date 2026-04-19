import type { CSSProperties, ReactNode } from 'react'

type Props = {
	children: ReactNode
}

/** Shared centered layout for signer-facing screens. */
export function ScreenShell({ children }: Props) {
	return (
		<main style={styles.main}>
			<div style={styles.stack}>{children}</div>
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
	} as CSSProperties,
	stack: {
		display: 'flex',
		flexDirection: 'column',
		alignItems: 'center',
		gap: '1.25rem',
		maxWidth: '42rem',
		width: '100%',
	} as CSSProperties,
}
