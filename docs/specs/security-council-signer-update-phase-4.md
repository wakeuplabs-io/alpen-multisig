# Security Council — Signer Update (V3), Phase 4: The cancel and the e2e

**Functional contract:** [`security-council-signer-update.md`](./security-council-signer-update.md) —
the SSOT for *what* V3 must do. This document never overrides it.

**Build plan:** [`security-council-signer-update-implementation.md`](./security-council-signer-update-implementation.md)
§4 Phase 4. This document is that phase at implementation detail.

**Closes:** [AC 6](./security-council-signer-update.md#6-it-is-queued-not-enacted-on-broadcast),
[AC 7b](./security-council-signer-update.md#7b-the-new-council-can-act-and-the-removed-signers-cannot),
[AC 8](./security-council-signer-update.md#8-a-cancelled-rotation-never-applies),
[AC 9](./security-council-signer-update.md#9-the-cancel-is-signed-by-the-strata-administrator) and
[AC 10](./security-council-signer-update.md#10-the-council-never-sees-the-proposal-that-rotates-it);
Constraint 1 against a real regtest ASM. The implementation path for
[AC 13](./security-council-signer-update.md#13-the-manual-fallback-works) is structurally covered,
but its manual/external-RPC execution evidence remains pending (§10).

**Story:** US-E7 — a Strata Administrator signer rotates the Security Council's membership and
threshold, with a cancellable activation window.

## 1. The change in one sentence

The first end-to-end exercise of tx type 15 proves that an administrator-authorized rotation changes
who can operate the council only after its live depth, while an administrator-authorized cancel
inside that window leaves the council unchanged.

In user terms: an administrator can replace a council signer without granting the removed signer
continued emergency power, and can still stop the rotation before it takes effect.

## 2. What this phase is not

It is not the create form, target-role derivation, enactment predicate, codec vocabulary or signing
message. Phases 1–3 shipped those pieces and their focused tests.

It does not repair
[Constraint 4](./security-council-signer-update.md#4-acceptance-is-not-application-and-upstream-does-not-say-so),
add a client-side protocol validator, or add a target authority to the proposal row. It does not
implement V4.

It does not add an ASM-backed test inside `orchestrator-be`: Phase 2 deliberately kept the
two-role truth table pure, and this phase supplies the real-chain evidence once, at the e2e layer.
It adds no DOM or source-text test. The desktop cancel journey, countdown and manual route remain
the explicit manual validation in §10.

The deliverable is a new `e2e-tests/tests/e2e_council_rotation.rs`. It does not extend
`e2e_defcon_probe.rs`, whose helper and fixture assume the administrator and council have the same
two keys — exactly the ambiguity this phase must avoid.

### 2.1 Acceptance boundary

This is an **implementation phase spec and protocol regression e2e**, not the stakeholder-facing BDD
contract. The Given/When/Then user journeys already live in the functional contract's AC 6, 7b, 8,
9, 10 and 13 and remain the living business documentation.

The chain has no rejection response: administration errors are intentionally ignored while the
worker continues processing the block. Therefore the only truthful driving-port evidence for a
removed signer is the canonical ASM read after its transaction confirms: emergency mode remains
off, no Defcon is queued and the council sequence is reusable. Replacing those observations with a
fabricated direct error would test behaviour the protocol does not expose.

The old-quorum rejection and new-quorum acceptance remain one e2e because sharing the same action,
sequence and harness is the discriminating proof: the second transaction succeeding demonstrates
that the first neither applied nor consumed sequence state. Splitting them would add a harness boot,
lose that proof and make the suite more expensive.

## 3. Spec traceability audit

| Contract claim | Existing evidence | Phase 4 evidence |
|---|---|---|
| AC 6 — queued, not enacted | `depth_for_action` resolves tx type 15; lifecycle reads queue state | Real ASM queue contains tx 15 after reveal; council config is byte-for-byte unchanged |
| AC 7 / 7a — two roles | Phase 2's three named truth-table tests | Enacted path observes council config and both roles' sequence counters |
| AC 7b — membership has an effect | None here or upstream | A Defcon signed by the new quorum applies; the old quorum with the removed signer does not |
| AC 8 — cancel never applies | Generic administrator cancel e2e | Tx 15 cancel, then a measured tip past its original activation height, with council config unchanged |
| AC 9 — administrator cancels | Generic `create_cancel_proposal` authority-scope tests; upstream derives a cancel role from its embedded update | Tx 15 cancel is signed by the administrator and advances only its sequence counter |
| AC 10 — council cannot see it | Generic list and detail authority-scope tests | One explicit application test uses a Strata Admin proposal and a Security Council read |
| AC 13 — manual fallback | Phase 1 decode/type coverage; V2 Phase 7 made cancels decodable | Structural audit in §4.3; execution evidence remains pending in §10 |

## 4. The build plan's “no new backend code” bet

The protocol bet holds: tx type 15 needs no new production branch. The AC 10 audit did expose one
pre-existing coordination-layer security gap, fixed in §4.4.

### 4.1 Cancel creation is already action-generic

`create_cancel_proposal` requires the target's authority, stores the cancel under that authority and
derives cancelability from the target action's live depth. A council rotation proposal belongs to
`strata_admin`, while its action hex names the council as target. That existing split is the correct
one: the administrator files and signs the cancel; the council has no veto over its own rotation.

The existing foreign-authority test uses a Strata Admin target and a Sequencer Manager session. The
rule is generic, but AC 9 names the Security Council case because that inversion is the point of V3.
A cheap test uses a tx type 15 fixture and proves that a Security Council session is refused and
nothing is persisted.

### 4.2 Proposal visibility is already authority-generic

`list_proposals` queries by the authenticated authority and `get_update_action` scopes the read.
Existing tests use other authority pairs. AC 10 explicitly says
“asserted rather than assumed”, so one test inserts a Strata Admin tx type 15 proposal and proves
that a Security Council list is empty and direct detail access is indistinguishable from a missing
id. It tests the two driving reads in one setup.

### 4.3 The manual fallback already accepts tx type 15

Phase 1 made a council rotation decode as `multisig_update` with target role
`security_council`. Phase 3's `decodedActionAuthorizingAuthority`
(`desktop-app/src/domain/manual-proposal/model/authorizing-authority.ts:9-20`) maps that one target
to `strata_admin`; the typed and JSON import paths both apply it before loading the authorizing
multisig config. `actionTypeFromDecoded` reports `council_signer_update`, and the generic signing,
aggregation and broadcast functions operate on the upstream `MultisigAction`.

Those pure seams already have discriminating tx type 15 tests. Another unit test would repeat a
mapping, while a hook test would require mocking every Tauri boundary and pin orchestration rather
than behaviour. They establish implementation readiness, not the external-RPC execution evidence
AC 13 requires. If the manual walk finds a rejection, it is a Phase 4 regression and is fixed before
V3 is marked fully validated.

### 4.4 Audit finding: foreign detail reads must not be an existence oracle

Before this phase, an authenticated signer received `Unauthorized` for an existing proposal owned
by another authority and `NotFound` for an unknown id. Those 401/404 responses revealed whether a
guessed action id existed, contrary to the backend non-enumerability rule.

Read paths now use a narrow visibility guard that returns `NotFound` for both cases. Write paths
retain `Unauthorized`, where naming an authority mismatch is appropriate. The handler and
application tests pin the public response and the tx type 15 council case. This is the phase's only
production behaviour change; it does not implement protocol validity.

## 5. E2E design

### 5.1 A deliberately non-overlapping fixture

The file owns a small fixture built from fixed valid `SecretKey` bytes:

- administrator: 2-of-2, keys `A0 = [1; 32]` and `A1 = [2; 32]`;
- council before rotation: 2-of-3, keys `C0 = [3; 32]`, `C1 = [4; 32]`,
  `C2 = [5; 32]`;
- council after rotation: 2-of-3, keys `C0`, `C2`, `C3` — remove `C1`, add `C3`;
- replacement key: `C3 = [6; 32]`;
- every unrelated role: a valid 1-of-1 config;
- `strata_security_council_multisig_update = 5`;
- `strata_admin_multisig_update = 9`.

The administrator and council sets are disjoint. That is load-bearing: with the repository's shared
mnemonic fixture, a test can accidentally authorize tx type 15 with council keys or a Defcon with
administrator keys and still look plausible.

Fixed keys make a failed run reproducible. The two deliberately different depths prove tx type 15
does not inherit the administrator's own update depth merely because the administrator authorizes
both actions. Five blocks preserve a real queue and a useful cancel window while keeping both tests
cheap. Each top-level test builds its own harness; no chain or mutable fixture is shared.

The rotation uses administrator sequence **7**, while the first global queue `UpdateId` on a fresh
harness is **0**. Making the values visibly different prevents a fixture from masking the most
dangerous identifier confusion in this phase: sequence numbers belong to roles and protect replay;
the queue id is global and identifies the entry a cancel removes.

### 5.2 One submission helper

`submit_action(harness, action, seq_no, signer_keys, signer_indices)`:

1. creates the upstream `SignatureSet` with `create_signature_set`;
2. builds a `SignedPayload`;
3. builds and submits the real SPS-50 envelope;
4. returns the measured height of the block containing the reveal.

The helper is local to the new file. Extracting it into `e2e-tests/src/test_harness.rs` would mix
admin-subprotocol policy into a generic harness and touch every shipped e2e for one new consumer.
Its documentation states why key material and signature indices are separate: the invalid-old-quorum
case intentionally signs for a live canonical index with the removed signer's key so the malformed
quorum reaches the ASM rather than failing in a desktop pre-check.

The rotation itself is encoded through the desktop domain codec:
`Action::MultisigUpdate { role: Authority::SecurityCouncil, … }` → `action_codec::encode_hex` →
upstream `MultisigAction::from_ssz_bytes`. This is the first on-chain proof of our tx type 15
mapping. Cancels and Defcon actions are constructed from upstream types because the desktop domain
does not compose either payload from a form.

### 5.3 Heights are measured, never counted

`submit_and_mine_tx` may mine up to ten blocks before finding the reveal. Therefore:

```text
reveal_height = measured block height returned by submit_action
activation_height = reveal_height + configured depth
blocks_to_mine = (activation_height + 1) - current tip, saturating at zero
```

The enacted path first mines to `activation_height - 1` and proves the update is still queued, then
mines the activation block and proves it applies there. The cancel path asserts
`cancel_height < activation_height`, then later asserts `tip > activation_height`.

The strict inequality is protocol, not style: the administration subprotocol processes mature
queue entries before transactions in the same block. A cancel at equality is too late — the
rotation applies first and the cancel is ignored as `UnknownAction`. These “wrong reason” guards
prevent an empty queue from passing because the update enacted before a late cancel.

There is no sleep, timeout poll or wall-clock assertion. `mine_block` waits for the ASM worker to
process every submitted block.

### 5.4 Enacted path

`e2e_council_rotation_enacts_and_changes_who_can_trigger_defcon`:

1. snapshot both roles' configs and sequence counters;
2. submit the desktop-encoded rotation at administrator sequence 7 with `A0 + A1`;
3. assert exactly one matching tx type 15 entry is queued, its `UpdateId == 0` and its measured
   `activation_height == reveal_height + depth`; the council config is unchanged, administrator
   `last_seqno == 7`, and council `last_seqno` is unchanged;
4. mine to `activation_height - 1` and reassert queued plus unchanged; mine the activation block;
5. assert the queue entry is gone; the council set is exactly `{C0, C2, C3}` at threshold 2;
   administrator config is unchanged; administrator `last_seqno == 7`; council `last_seqno` is
   still unchanged.

Then AC 7b uses Defcon 1 because it has depth zero and its observable bridge effect is immediate:

6. submit a Defcon 1 at council sequence 1 with `C0` plus the **removed** key `C1`, assigning `C1` to
   the canonical slot now occupied by `C3`; mine its reveal and assert the safe harbour remains off,
   no Defcon is queued and the council sequence stays unchanged;
7. submit the same Defcon 1 at sequence 1 with the valid new quorum `C0 + C3`; assert the safe
   harbour turns on and the council sequence advances.

The second submission deliberately uses the **same sequence 1**, not a retry sequence. The invalid
attempt and valid counter-case share action and sequence. The second succeeding proves
the first was rejected rather than merely delayed, and that the new set — not a stale cache — is
authoritative. It is stronger than checking a verification helper before broadcast.

### 5.5 Cancelled path

`e2e_council_rotation_cancelled_never_changes_the_council`:

1. snapshot the council config;
2. submit the same rotation at administrator sequence 7;
3. recover the exact queued `UpdateId` and `UpdateAction`;
4. build `CancelAction::new(id, queued_action)` and submit it with `A0 + A1` at administrator
   sequence 8;
5. assert `cancel_height < activation_height` and that the cancel removed the queue entry;
6. mine to one block past the original activation height;
7. assert the queue remains empty, the council config equals the snapshot, the added signer is
   absent, administrator `last_seqno == 8`, council `last_seqno` is unchanged, and the cancel did
   not increment the global next-update id.

Embedding the exact queued action is required: upstream resolves the authorizing role from it and
checks equality against the queue entry. Reconstructing an equivalent-looking value would weaken
the test around the identity the cancel signs.

## 6. Tests deliberately not written

- No second Phase 2 truth table or ASM-backed backend integration test.
- No component test: the repository has no DOM runner.
- No `readFileSync` assertion on copy or wiring.
- No WebDriver flow; Phase 5 owns a durable selector and any UI finding.
- No test of Constraint 4's swallowed upstream apply error.
- No zero-depth tx type 15 case; this phase needs a non-zero queue to exercise cancel.
- No parametrized enacted/cancelled table: their assertions are intentionally opposite.

## 7. TDD and commit migration

Every commit compiles, formats and leaves all previously green tests green. A commit never repairs
the one before it.

| # | Commit | Contents |
|---|---|---|
| 0 | `docs(security-council): specify V3 Phase 4` | This document, reviewed before production work |
| 1 | `test(e2e): add council rotation enactment path` | Fixture, local submit/state helpers and queued→enacted assertions |
| 2 | `test(e2e): prove rotated council membership takes effect` | Removed-member rejection plus valid-new-quorum counter-case |
| 3 | `test(e2e): add cancelled council rotation path` | Exact queued action, measured heights and unchanged-config assertions |
| 4 | `test(orchestrator): pin council rotation isolation` | Tx 15 test fixture; AC 9 cancel refusal and AC 10 list/detail isolation |
| 5 | `fix(orchestrator): conceal foreign proposal existence` | AC 10 audit finding: foreign and missing ids both return `NotFound`; correct V2 cancel diagnostic |
| 6 | `docs(security-council): close V3 Phase 4` | Honest pending-manual status, phase board, stage board and slice board |

The e2e is test-only but it is not “red until production appears”: Phases 1–3 are the production
implementation. Its first run is the acceptance RED/GREEN gate for their composition. Any failure
is reduced to the smallest responsible layer before code changes. If production code is required,
this document is updated first with the discovered gap and the new GREEN commit.

## 8. Blast radius

- One new e2e file, two independent regtest harnesses and roughly a few dozen processed blocks.
- One test-only tx type 15 fixture and two focused application tests.
- One narrow read-side behaviour change in `orchestrator-be`: foreign proposal ids now return
  `NotFound`, matching unknown ids. Write-side authority errors are unchanged.
- Inline backend tests and a test-only action fixture change. Zero diff in `desktop-app` and the
  pinned ASM.
- No dependency, schema, route, DTO or persisted-data change.

The invalid Defcon transaction is still mined on Bitcoin; “rejected” means the administration
subprotocol does not mutate its state or consume the council sequence. This is expected SPS-50
behaviour and the test asserts state, not mempool acceptance.

The cost budget is two harness boots, two rotations, one cancel and two Defcon submissions. Three
consecutive focused runs passed in **10.22 s, 7.24 s and 8.10 s** (mean 8.52 s); the full workspace
suite also passed. This is the initial runtime baseline, not a permanent timing assertion.

## 9. Verification

Focused:

```bash
cargo test -p alpen-multisig-e2e-tests --test e2e_council_rotation -- --nocapture
cargo test -p orchestrator-be council_rotation
```

Full local CI:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app
npm run format:check
npm run lint
npm run build
npm run test:unit
```

Structural:

```bash
git diff develop -- orchestrator-be/src
git diff --stat develop -- desktop-app/src desktop-app/src-tauri/src
git grep -n 'sleep\\|tokio::time' -- e2e-tests/tests/e2e_council_rotation.rs
```

Review the backend hunks: the only production addition is the narrow read-visibility guard in §4.4;
the rest is below `#[cfg(test)]`. The desktop diff is empty and the anti-flake search returns no
match.

## 10. Manual walk and close-out

Run the enacted and cancelled journeys from separate clean-stack resets:

1. Administrator creates the council rotation, reaches quorum and broadcasts it.
2. It reads Awaiting enactment with a countdown to the live tx type 15 depth; council config is
   unchanged.
3. Enacted path: mine the depth; proposal reads Enacted and the new council config is visible.
4. After a reset, cancelled path: create the rotation again, cancel inside the window; target reads Canceled and never
   Enacted after its original activation height.
5. A Security Council session sees neither rotation in its list and cannot open its action id.
6. Export/import a quorum bundle on `/manual` under `strata_admin`; it is named Security Council
   signer update, shows the administrator's signer roster, and refuses import under
   `security_council`.
7. Exercise the external-RPC recovery promised by AC 13: after composing the transactions, capture
   the raw commit and reveal hexes, broadcast them in order with `bitcoin-cli sendrawtransaction`,
   mine the reveals and verify the ASM processes tx type 15.

Findings introduced by this phase are fixed here. UX improvements or pre-existing copy/selector
debt are recorded for Phase 5 rather than smuggled into the e2e PR.

Close-out updates:

- this build plan's status and Phase 4 row;
- the functional contract's status;
- `security-council.md` stage and V3 slice boards.
- `e2e-tests/README.md` test inventory.
