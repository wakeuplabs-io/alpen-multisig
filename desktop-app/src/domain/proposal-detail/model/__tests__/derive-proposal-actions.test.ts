import assert from 'node:assert/strict'
import { deriveProposalActions, type ProposalActionInput } from '../derive-proposal-actions.ts'

const SIGNER_A = '02aaaa'
const SIGNER_B = '02bbbb'
const SIGNER_C = '02cccc'

function proposal(overrides: Partial<ProposalActionInput> = {}): ProposalActionInput {
	return {
		status: 'pending',
		broadcastStatus: 'idle',
		requiredSignatures: 2,
		signatures: [],
		...overrides,
	}
}

// ── REGRESSION: quorum reached but NOT broadcasted ───────────────────────────
// An eligible signer who has not signed yet must still be able to sign while
// the proposal is not broadcasted, even though quorum is already met.
// Before the fix, the Sign affordance was gated on `!hasQuorum`, so canSign
// was false here — this assertion would FAIL against the buggy code.
{
	const actions = deriveProposalActions(
		proposal({
			status: 'pending',
			broadcastStatus: 'idle',
			requiredSignatures: 2,
			signatures: [{ signerPubkey: SIGNER_A }, { signerPubkey: SIGNER_B }],
		}),
		SIGNER_C,
	)
	assert.equal(actions.hasQuorum, true, 'quorum reached by signature count')
	assert.equal(actions.broadcastStarted, false, 'broadcast has not started')
	assert.equal(actions.canSign, true, 'eligible non-signer can still sign with quorum but no broadcast')
}

// ── REGRESSION: approved (quorum) + idle broadcast still allows signing ──────
{
	const actions = deriveProposalActions(
		proposal({
			status: 'approved',
			broadcastStatus: 'idle',
			requiredSignatures: 2,
			signatures: [{ signerPubkey: SIGNER_A }, { signerPubkey: SIGNER_B }],
		}),
		SIGNER_C,
	)
	assert.equal(actions.canSign, true, 'approved-but-not-broadcasted still allows an eligible signer to sign')
	assert.equal(actions.canBroadcast, true, 'approved + idle is broadcastable')
}

// ── Pending, eligible non-signer: can sign ───────────────────────────────────
{
	const actions = deriveProposalActions(proposal({ signatures: [{ signerPubkey: SIGNER_A }] }), SIGNER_B)
	assert.equal(actions.canSign, true, 'pending proposal allows eligible non-signer to sign')
	assert.equal(actions.hasQuorum, false, 'one of two signatures is not quorum')
}

// ── Already signed: cannot sign again ────────────────────────────────────────
{
	const actions = deriveProposalActions(proposal({ signatures: [{ signerPubkey: SIGNER_A }] }), SIGNER_A)
	assert.equal(actions.alreadySigned, true, 'signer present in signatures is alreadySigned')
	assert.equal(actions.canSign, false, 'a signer who already signed cannot sign again')
}

// ── alreadySigned is case-insensitive on pubkey ──────────────────────────────
{
	const actions = deriveProposalActions(proposal({ signatures: [{ signerPubkey: SIGNER_A.toUpperCase() }] }), SIGNER_A)
	assert.equal(actions.alreadySigned, true, 'pubkey match is case-insensitive')
}

// ── Broadcast started: signing is closed ─────────────────────────────────────
{
	const actions = deriveProposalActions(
		proposal({
			status: 'approved',
			broadcastStatus: 'commit_broadcasted',
			signatures: [{ signerPubkey: SIGNER_A }, { signerPubkey: SIGNER_B }],
		}),
		SIGNER_C,
	)
	assert.equal(actions.broadcastStarted, true, 'commit_broadcasted means broadcast started')
	assert.equal(actions.canSign, false, 'once broadcast starts, signing is no longer offered')
	assert.equal(actions.canBroadcast, false, 'cannot re-broadcast once broadcast started')
}

// ── reveal_confirmed (the screenshot case): no signing ───────────────────────
{
	const actions = deriveProposalActions(
		proposal({
			status: 'approved',
			broadcastStatus: 'reveal_confirmed',
			signatures: [{ signerPubkey: SIGNER_A }, { signerPubkey: SIGNER_B }],
		}),
		SIGNER_C,
	)
	assert.equal(actions.canSign, false, 'reveal-confirmed proposal does not offer signing')
}

// ── Terminal states: no actions ──────────────────────────────────────────────
for (const status of ['enacted', 'canceled', 'expired'] as const) {
	const actions = deriveProposalActions(
		proposal({ status, signatures: [{ signerPubkey: SIGNER_A }, { signerPubkey: SIGNER_B }] }),
		SIGNER_C,
	)
	assert.equal(actions.isTerminal, true, `${status} is terminal`)
	assert.equal(actions.canSign, false, `${status}: no signing`)
	assert.equal(actions.canBroadcast, false, `${status}: no broadcast`)
}

// ── No connected signer: cannot sign ─────────────────────────────────────────
{
	const actions = deriveProposalActions(proposal(), null)
	assert.equal(actions.canSign, false, 'no signerPubkey means no sign affordance')
	assert.equal(actions.alreadySigned, false, 'no signer cannot have already signed')
}

console.log('derive-proposal-actions: all assertions passed.')
