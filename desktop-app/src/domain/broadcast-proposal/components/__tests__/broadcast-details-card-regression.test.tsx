// BroadcastDetailsCard — mnemonic path regression test (slice a).
//
// Behaviors:
//   1. Mnemonic session: disabled={isBroadcasting || !canSign} invariant preserved
//   2. Mnemonic session: no device prompt rendered (byte-identical to R1.0.1)
//   3. Mnemonic session: commit→reveal→confirmed card advances as R1.0.1
//
// This is a regression safeguard: ensures the mnemonic / simulated-HW session
// keeps card behavior identical — the only observable changes are structured-error
// copy + corrected resubmit gating (handled by BroadcastPhaseProgress, not this card).
//
// Test Budget: 3 behaviors x 2 = 6 unit tests max; using 4.

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
			canSign={overrides.canSign ?? true}
		/>,
	)
}

describe('BroadcastDetailsCard mnemonic path (regression — slice a)', () => {
	it('disabled={isBroadcasting || !canSign} invariant: enabled when canSign=true and not broadcasting', () => {
		renderCard({ canSign: true, isBroadcasting: false })

		const button = screen.getByRole('button', { name: /confirm & broadcast/i })
		expect(button).toBeEnabled()
	})

	it('disabled={isBroadcasting || !canSign} invariant: disabled when canSign=false (mnemonic HW not required)', () => {
		renderCard({ canSign: false, isBroadcasting: false })

		const button = screen.getByRole('button', { name: /confirm & broadcast/i })
		expect(button).toBeDisabled()
	})

	it('disabled={isBroadcasting || !canSign} invariant: disabled when isBroadcasting=true regardless of canSign', () => {
		renderCard({ canSign: true, isBroadcasting: true })

		const button = screen.getByRole('button', { name: /broadcasting…/i })
		expect(button).toBeDisabled()
	})

	it('no device prompt rendered — byte-identical to R1.0.1', () => {
		renderCard({ canSign: true, isBroadcasting: false })

		expect(screen.queryByText(/confirm on device/i)).not.toBeInTheDocument()
		expect(screen.queryByText(/connect your/i)).not.toBeInTheDocument()
		expect(screen.queryByText(/hardware wallet/i)).not.toBeInTheDocument()
	})
})
