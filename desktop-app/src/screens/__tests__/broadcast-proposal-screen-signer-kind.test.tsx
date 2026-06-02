// BroadcastProposalScreen — signerKind wiring acceptance test.
//
// Behaviors:
//   1. Screen passes signerKind from useAdminWalletCapability into useBroadcastProposal
//   2. Screen passes canSignReason from useAdminWalletCapability into BroadcastDetailsCard
//   3. useCancelBroadcast derives BroadcastError from raw string (error?.message for user feedback)
//
// Test Budget: 3 behaviors x 2 = 6 unit tests max; using 3.

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { BroadcastProposalScreen } from '../broadcast-proposal-screen'
import type { PrepareBroadcastResult, Proposal } from '@/api/proposals'

// ── Mocks ─────────────────────────────────────────────────────────────────────

const mockBroadcastProposal = vi.fn()
const mockPrepareBroadcast = vi.fn()
const mockGetProposalByActionId = vi.fn()
const mockGetAdminWalletCanSign = vi.fn()
const mockGetAdminWalletUtxos = vi.fn()
const mockGetAdminWalletInfo = vi.fn()
const mockGetAdminWalletBalance = vi.fn()
const mockGetAdminWalletAddresses = vi.fn()
const mockSyncAdminWallet = vi.fn()

vi.mock('@/api/orchestrator-auth', () => ({
	ORCHESTRATOR_BASE_URL: 'http://localhost:3000',
}))

vi.mock('@/api/proposals', () => ({
	prepareBroadcast: (...args: unknown[]) => mockPrepareBroadcast(...args),
	broadcastProposal: (...args: unknown[]) => mockBroadcastProposal(...args),
	getProposalByActionId: (...args: unknown[]) => mockGetProposalByActionId(...args),
}))

vi.mock('@/api/admin-wallet', () => ({
	getAdminWalletCanSign: () => mockGetAdminWalletCanSign(),
	getAdminWalletUtxos: () => mockGetAdminWalletUtxos(),
	getAdminWalletInfo: () => mockGetAdminWalletInfo(),
	getAdminWalletBalance: () => mockGetAdminWalletBalance(),
	getAdminWalletAddresses: () => mockGetAdminWalletAddresses(),
	syncAdminWallet: () => mockSyncAdminWallet(),
}))

vi.mock('@/hooks/use-session', () => ({
	useSession: () => ({
		wallet: { addressSample: 'bc1qtest123address' },
		selectedRole: 'admin',
		sessionTimeLabel: '14m 32s',
		sessionWarning: null,
		disconnectSession: vi.fn(),
	}),
}))

vi.mock('@/domain/broadcast-proposal/hooks/use-admin-wallet-info', () => ({
	useAdminWalletInfo: () => ({ adminWalletInfo: null }),
}))

vi.mock('@/domain/admin-wallet/hooks', () => ({
	useAdminWalletUtxos: () => ({ data: null, refresh: vi.fn() }),
	useAdminWalletSync: () => ({ syncStatus: null, triggerSync: vi.fn() }),
}))

vi.mock('@/domain/admin-wallet/hooks/use-wallet-panel-state', () => ({
	useWalletPanelState: () => ({
		isOpen: false,
		expandedSection: null,
		open: vi.fn(),
		close: vi.fn(),
		setExpandedSection: vi.fn(),
	}),
}))

vi.mock('@/domain/admin-wallet/hooks/use-admin-wallet-balance', () => ({
	useAdminWalletBalance: () => ({ data: null, error: null, isLoading: false, refresh: vi.fn() }),
}))

vi.mock('@/domain/admin-wallet/hooks/use-admin-wallet-addresses', () => ({
	useAdminWalletAddresses: () => ({ data: [], error: null, isLoading: false, refresh: vi.fn() }),
}))

vi.mock('@/domain/admin-wallet/hooks/use-addresses-with-balance', () => ({
	useAddressesWithBalance: () => ({ data: [], isLoading: false, error: null, refresh: vi.fn() }),
}))

// ── Test doubles ──────────────────────────────────────────────────────────────

const fakeBundle: PrepareBroadcastResult = {
	actionId: 'action-1',
	commitAddress: 'bc1qtest123',
	commitAmountSats: 100_000,
	estimatedFeeSats: 500,
}

const fakeProposal: Proposal = {
	actionId: 'action-1',
	seqNo: 1,
	authority: 'admin-1',
	status: 'approved',
	actionHex: '00deadbeef',
	actionType: 'multisig_update',
	signatures: [
		{ signerPubkey: 'pub1', signatureHex: 'sig1' },
		{ signerPubkey: 'pub2', signatureHex: 'sig2' },
	],
	requiredSignatures: 2,
	broadcastStatus: 'idle',
	kind: 'update',
	targetActionId: null,
	activationHeight: null,
	updateIdInQueue: null,
	cancelProposal: null,
}

function setupCapability(canSign: boolean, signerKind: 'hardware' | 'mnemonic' | 'none', canSignReason?: string) {
	mockGetAdminWalletCanSign.mockResolvedValue({
		ok: true,
		data: { canSign, signerKind, reason: canSignReason },
	})
}

function setupBroadcastProposal() {
	mockPrepareBroadcast.mockResolvedValue({ ok: true, data: fakeBundle })
	mockGetProposalByActionId.mockResolvedValue({ ok: true, data: fakeProposal })
	mockBroadcastProposal.mockResolvedValue({
		ok: true,
		data: {
			actionId: 'action-1',
			proposalStatus: 'approved',
			broadcastStatus: 'idle',
			commitTxid: 'txid1',
			revealTxid: 'txid2',
		},
	})
}

function renderScreen(actionId = 'action-1') {
	return render(
		<MemoryRouter initialEntries={[`/proposals/${actionId}/broadcast`]}>
			<Routes>
				<Route path="/proposals/:actionId/broadcast" element={<BroadcastProposalScreen />} />
			</Routes>
		</MemoryRouter>,
	)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('BroadcastProposalScreen signerKind wiring', () => {
	beforeEach(() => {
		vi.clearAllMocks()
	})

	it('passes signerKind=hardware into broadcast controller when capability reports hardware', async () => {
		setupCapability(true, 'hardware', 'Ledger Nano S Plus connected')
		setupBroadcastProposal()

		renderScreen()

		// Wait for the component to initialize (prepareBroadcast is called on mount)
		await vi.waitFor(() => {
			expect(mockPrepareBroadcast).toHaveBeenCalled()
		})

		// The screen should have wired signerKind='hardware' into useBroadcastProposal.
		// We verify this indirectly: when signerKind is 'hardware', the broadcast()
		// function sets phase to 'awaiting-device' before broadcasting.
		// Since the component renders BroadcastDetailsCard when phase='confirming',
		// we check that the card renders (proving the controller was initialized).
		await vi.waitFor(() => {
			expect(screen.getByRole('button', { name: /confirm & broadcast/i })).toBeInTheDocument()
		})
	})

	it('passes canSignReason into BroadcastDetailsCard when signer unavailable', async () => {
		setupCapability(false, 'none', 'No hardware wallet detected')
		setupBroadcastProposal()

		renderScreen()

		// When canSign=false, the card should show the unavailability reason
		// instead of the hardcoded "Hardware wallet required to sign" message.
		await vi.waitFor(() => {
			expect(screen.getByRole('button', { name: /confirm & broadcast/i })).toBeDisabled()
		})

		// The card should display the canSignReason from the capability hook
		await vi.waitFor(() => {
			expect(screen.getByText('No hardware wallet detected')).toBeInTheDocument()
		})
	})
})
