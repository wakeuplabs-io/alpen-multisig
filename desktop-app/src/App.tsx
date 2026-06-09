import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import type { ReactElement } from 'react'
import { AuthSessionProvider } from '@/contexts/auth-session-provider'
import { SessionProvider } from '@/contexts/session-provider'
import { ProposalsDashboardScreen } from '@/screens/proposals-dashboard-screen'
import { WalletSessionProvider } from '@/contexts/wallet-session-provider'
import { useSession } from '@/hooks/use-session'
import { BroadcastProposalScreen } from '@/screens/broadcast-proposal-screen'
import { CancelProposalBroadcastScreen } from '@/screens/cancel-proposal-broadcast-screen'
import { CancelProposalScreen } from '@/screens/cancel-proposal-screen'
import { CancelProposalSignScreen } from '@/screens/cancel-proposal-sign-screen'
import { ProposalDetailScreen } from '@/screens/proposal-detail-screen'
import { CreateProposalScreen } from '@/screens/create-proposal-screen'
import { SignScreen } from '@/screens/sign-screen'
import { WalletConnectScreen } from '@/screens/wallet-connect-screen'
import { BlockPayoutsScreen } from '@/screens/block-payouts-screen'
import { ManualProposalScreen } from '@/screens/manual-proposal-screen'

function RequireAuth({ children }: { children: ReactElement }) {
	const { isAuthenticated, isOrchestratorSessionActive, isLoading } = useSession()
	if (isLoading) {
		return null
	}
	if (!isAuthenticated && !isOrchestratorSessionActive) {
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
										<CreateProposalScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/dev/proposal"
								element={
									<RequireAuth>
										<CreateProposalScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/proposals/:actionId/sign"
								element={
									<RequireAuth>
										<SignScreen />
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
								path="/proposals/:actionId/cancel"
								element={
									<RequireAuth>
										<CancelProposalScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/proposals/:actionId/cancel/sign"
								element={
									<RequireAuth>
										<CancelProposalSignScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/proposals/:actionId/cancel/broadcast"
								element={
									<RequireAuth>
										<CancelProposalBroadcastScreen />
									</RequireAuth>
								}
							/>
							<Route
								path="/dev/sign"
								element={
									<RequireAuth>
										<SignScreen />
									</RequireAuth>
								}
							/>
							<Route path="/manual" element={<ManualProposalScreen />} />
							<Route path="/block-payouts" element={<BlockPayoutsScreen />} />
							<Route path="*" element={<Navigate to="/" replace />} />
						</Routes>
					</SessionProvider>
				</WalletSessionProvider>
			</AuthSessionProvider>
		</BrowserRouter>
	)
}
