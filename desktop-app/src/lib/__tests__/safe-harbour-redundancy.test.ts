import assert from 'node:assert/strict'
import { changedNothingActionIds, type HarbourActivationCandidate } from '../safe-harbour-redundancy.ts'

function proposal(overrides: Partial<HarbourActivationCandidate> = {}): HarbourActivationCandidate {
	return {
		actionId: 'a',
		actionType: 'defcon_1',
		status: 'enacted',
		activationHeight: 100,
		seqNo: 1,
		...overrides,
	}
}

// ── Height beats sequence number, across both Defcon types ─────────────────
// The discriminating case: a Defcon 3 revealed at block 100 with `defcon3 = 20` matures at 120,
// and a Defcon 1 revealed at 118 sweeps the bridge two blocks before it. The Defcon 3 holds the
// LOWER sequence number, so ordering by seqno names it the activator and leaves a genuinely
// redundant proposal unbadged. The activation height is what tells the truth.
{
	const redundant = changedNothingActionIds([
		proposal({ actionId: 'd3', seqNo: 5, activationHeight: 120, actionType: 'defcon_3' }),
		proposal({ actionId: 'd1', seqNo: 6, activationHeight: 118, actionType: 'defcon_1' }),
	])
	assert.ok(redundant.has('d3'), 'the Defcon 3 matured after the harbour was already up')
	assert.ok(!redundant.has('d1'), 'the Defcon 1 activated it, despite the higher sequence number')
}

// ── The V1 answer, preserved ────────────────────────────────────────────────
// The earliest enacted proposal is the one that activated the safe harbour; every enacted one
// after it ran against a flag that was already true. Heights are monotone in seqno here, which is
// what a Defcon-1-only history always looks like: a Defcon 1's lock period is 0, so its activation
// height is its reveal block, and the ASM accepts at the reveal.
{
	const redundant = changedNothingActionIds([
		proposal({ actionId: 'c', seqNo: 4, activationHeight: 140 }),
		proposal({ actionId: 'a', seqNo: 1, activationHeight: 110 }),
		proposal({ actionId: 'b', seqNo: 2, activationHeight: 120, status: 'superseded' }),
		proposal({ actionId: 'd', seqNo: 3, activationHeight: 130 }),
	])
	assert.deepEqual([...redundant].sort(), ['c', 'd'], 'every enactment after the first is redundant')
	assert.ok(!redundant.has('a'), 'the earliest activation height is the one that turned the harbour on')
	assert.ok(!redundant.has('b'), 'a proposal that never enacted changed nothing to report')
}

// ── Another action type shares no state with the safe harbour ───────────────
// Heights are non-null on purpose: with nulls this would pass for the wrong reason.
assert.equal(
	changedNothingActionIds([
		proposal({ actionId: 'x', seqNo: 1, activationHeight: 100, actionType: 'vk_update' }),
		proposal({ actionId: 'y', seqNo: 2, activationHeight: 105, actionType: 'vk_update' }),
	]).size,
	0,
	'only harbour-activating actions are considered',
)

// ── A null activation height is neither activator nor redundant ────────────
// The height is computed once, non-fatally, when the reveal confirms, and never retried, so a null
// is a missing observation rather than an early block.
//
// This case is also where the rule COSTS something, and the fixture is built to show it: if
// `unknown-height` is what really activated the harbour, then `first` changed nothing and V1 would
// have badged it — ordering by seqno, it did not need the height. Here it goes unbadged. The trade
// is that a row with no height cannot be ranked against one with a real number without guessing its
// position from the sequence number, which is sound for a Defcon 1 and false for a Defcon 3. The
// badge errs towards saying nothing rather than towards calling an activation redundant. The real
// fix is a backend that retries the height.
{
	const redundant = changedNothingActionIds([
		proposal({ actionId: 'unknown-height', seqNo: 1, activationHeight: null }),
		proposal({ actionId: 'first', seqNo: 2, activationHeight: 100 }),
		proposal({ actionId: 'later', seqNo: 3, activationHeight: 105 }),
	])
	assert.deepEqual([...redundant], ['later'], 'the earliest KNOWN height is the activator')
	assert.ok(!redundant.has('unknown-height'), 'a proposal with no height makes no claim either way')
}

// ── All heights null: no activator, no badges. Deliberate ──────────────────
// Every proposal that enacted before the activation_height migration carries null forever, and it
// has no backfill. The badge disappearing is the correct failure: the dashboard still reports the
// harbour from a live chain read, it just stops attributing the activation to a row. A future
// author who "fixes" this by falling back to the sequence number should go red here.
assert.equal(
	changedNothingActionIds([
		proposal({ actionId: 'old-1', seqNo: 1, activationHeight: null }),
		proposal({ actionId: 'old-2', seqNo: 2, activationHeight: null }),
	]).size,
	0,
	'no evidence of which one activated the harbour means no claim about either',
)

// ── A tie is broken by sequence number, not by arrival order ───────────────
// Two proposals can share an activation height. `Array.prototype.sort` is stable, so without a
// tie-break the winner would be whichever the backend happened to return first — the higher-seqno
// one is placed first here precisely to catch that.
{
	const redundant = changedNothingActionIds([
		proposal({ actionId: 'later-seqno', seqNo: 9, activationHeight: 200 }),
		proposal({ actionId: 'earlier-seqno', seqNo: 8, activationHeight: 200 }),
	])
	assert.deepEqual([...redundant], ['later-seqno'], 'at equal height the lower sequence number activated')
}

// ── A single enactment is the activation itself ─────────────────────────────
assert.equal(changedNothingActionIds([proposal({ actionId: 'solo', seqNo: 7 })]).size, 0)

console.log('safe-harbour-redundancy: all assertions passed')
