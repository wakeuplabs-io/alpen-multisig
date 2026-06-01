// BroadcastDetailsCard — canSign behavioral tests.
//
// Behaviors:
//   1. When canSign=false: broadcast button is disabled and the "Hardware wallet required to sign" label is shown
//   2. When canSign=true (default): broadcast button is enabled
//   3. When isBroadcasting=true: broadcast button is disabled regardless of canSign
//
// Test Budget: 3 behaviors x 2 = 6 unit tests max; using 3.

import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { BroadcastDetailsCard } from '../broadcast-details-card'
import type { PrepareBroadcastResult, Proposal } from '@/api/proposals'

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

function renderCard(overrides: { canSign?: boolean; isBroadcasting?: boolean } = {}) {
	return render(
		<BroadcastDetailsCard
			bundle={fakeBundle}
			proposal={fakeProposal}
			onBroadcast={() => {}}
			isBroadcasting={overrides.isBroadcasting ?? false}
			canSign={overrides.canSign}
		/>,
	)
}

describe('BroadcastDetailsCard broadcast button', () => {
	it('disables button and shows hardware wallet label when canSign=false', () => {
		renderCard({ canSign: false })

		const button = screen.getByRole('button', { name: /confirm & broadcast/i })
		expect(button).toBeDisabled()
		expect(screen.getByText('Hardware wallet required to sign')).toBeInTheDocument()
	})

	it('enables button when canSign=true and not broadcasting', () => {
		renderCard({ canSign: true })

		const button = screen.getByRole('button', { name: /confirm & broadcast/i })
		expect(button).toBeEnabled()
	})

	it('disables button when isBroadcasting=true regardless of canSign', () => {
		renderCard({ isBroadcasting: true, canSign: true })

		const button = screen.getByRole('button', { name: /broadcasting…/i })
		expect(button).toBeDisabled()
	})
})
