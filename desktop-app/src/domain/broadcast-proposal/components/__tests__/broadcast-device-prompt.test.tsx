// BroadcastDevicePrompt — acceptance + unit tests.
//
// Behaviors:
//   1. BroadcastDetailsCard renders BroadcastDevicePrompt during awaiting-device phase
//   2. BroadcastDevicePrompt renders "Confirm on your device" text
//   3. BroadcastDevicePrompt renders a device glyph (svg element)
//
// Test Budget: 3 behaviors x 2 = 6 unit tests max; using 3.

import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { BroadcastDetailsCard } from '../broadcast-details-card'
import { BroadcastDevicePrompt } from '../broadcast-device-prompt'
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

describe('BroadcastDevicePrompt during awaiting-device phase', () => {
	it('renders device confirmation guidance when phase is awaiting-device', () => {
		render(
			<BroadcastDetailsCard
				bundle={fakeBundle}
				proposal={fakeProposal}
				onBroadcast={() => {}}
				isBroadcasting={false}
				canSign={true}
				phase="awaiting-device"
			/>,
		)

		expect(screen.getByText('Confirm on your device')).toBeInTheDocument()
	})
})

describe('BroadcastDevicePrompt component', () => {
	it('renders device confirmation guidance text', () => {
		render(<BroadcastDevicePrompt />)
		expect(screen.getByText('Confirm on your device')).toBeInTheDocument()
	})

	it('renders a device glyph icon', () => {
		const { container } = render(<BroadcastDevicePrompt />)
		const svg = container.querySelector('svg')
		expect(svg).toBeInTheDocument()
	})
})
