import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import type { ReactElement } from 'react'
import { AuthSessionProvider } from '@/contexts/auth-session-provider'
import { WalletSessionProvider } from '@/contexts/wallet-session-provider'
import { useAuthSession } from '@/hooks/use-auth-session'
import { ProposalPocScreen } from '@/screens/proposal-poc-screen'
import { SignPocScreen } from '@/screens/sign-poc-screen'
import { WalletConnectScreen } from '@/screens/wallet-connect-screen'

function RequireAuth({ children }: { children: ReactElement }) {
	const { isAuthenticated, isLoading } = useAuthSession()
	if (isLoading) {
		return null
	}
	if (!isAuthenticated) {
		return <Navigate to="/" replace />
	}
	return children
}

export default function App() {
	return (
		<BrowserRouter>
			<AuthSessionProvider>
				<WalletSessionProvider>
					<Routes>
						<Route path="/" element={<WalletConnectScreen />} />
						<Route
							path="/dev/proposal"
							element={
								<RequireAuth>
									<ProposalPocScreen />
								</RequireAuth>
							}
						/>
						<Route
							path="/dev/sign"
							element={
								<RequireAuth>
									<SignPocScreen />
								</RequireAuth>
							}
						/>
						<Route path="*" element={<Navigate to="/" replace />} />
					</Routes>
				</WalletSessionProvider>
			</AuthSessionProvider>
		</BrowserRouter>
	)
}
