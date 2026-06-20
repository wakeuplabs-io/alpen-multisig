import { createBrowserRouter, Navigate, Outlet, RouterProvider } from 'react-router-dom'
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
import { SessionExpiryModal } from '@/components/session-expiry-modal'

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

function AppLayout() {
	return (
		<AuthSessionProvider>
			<WalletSessionProvider>
				<SessionProvider>
					<SessionExpiryModal />
					<Outlet />
				</SessionProvider>
			</WalletSessionProvider>
		</AuthSessionProvider>
	)
}

const router = createBrowserRouter([
	{
		element: <AppLayout />,
		children: [
			{ path: '/', element: <WalletConnectScreen /> },
			{
				path: '/proposals',
				element: (
					<RequireAuth>
						<ProposalsDashboardScreen />
					</RequireAuth>
				),
			},
			{
				path: '/proposals/:actionId',
				element: (
					<RequireAuth>
						<ProposalDetailScreen />
					</RequireAuth>
				),
			},
			{
				path: '/proposals/create',
				element: (
					<RequireAuth>
						<CreateProposalScreen />
					</RequireAuth>
				),
			},
			{
				path: '/dev/proposal',
				element: (
					<RequireAuth>
						<CreateProposalScreen />
					</RequireAuth>
				),
			},
			{
				path: '/proposals/:actionId/sign',
				element: (
					<RequireAuth>
						<SignScreen />
					</RequireAuth>
				),
			},
			{
				path: '/proposals/:actionId/broadcast',
				element: (
					<RequireAuth>
						<BroadcastProposalScreen />
					</RequireAuth>
				),
			},
			{
				path: '/proposals/:actionId/cancel',
				element: (
					<RequireAuth>
						<CancelProposalScreen />
					</RequireAuth>
				),
			},
			{
				path: '/proposals/:actionId/cancel/sign',
				element: (
					<RequireAuth>
						<CancelProposalSignScreen />
					</RequireAuth>
				),
			},
			{
				path: '/proposals/:actionId/cancel/broadcast',
				element: (
					<RequireAuth>
						<CancelProposalBroadcastScreen />
					</RequireAuth>
				),
			},
			{
				path: '/dev/sign',
				element: (
					<RequireAuth>
						<SignScreen />
					</RequireAuth>
				),
			},
			{ path: '/manual', element: <ManualProposalScreen /> },
			{ path: '/block-payouts', element: <BlockPayoutsScreen /> },
			{ path: '*', element: <Navigate to="/" replace /> },
		],
	},
])

export default function App() {
	return <RouterProvider router={router} />
}
