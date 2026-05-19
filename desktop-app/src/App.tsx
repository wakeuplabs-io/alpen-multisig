import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import type { ReactElement } from 'react'
import { AuthSessionProvider } from '@/contexts/auth-session-provider'
import { SessionProvider } from '@/contexts/session-provider'
import { ProposalsDashboardScreen } from '@/screens/proposals-dashboard-screen'
import { WalletSessionProvider } from '@/contexts/wallet-session-provider'
import { useAuthSession } from '@/hooks/use-auth-session'
import { BroadcastProposalScreen } from '@/screens/broadcast-proposal-screen'
import { ProposalDetailScreen } from '@/screens/proposal-detail-screen'
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
					<SessionProvider>
						<Routes>
							<Route path="/" element={<WalletConnectScreen />} />
							<Route
								path="/proposals"
								element={
									<RequireAuth>
										<ProposalsDashboardScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/proposals/:actionId"
								element={
									<RequireAuth>
										<ProposalDetailScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/proposals/create"
								element={
									<RequireAuth>
										<ProposalPocScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/dev/proposal"
								element={
									<RequireAuth>
										<ProposalPocScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/proposals/:actionId/sign"
								element={
									<RequireAuth>
										<SignPocScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/proposals/:actionId/broadcast"
								element={
									<RequireAuth>
										<BroadcastProposalScreen />
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
					</SessionProvider>
				</WalletSessionProvider>
			</AuthSessionProvider>
		</BrowserRouter>
	)
}
