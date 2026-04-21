import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { WalletSessionProvider } from '@/contexts/wallet-session-provider'
import { SignPocScreen } from '@/screens/sign-poc-screen'
import { WalletConnectScreen } from '@/screens/wallet-connect-screen'

export default function App() {
	return (
		<BrowserRouter>
			<WalletSessionProvider>
				<Routes>
					<Route path="/" element={<WalletConnectScreen />} />
					<Route path="/dev/sign" element={<SignPocScreen />} />
					<Route path="*" element={<Navigate to="/" replace />} />
				</Routes>
			</WalletSessionProvider>
		</BrowserRouter>
	)
}
