import type { ConnectViewState } from '@/domain/connect-wallet/model/hw-wallet-connect.types'

export function ConnectionIcon({ state }: { state: ConnectViewState }) {
	if (state === 'success') {
		return <SuccessIcon />
	}

	return (
		<svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
			<path d="M8 9.5L5 6.5" stroke="#0A0A0A" strokeWidth="1.8" strokeLinecap="round" />
			<path d="M10.5 7L7.5 4" stroke="#0A0A0A" strokeWidth="1.8" strokeLinecap="round" />
			<path d="M14 14.5L17 17.5" stroke="#0A0A0A" strokeWidth="1.8" strokeLinecap="round" />
			<path d="M16.5 12L19.5 15" stroke="#0A0A0A" strokeWidth="1.8" strokeLinecap="round" />
			<path
				d="M9.7 14.3a3.3 3.3 0 010-4.6l.6-.6a3.3 3.3 0 014.6 0"
				stroke="#0A0A0A"
				strokeWidth="1.8"
				strokeLinecap="round"
			/>
		</svg>
	)
}

export function SuccessIcon() {
	return (
		<svg width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
			<path
				d="M20 7L10.5 16.5L6 12"
				stroke="currentColor"
				strokeWidth="2"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	)
}
