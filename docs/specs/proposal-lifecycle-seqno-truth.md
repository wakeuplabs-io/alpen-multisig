# Proposal lifecycle — the sequence number tells the truth

**Applies to:** every authority and every action type. Nothing here is Defcon-specific; it is
written now because running the Defcon 1 flow is what exposed it.

**Related:** [`security-council-defcon-implementation.md`](./security-council-defcon-implementation.md)
§4 Phase 8 points here. [`security-council-defcon.md`](./security-council-defcon.md) is the
functional contract for Defcon 1 and gains the state this document adds.
[`proposal-lifecycle-expiry-and-status-completion.md`](./proposal-lifecycle-expiry-and-status-completion.md)
built the 7-day expiry this document extends to a second terminal state.

**Authority:** PRD 02 §4 — *Safe Multisig and Deviation* — is the requirement this document
implements, and §4.3 (*the backend MUST NOT enforce ordering*) is the line it must not cross.

## 1. The two symptoms

Both came from one manual session on regtest, with three Defcon 1 proposals numbered 1, 2, 3.

1. **Two proposals with quorum were sent while the first was still in the mempool.** Both cards
   settled on *Reveal sent — the reveal transaction is in the mempool, waiting to be mined*, and
   stayed there. Neither reached `Enacted`.
2. **With the safe harbour already active, the third proposal could still be signed and sent.** The
   dashboard note added in Phase 7 was on screen; the sign screen said nothing, and offered its
   usual controls.

Neither symptom is the defect. They are what one unmodelled upstream fact looks like from the
outside.

## 2. The upstream fact

`MultisigAuthority::verify_action_signature`
(`asm/crates/subprotocols/admin/subprotocol/src/authority.rs:66-80`) accepts an action only inside
a window:

```rust
if payload.seqno <= self.last_seqno {
    return Err(AdministrationError::InvalidSeqno { .. });
}
if payload.seqno > self.last_seqno + max_seqno_gap.get() as u64 {
    return Err(AdministrationError::SeqnoGapTooLarge { .. });
}
```

and `update_last_seqno` (`authority.rs:97-99`) **jumps**:

```rust
self.last_seqno = seqno.0;   // not += 1
```

Three consequences follow, and all three are load-bearing:

- **The seqno is inside the signed message.** `SigningMessage::for_action`
  (`asm/crates/subprotocols/admin/txs/src/signing_message.rs:22-36`) puts `Sequence: <n>` in the
  preimage. A proposal whose seqno has been passed cannot be relabelled and resent — it needs a
  fresh quorum over a new message. It is not stale, it is unusable.
- **A rejected action leaves no trace.** The subprotocol's tx loop is
  `let _ = handle_action(state, signed_payload, current_height, relayer);`
  (`asm/crates/subprotocols/admin/subprotocol/src/subprotocol.rs:57-63`) — no log entry, no state,
  no receipt. The only observable is negative: `last_seqno` did not move.
- **The order is the miner's.** Transactions are grouped in block order
  (`asm/crates/stf/src/tx_filter.rs:13-32`), so two proposals broadcast minutes apart can be
  applied in either order, and the proposer does not choose.

## 3. What this codebase does not model

### 3.1 `last_seqno >= seq_no` is not evidence that *this* proposal executed

Phase 7 gave the Defcon 1 predicate its seqno term and fixed the collapse to "the safe harbour is
active". It left a narrower version of the same error in place. `defcon1_enacted`
(`orchestrator-be/src/infrastructure/asm_enactment.rs:195-201`) reads:

```rust
last_seqno >= seq_no && safe_harbour_activated && !defcon1_queued
```

`last_seqno` advances for **any** action of the role and jumps past intermediate values, while the
other two terms are facts about the bridge rather than about this proposal. So the whole
post-condition is satisfiable by somebody else's transaction:

> Proposals #1 (`seq_no = 1`) and #2 (`seq_no = 2`), both Defcon 1, both with confirmed reveals.
> #2 is mined first: `last_seqno = 2`, the safe harbour activates. #1 is mined next and the ASM
> refuses it with `InvalidSeqno`. Reconciliation then reads `2 >= 1`, activated, queue clear —
> and marks **#1** `Enacted`, on a transaction the chain rejected.

The other arms are not in the same position. `multisig_update_post_conditions_met`
(`asm_enactment.rs:261`) and the operator-set predicates check the keys, the threshold and the VK
that the specific action was supposed to install; a jumped seqno alone does not satisfy them. Only
the Defcon 1 arm rests its whole weight on the seqno term. This document tightens that one arm and
says why the others stay as they are.

### 3.2 A proposal whose seqno is consumed is dead, and nothing retires it

Once `last_seqno >= seq_no`, the proposal's transaction will be refused for as long as the chain
exists. `expire_if_overdue` (`orchestrator-be/src/application/proposals.rs:280-287`) returns early
for anything that is not `Pending`, and there is no background job anywhere in the orchestrator —
`main.rs` spawns nothing; every transition is lazy, on an HTTP read. So the proposal keeps its
`Approved` status, keeps its place in the active list, and keeps offering *Send*.

That is symptom 1's after-image, and it is also the third of the four states the review found with
no way out (§9 lists the others, which this phase does not fix).

### 3.3 The screens assert things they have not checked

| Where | Says | Actually |
|---|---|---|
| `desktop-app/src/lib/proposal-send-state.ts:44-47` | *The reveal transaction is in the mempool, waiting to be mined.* | Reports the last persisted `broadcast_status`. Nothing has looked at the mempool since the broadcast screen was closed, and no elapsed time is shown |
| `proposal-send-state.ts:48-51` | *Nothing left to send; the ASM applies the change after the delay.* | A promise. There is no state for "confirmed, and the ASM refused it" |
| `desktop-app/src/domain/sign-proposal/components/sign-proposal-view.tsx:126-129` | *Approving this authorizes the bridge safe harbour to activate immediately once the proposal is broadcast.* | Future tense, rendered unchanged when the safe harbour is already active. This is the screen where symptom 2's decision was taken |

## 4. The rule set

Evaluated in this order, inside the reconcile pass that already runs on every read
(`reconcile_enacted_for_authority`, `application/proposals.rs:349`; `reconcile_enacted_for_action`,
`:457`).

**1. Enacted — tightened for Defcon 1 only.**

```rust
last_seqno == seq_no && safe_harbour_activated && !defcon1_queued
```

Equality is what attributes the jump to this proposal. `>=` asks "has the role moved past this
point", which anyone can cause; `==` asks "is the role standing exactly where this proposal would
have left it".

**2. Superseded — new terminal state.** A proposal that is `Approved`, is not enacted, and whose
`seq_no < last_seqno`, can never execute. It becomes `Superseded` and moves to the Past list.

The two rules are one pass and one ASM read, and the order matters: enactment is decided first, so
a proposal that did enact is never swept.

### 4.1 The residual ambiguity, stated rather than hidden

A proposal that enacted while nothing was reading, and was then jumped past by a later action,
resolves as `Superseded` rather than `Enacted`. The window is narrow — reconciliation runs on every
`GET /proposals`, and the broadcast screen polls every 8 s through exactly that period
(`use-broadcast-proposal.ts`) — and the alternative is the defect of §3.1: `Enacted` asserted on
evidence that belongs to another transaction. This phase prefers a wrong label on a finished
proposal to a wrong label on one that never ran.

`Superseded` is a statement about the future, and that part is never wrong: whatever happened
before, the chain will not accept this transaction now.

### 4.2 This is not ordering enforcement

PRD 02 §4.3 says the backend MUST NOT enforce strict ordering between sequence numbers, and §4.4.2
says it MAY expose metadata to support coordination. Nothing here blocks a higher `seq_no` from
executing before a lower one, refuses a creation, or reorders anything. It reports a fact the chain
has already decided.

## 5. The claim gate

`claim_broadcast_coordination` (`application/proposals.rs:330`) already refuses a claim whose
threshold snapshot has drifted, through `ensure_threshold_snapshot_current` (`:193`). It gains one
more precondition: a proposal whose `seq_no <= last_seqno` cannot be claimed, and the refusal names
the sequence.

The transaction it would broadcast is one the ASM will refuse (§2). Not sending it costs the signer
nothing and saves a commit fee, a reveal fee, and the ephemeral key that is destroyed in the
attempt. This is hygiene on a provably dead transaction, not ordering enforcement (§4.2): the
proposal it refuses is one no ordering could rescue.

Best-effort by construction — `last_seqno` can advance between the check and the block. That is the
same race `ensure_threshold_snapshot_current` accepts, for the same reason.

## 6. Migration — eight commits

The order is forced by one hazard. The TypeScript proposal schema is **closed**, so a `status`
value the frontend does not know is a parse error that takes down the whole list, not an unknown
status on one card — the lesson `security-council-defcon-phase-3.md` recorded for `actionType`.
**The frontend learns `superseded` before the backend can emit it.**

| # | What | Where |
|---|---|---|
| 1 | This document | `docs/specs/` |
| 2 | Defcon 1 enactment requires `last_seqno == seq_no` | `infrastructure/asm_enactment.rs` |
| 3 | The frontend accepts and renders `superseded`; inert until 4 | `api/proposals.ts`, `lib/proposal-status.ts`, the Past bucket, `derive-proposal-actions.ts` |
| 4 | `ProposalStatus::Superseded` and the sweep | `domain/proposal.rs`, `application/proposals.rs` |
| 5 | The claim gate | `application/proposals.rs` |
| 6 | Copy that stops asserting what it has not checked, and says how long | `lib/proposal-send-state.ts` and its two renderers |
| 7 | The safe-harbour note on the sign and broadcast screens | `domain/sign-proposal/`, `domain/broadcast-proposal/` |
| 8 | Back-propagation to the contract and the build plan | `docs/specs/` |

No database migration: `status` is `TEXT NOT NULL` with no `CHECK`
(`orchestrator-be/migrations/20260501000000_create_proposals_tables.sql:5`).

## 7. Tests

| # | Claim | Shape |
|---|---|---|
| 1 | The seqno must be this proposal's | `defcon1_enacted(true, false, 2, 1)` is false — activated, queue clear, seqno passed, and not this proposal's enactment |
| 2 | Equality enacts | `defcon1_enacted(true, false, 2, 2)` is true |
| 3 | The Phase 7 and Phase 4 cases still hold | the existing tests, re-expressed |
| 4 | A passed seqno supersedes | an `Approved` proposal with `seq_no < last_seqno` and unmet post-conditions is `Superseded` after a reconcile |
| 5 | An enacted proposal is never swept | the same pass leaves an enacted proposal `Enacted` |
| 6 | The claim gate refuses a consumed sequence | `claim_broadcast_coordination` returns a conflict naming the sequence, and the proposal's `broadcast_status` is unchanged |

The frontend carries no new automated test: the desktop has no DOM runner, and the `superseded`
label and bucket are pinned by the existing `proposal-display-status` and `derive-proposal-actions`
contract tests, which do run in CI.

## 8. Acceptance criteria

### A. A proposal is enacted only on its own sequence number
**Given** a Defcon 1 proposal whose reveal has confirmed, on a chain where the safe harbour is active
**When** the role's `last_seqno` is greater than the proposal's `seq_no` because another action consumed it
**Then** the proposal is not marked `Enacted`.

### B. A proposal whose sequence is consumed is retired
**Given** an `Approved` proposal that has not enacted
**When** the role's `last_seqno` passes its `seq_no`
**Then** it becomes `Superseded`, appears in the Past list, and offers neither Sign nor Send.

### C. A dead proposal is not broadcast
**Given** an `Approved` proposal whose `seq_no` the role's `last_seqno` has already passed
**When** a signer sends it
**Then** the claim is refused naming the sequence, no transaction is built, and no fee is spent.

### D. In-flight copy states what it has checked
**Given** a proposal whose reveal was broadcast some time ago
**When** a signer looks at it
**Then** the screen says when it was sent and does not assert the transaction's present location or promise enactment.

### E. The safe harbour is visible where the decision is taken
**Given** a Defcon 1 proposal and a chain whose safe harbour is already active
**When** a signer opens the sign screen or the send screen
**Then** the state is on screen, and the existing gates remain the only gates.

## 9. Not in this phase

Recorded because they were found, and because none of them is fixed by anything above. Each is a
candidate for the next phase.

- **Two commits can spend the same UTXO.** `build_and_sign_tx`
  (`desktop-app/src-tauri/src/application/wallet_service.rs:529-549`) runs BDK coin selection with
  no `unspendable()` and no UTXO reservation, and the freshly signed commit is never inserted into
  the local `TxGraph` — so a second broadcast, even seconds later, can select the same input and be
  mutually exclusive with the first in the mempool. The loser's reveal is unrecoverable: the
  ephemeral envelope key is evicted immediately after signing, which
  `desktop-app/src-tauri/src/application/pending_reveals.rs:20-26` already documents as the reason a
  commit must never be RBF-bumped. **This is the most likely cause of symptom 1**, and it is a
  change to the signing path, which is why it is not bundled with a phase about labels.
- **No mempool watcher.** `await_reveal_confirmation` returns `PendingConfirmation` on timeout and
  logs it; nothing ever degrades `reveal_broadcasted` to `failed`, and `proposals_resubmit_reveal`
  exists with no UI that can reach it.
- **`max_seqno_gap` is unknown off-chain.** A `seq_no` more than the deployment's gap past
  `last_seqno` is refused silently on chain (§2) and the app never warns. Nothing in
  `orchestrator-be` or `desktop-app` reads that parameter.
- **No "one in flight per authority" gate.** `claim_broadcast` filters on the primary key alone
  (`postgres_repo.rs`), so N proposals of one authority can be in flight at once.
- **`next_seq_no_from_state` counts dead proposals.** `local_max` spans every proposal regardless
  of status (`application/proposals.rs:272`), so superseded and expired ones keep inflating the
  next suggested sequence.

## 10. Manual verification

On the regtest stack that already carries the three stuck proposals:

1. The dashboard moves the proposals whose sequence the chain has passed to **Past** as
   *Superseded*, and leaves the one that executed reading *Enacted*.
2. A superseded proposal offers no Sign and no Send, on the card and on the detail screen.
3. A fresh Defcon 1 with the next sequence reaches `Enacted` on its own sequence, and only then.
4. Sending a proposal whose sequence is consumed is refused, the message names the sequence, and no
   fee is spent.
5. The sign screen for a Defcon 1 shows the safe-harbour state when it is active, and the
   type-to-confirm gate is still the only gate.
