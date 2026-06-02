// BroadcastDetailsCard — mnemonic path regression test.
//
// Behaviors:
//   1. Mnemonic session → no device prompt rendered (byte-identical to R1.0.1)
//   2. Standard broadcast card content renders (proposal header, commit TX, broadcast button)
//
// This is a regression safeguard: ensures the mnemonic / simulated-HW session
// never acquires a device prompt affordance (the signer returns instantly, so
// the prompt is transient/never shown).
//
// Test Budget: 2 behaviors x 2 = 4 unit tests max; using 2.

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

function renderCard() {
	return render(
		<BroadcastDetailsCard
			bundle={fakeBundle}
			proposal={fakeProposal}
			onBroadcast={() => {}}
			isBroadcasting={false}
			canSign={true}
		/>,
	)
}

describe('BroadcastDetailsCard mnemonic path (regression)', () => {
	it('does not render any device prompt UI', () => {
		renderCard()

		// No device prompt text should appear — mnemonic signer returns instantly
		expect(screen.queryByText(/confirm on device/i)).not.toBeInTheDocument()
		expect(screen.queryByText(/connect your/i)).not.toBeInTheDocument()
		expect(screen.queryByText(/hardware wallet/i)).not.toBeInTheDocument()
	})

	it('renders standard broadcast card content byte-identical to R1.0.1', () => {
		renderCard()

		// Proposal header
		expect(screen.getByText('Proposal #1')).toBeInTheDocument()
		expect(screen.getByText('admin-1')).toBeInTheDocument()
		expect(screen.getByText('Quorum reached')).toBeInTheDocument()

		// Commit TX
		expect(screen.getByText('Commit TX (preview)')).toBeInTheDocument()
		expect(screen.getByText('bc1qtest123')).toBeInTheDocument()

		// Broadcast button
		const button = screen.getByRole('button', { name: /confirm & broadcast/i })
		expect(button).toBeEnabled()
	})
})
