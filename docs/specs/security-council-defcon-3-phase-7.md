# Security Council — Defcon 3 (V2), Phase 7: The cancel, end to end

**Functional contract:** [`security-council-defcon-3.md`](./security-council-defcon-3.md) — SSOT for
*what* V2 must do. This document never overrides it.

**Build plan:** [`security-council-defcon-3-implementation.md`](./security-council-defcon-3-implementation.md)
§4 Phase 7. This document is that phase at implementation detail.

**Closes:** [AC 11](./security-council-defcon-3.md#11-the-cancel-is-signed-by-the-council-itself),
[AC 12](./security-council-defcon-3.md#12-a-cancelled-defcon-3-never-activates-the-harbour) and
[AC 14](./security-council-defcon-3.md#14-the-manual-fallback-works-for-both);
[Constraint 3](./security-council-defcon-3.md#3-a-cancelled-defcon-3-must-never-be-reported-as-enacted).

## 1. The change in one sentence

The cancelled path gets the only automated evidence it has ever had — `run_defcon3_canceled` against
a real regtest ASM — and the one place the build plan's *"no new code is expected"* turns out to be
wrong gets fixed: the offline route refuses to import a cancel at all, which is AC 14 failing, not
degrading.

## 2. What this phase is not

It is not the create flow (Phase 5) or the queued lifecycle (Phase 6), and it changes **no**
`orchestrator-be` code: §4 checks that claim path by path rather than asserting it. It does not build
an offline cancel *composer* — AC 14 says a cancel can be *aggregated and broadcast*, not composed
without an orchestrator. It does not touch the two-live-Defcon-3 ambiguity, which the contract
records as an edge case, and it does not add a details panel for a cancel on the sign view. Phase 8
owns whatever the manual walk finds.

## 3. Spec traceability audit

| Document | What Phase 7 takes from it |
|---|---|
| [`security-council-defcon-3.md`](./security-council-defcon-3.md) § Test Plan | The e2e's shape: queue → assert harbour off → cancel → mine → queue empty **and harbour still off** |
| Same, § Cancel creation | "No new code is expected" — the claim §4 audits |
| Same, § Manual fallback | "a Defcon 3, **and its cancel**" — the clause §6 discovers is unmet |
| Same, Edge Cases | Two live Defcon 3s stay a recorded ambiguity; the e2e keeps exactly one in flight |
| [`security-council-defcon-3-phase-4.md`](./security-council-defcon-3-phase-4.md) §4.7 | The out-of-band cancel has no observable distinction; this phase pins the **in-band** path |
| [`security-council-defcon-3-phase-1.md`](./security-council-defcon-3-phase-1.md) | Emitter and acceptor of a closed Zod union cannot be split across commits — restated in §9 commit 3 |
| [`security-council-defcon-3-phase-6.md`](./security-council-defcon-3-phase-6.md) §8 | Migration shape: the seam is opened in its own commit, before the test that uses it |
| [`cancel-approved-proposal.md`](./cancel-approved-proposal.md) | The desktop cancel journey, unchanged by this phase |

## 4. The build plan's bet, audited

The plan bet that no new code is needed, *"and if the phase discovers otherwise, that discovery is
the phase's most valuable output."* Checked layer by layer, the bet holds for AC 11 and AC 12 and
fails for AC 14.

| Question | Answer | Evidence |
|---|---|---|
| Does a queued Defcon 3 stay `Approved`, so the cancel is admissible? | Yes | `confirm_reveal_if_mined` passes `None` as the proposal status and writes only `broadcast_status` (`orchestrator-be/src/application/proposals.rs:397-406`); `supersede_if_seq_no_consumed` returns early while the entry is in the queue (`:466-467`) |
| Does `create_cancel_proposal` admit a Defcon 3 and refuse a Defcon 1? | Yes | Gates on `require_proposal_authority`, `status == Approved`, and `lock_period_for_action(..) > 0` (`:760-779`). Defcon 1 is depth `0`; Defcon 3 is not |
| Is the cancel filed under the council? | Yes | `authority: target.authority` (`:792`), and a Defcon 3 target is the council's — which is also what upstream requires, since a cancel's authorizing role is the role of the update it cancels |
| Does the cancel hex build for a Defcon 3 target? | Yes | `encode_cancel_hex_for_target` accepts any `MultisigAction::Update` (`desktop-app/src-tauri/src/infrastructure/action_codec.rs:88-97`) |
| Does the signer see the cancel's four canonical lines? | Yes | `render_signing_message` and `compute_sighash` decode with the **upstream** `MultisigAction`, which has a `Cancel` variant (`src-tauri/src/infrastructure/signing.rs:59-61,138-144`) |
| Is a cancelled target written `Canceled` rather than `Superseded`? | Yes | `cancel_reached_chain` (`application/proposals.rs:515-524`) plus `enact_cancel`'s atomic `Approved → Canceled` (`infrastructure/memory_repo.rs:190-218`, `postgres_repo.rs:443-`), both landed in PR #527 |
| **Can a council signer aggregate and broadcast a cancel offline?** | **No** | §6 |

### 4.1 The one place it fails

`/manual` refuses any hex whose decoded kind is `unknown`
(`desktop-app/src/domain/manual-proposal/hooks/use-manual-proposal.ts:201-204` for the typed form and
`:280-283` for the JSON bundle), and a `MultisigAction::Cancel` decodes to
`DecodedAction::Unknown` because the desktop's **domain** `Action` enum has no `Cancel` variant
(`src-tauri/src/commands/action_builder.rs:60-62`). So on the one route built for *"the orchestrator
is unavailable"*, the cancel cannot even be imported.

This is a rejection, not a bad label. The Rust underneath is already generic — `prepare_broadcast_manual`
and `broadcast_manual` re-decode `MultisigAction` for `action.tag()` and never consult the domain
enum — so lifting the gate is all that stands between the current behaviour and AC 14.

## 5. The e2e — `run_defcon3_canceled`

### 5.1 Heights are measured, never counted

`submit_council_action` mines **a variable number of blocks**: one for the commit
(`e2e-tests/tests/e2e_defcon_probe.rs:275`) and then up to ten inside `submit_and_mine_tx` until the
reveal is included (`e2e-tests/src/test_harness.rs:178-192`). Any arithmetic a caller does from a
guess about where the tip ended up is a race.

`e2e_cancel_proposal.rs:163-166` does the implicit thing — `mine_blocks(DEPTH + 1)` — and it is
correct there only because its target's reveal happens to be the tip at that moment. **Do not copy
it.** `submit_council_action` therefore returns the height of the block its reveal landed in, and the
test computes `activation_height = reveal_height + depth` from a measured value.

This is a deliberate departure from the build plan's *"mine exactly `depth` blocks"*: the plan's
sentence describes the intent, and measuring is how the intent survives a helper that mines a
variable amount.

### 5.2 The seam

```rust
/// Sign `action` at `seq_no` with both security-council keys, drive it through commit → reveal,
/// and return the height of the block the reveal landed in.
///
/// The height is returned rather than counted by the caller: this function mines one block for the
/// commit and then up to ten until the reveal confirms, so any arithmetic done from a caller's
/// guess about the tip is a race.
async fn submit_council_action(
    harness: &AsmTestHarness,
    fixture: &SignerUpdateEnactedFixture,
    admin_section: &serde_json::Value,
    action: &MultisigAction,
    seq_no: u64,
) -> anyhow::Result<u64>
```

The body is unchanged except that `let seq_no = fixture.seq_no;` (`:196`) is deleted and the final
line becomes a `get_block_height` on the block hash `submit_and_mine_tx` already returns. The
`Reader` trait that provides it is already imported (`:27`) and the harness calls it the same way
(`test_harness.rs:128`).

Both existing callers become `let _ = submit_council_action(&harness, fixture, &admin_section, &action, fixture.seq_no).await?;`.
**`run_defcon3`'s `mine_blocks(depth)` stays exactly as it is** — it is correct, because there the
reveal block *is* the tip when the helper returns.

### 5.3 The test

```rust
async fn run_defcon3_canceled(fixture: &SignerUpdateEnactedFixture) -> anyhow::Result<()> {
    let admin_section = parse_admin_section(fixture.admin_section_json);
    let depth = defcon3_confirmation_depth(&admin_section) as u64;
    anyhow::ensure!(depth > 0, "the fixture must configure a non-zero defcon3 depth: at 0 there is no window to cancel in");

    let harness = /* AsmTestHarnessBuilder, as in run_defcon3 */;
    anyhow::ensure!(!bridge_safe_harbour_activated(&harness)?, "safe harbour must start deactivated");

    // 1 — queue a Defcon 3.
    let action = MultisigAction::Update(UpdateAction::Defcon3(Defcon3Update));
    let reveal_height = submit_council_action(&harness, fixture, &admin_section, &action, fixture.seq_no).await?;
    // `process_queued` drains at `activation_height <= tip`, and the activation height is the
    // reveal height plus the depth.
    let activation_height = reveal_height + depth;

    let (queued_id, queued_action) = queued_defcon3(&harness)?
        .ok_or_else(|| anyhow::anyhow!("Defcon 3 must sit in the admin queue before its depth elapses"))?;
    anyhow::ensure!(!bridge_safe_harbour_activated(&harness)?, "safe harbour must stay off while the Defcon 3 is queued");

    // 2 — cancel it, signed by the same council.
    //
    // A cancel's authorizing role is the role of the update it cancels, so a Defcon 3 cancel is a
    // council action. Upstream consumed the council seqno when it *accepted* the Defcon 3 at the
    // reveal — not when the queued entry matures — so the next valid seqno is `+ 1`.
    //
    // The queued `UpdateAction` is embedded verbatim rather than reconstructed: the upstream
    // handler resolves the role from it and checks it for equality against the queue entry.
    let cancel = MultisigAction::Cancel(CancelAction::new(queued_id, queued_action));
    let cancel_height = submit_council_action(&harness, fixture, &admin_section, &cancel, fixture.seq_no + 1).await?;
    anyhow::ensure!(
        cancel_height <= activation_height,
        "the cancel must land inside the window (landed at {cancel_height}, activation {activation_height}); \
         past it upstream rejects it as UnknownAction and the queue would be empty because the update enacted"
    );

    // A cancel has depth 0, so the entry is gone in the cancel's own reveal block.
    anyhow::ensure!(queued_defcon3(&harness)?.is_none(), "the cancel must remove the Defcon 3 from the queue");
    anyhow::ensure!(!bridge_safe_harbour_activated(&harness)?, "the cancel must not activate the harbour it removed");

    // 3 — take the tip past the height the Defcon 3 would have activated at. Measured, not assumed.
    let tip = harness.get_chain_tip().await?;
    let _ = harness.mine_blocks((activation_height + 1).saturating_sub(tip) as usize).await?;
    let tip = harness.get_chain_tip().await?;
    anyhow::ensure!(tip > activation_height, "tip {tip} must have passed the original activation height {activation_height}");

    // 4 — Constraint 3: leaving the queue is not evidence of enactment.
    anyhow::ensure!(queued_defcon3(&harness)?.is_none(), "the queue must stay empty past the activation height");
    anyhow::ensure!(!bridge_safe_harbour_activated(&harness)?, "a cancelled Defcon 3 must never activate the safe harbour");

    // Both actions were accepted by the council, not silently dropped. Never `==`: the council may
    // accept further actions, exactly as Constraint 2 says. Written `last > fixture.seq_no` rather
    // than `last >= fixture.seq_no + 1`, which clippy's `int_plus_one` rejects; same predicate.
    let last = council_last_seqno(&harness)?;
    anyhow::ensure!(last > fixture.seq_no, "the council seqno must have consumed the cancel (is {last})");

    Ok(())
}
```

`queued_defcon3` is a small helper returning `Option<(u32, UpdateAction)>` — the queue `UpdateId` and
the entry's action — and it asserts that at most one Defcon 3 is queued, because two are
byte-identical and the contract records that ambiguity rather than defining it.

### 5.4 The arithmetic

With `FAST_ENACTMENT`, `confirmation_depths.defcon3 = 5`
(`e2e-tests/src/fixtures/signer_update_enacted.rs:139`).

- The Defcon 3 reveal lands at measured height `H`; upstream sets `activation_height = H + 5`.
- The cancel is submitted immediately: commit at `H+1`, reveal at `H+2`. `H+2 ≤ H+5` holds with three
  blocks to spare, and the `cancel_height <= activation_height` assertion is what turns a future
  depth reduction into a readable failure instead of a flake.
- Then mine `(H + 6) − tip` blocks, normally `4`, reaching `tip = H + 6 > H + 5` — one block past the
  height the entry would have drained at.
- Total new chain work: roughly **8–10 regtest blocks** and one extra harness build, about the cost
  of the existing `run_defcon3` plus one submission.

### 5.5 Anti-flake

No `sleep`, no polling, no wall clock. `mine_block` submits each block to the ASM worker and blocks
until it is processed (`test_harness.rs:134-137`), so state is settled whenever the call returns.
Every height is read from the chain. The two "wrong reason" guards — the cancel landed inside the
window, and the tip really passed the activation height — turn a silent false green into a named
failure. The existing `bitcoind`-in-PATH skip is reused verbatim.

**If the assertion "the queue is empty right after the cancel's reveal block" fails, do not weaken it
into a loop and do not add a sleep.** It would mean upstream defers removal to a later block: move
only that assertion below step 3, leave step 4 untouched, and record the finding here. That is the
only authorised response.

## 6. A cancel becomes decodable, end to end (AC 14)

`decode_action_hex` grows a cancel branch **before** it reaches the domain decode, because a cancel
hex fails `decode_hex` and would otherwise land in the `Err(_)` group that answers `Unknown`
(`action_builder.rs:60-62`). The branch is built on a new inverse of the encoder:

```rust
/// The `(target_update_id, target update hex)` carried by a cancel action, or `None` when the hex
/// is not a `MultisigAction::Cancel`.
///
/// The inverse of `encode_cancel_hex_for_target`. It decodes at the upstream `MultisigAction`
/// layer rather than through the domain `Action`, which has no `Cancel` variant: a cancel is an
/// envelope around an update, not an action the desktop ever builds from a form.
pub fn decode_cancel_target_hex(action_hex: &str) -> Result<Option<(u32, String)>, CodecError>
```

The DTO gains `Cancel { target_update_id: u32, target_action_hex: String }`, the Zod
`decodedActionSchema` gains the matching member, `ACTION_TYPE_BY_KIND` gains `cancel: 'cancel'`, and
`inferProposalTypeLabel` gains an `actionType === 'cancel'` arm — needed because
`manual-sign-collect.tsx:56` hardcodes `kind: 'update'` on the synthetic proposal, so the existing
`kind === 'cancel'` arm never fires on the offline route.

**In the same commit**, `encode_cancel_hex_for_target`'s parameter is renamed `target_seq_no` →
`target_update_id`. Its only caller passes the queue `UpdateId`
(`commands/action_builder.rs:298-307`), the doc comment says "seq_no", and the two functions only
read as inverses once the name is honest.

**This needs a `CancelActionDetails` arm, and the first draft of this spec got that wrong.** It
reasoned that the ternary chain at `sign-proposal-view.tsx:184-192` ending in `null` merely left a
cancel with less on screen than an update. It is worse than that: today a cancel decodes to
`unknown` and renders `UnknownActionDetails`, so teaching it a kind of its own **removes** the only
payload the signer sees, under copy that tells them to *"review the action details above"*. The
manual walk caught it. The arm shows the queue `UpdateId` and the wrapped update's hex — strictly
more than the raw cancel hex it replaces.

## 7. The cancel screen names what it is cancelling

`CancelTargetSummary` renders `changeLabel ?? \`Proposal #${proposal.seqNo}\``
(`domain/cancel-proposal/components/cancel-target-summary.tsx:32`), and `changeLabel` is only
populated for a `multisig_update` (`use-decoded-proposal.ts:50`). So the card headed *"Proposal being
cancelled"* identifies a Defcon 3 as a bare **Proposal #2** — on the one screen where a council
signer decides whether to cancel the lever that sweeps the bridge.

The repo already holds both answers: `buildProposalTitle` (`src/lib/proposal-title.ts:18`) falls back
to `derivedProposalLabel`, which reads `Proposal #N - Defcon 3` through `inferProposalTypeLabel`
(`src/lib/proposal-type-label.ts:11`). The fix is to prefer `changeLabel` and fall back to
`buildProposalTitle(proposal)` instead of the hand-rolled string.

Untested by construction — no DOM runner — and covered by the manual walk. It changes the card for
every authority, which is a fix for them too.

## 8. Tests

| # | Claim | Assertion |
|---|---|---|
| 1 | A cancelled Defcon 3 leaves the queue (AC 12) | no `UpdateAction::Defcon3` in `admin.queued()`, checked after the cancel's reveal **and** past the activation height |
| 2 | …and never activates the harbour (Constraint 3) | `!safe_harbour().is_activated()` while queued, right after the cancel, and with `tip > activation_height` — the third is what the test exists for |
| 3 | The cancel really landed inside the window | `cancel_height <= activation_height`; without it an empty queue could mean the opposite outcome |
| 4 | The tip really passed the original activation height | `tip > reveal_height + depth`, both terms measured |
| 5 | The council itself authorised the cancel (AC 11) | signed by the same two council mnemonics at `seq_no + 1`, and `council_last_seqno() > seq_no` afterwards |
| 6 | A cancel hex decodes back to the update it wraps | `decode_cancel_target_hex(encode_cancel_hex_for_target(defcon3_hex, 7))` is `Some((7, defcon3_hex))`; the same call on a plain Defcon 3 hex is `None` |
| 7 | `decode_action_hex` answers `Cancel`, not `Unknown` | the exact gate `/manual` fails on today |
| 8 | The Zod schema accepts the new kind | `decodedActionSchema` parses a `cancel` member in `src/api/ipc-schemas.test.ts` — the Rust/TS divergence guard |
| 9 | The offline route names a cancel *Cancel* | `actionTypeFromDecoded({ kind: 'cancel', … })` is `'cancel'`, and `inferProposalTypeLabel({ actionType: 'cancel', kind: 'update' })` is `'Cancel'` — `kind: 'update'` on purpose, per §6 |

**Not tested, deliberately.**

- **An HTTP round trip of the cancel creation.** Its gate is `lock_period_for_action(..) > 0`, which
  Phase 3 already unit-tests through the closure seam; the handler is a thin map.
- **An ASM-backed integration test inside `orchestrator-be`.** Phase 4 recorded it as the flakiest
  test the repo could own, and the e2e proves the chain half.
- **The desktop cancel journey.** No DOM runner (`@testing-library/react` is not installed), and a
  `readFileSync` test pins a phrasing rather than a behaviour. Manual walk, §11.
- **Two live Defcon 3s.** Pinning the first-match resolution would encode a recorded ambiguity as
  expected behaviour.
- **A cancel arriving after the activation height.** It needs a race against maturity to construct
  and asserts an upstream rejection the contract calls impossible by construction; assertion 3 is the
  cheap half of that coverage.
- **Any source-text wiring test.** Nothing here threads a prop.

## 9. Migration — seven commits, each atomic

| # | Commit | Why it is safe on its own |
|---|---|---|
| 0 | This spec | Docs only |
| 1 | `submit_council_action` takes a `seq_no` and returns the reveal height | Pure refactor: both callers pass `fixture.seq_no` and discard the height, so behaviour is byte-identical and the two shipped Defcon tests are the proof |
| 2 | `run_defcon3_canceled`, its `#[tokio::test]` wrapper and `queued_defcon3` | Adds a test; touches nothing shipped |
| 3 | A cancel decodes end to end, plus the `target_update_id` rename | Emitter and acceptor **must** land together: `decodedActionSchema` is a closed `z.discriminatedUnion`, so a Tauri emitting `kind: 'cancel'` against a schema that rejects it fails the parse — Phase 1's argument, restated |
| 4 | `CancelTargetSummary` names its target | Swaps a hand-rolled fallback for two functions the repo already tests; nothing else reads the component |
| 5 | Close-out: the `Status:` headers, the stage and slice boards, row 7 ✅ | Docs only; by Phase 6 precedent this is the commit that ships the phase |
| 6 | `CancelActionDetails` on the sign view | Found by the manual walk: commit 3 removed the panel a cancel used to get through `UnknownActionDetails`. A regression of this phase, fixed inside it |

Commit 1 precedes commit 2 for Phase 6's reason: the seam is opened first, so the test that uses it
does not double as the justification for changing a shipped helper. Commit 3 is independent of 1–2;
it is ordered after so the phase's stated deliverable lands first.

## 10. Blast radius

- **`e2e_defcon_probe.rs` gains one test that really runs in CI.** The workflow installs Bitcoin Core
  29.0 and runs `cargo test --workspace`, so this is roughly 8–10 extra regtest blocks and one extra
  harness build — the file's runtime about doubles.
- **`submit_council_action`'s signature changes** and it is used by the shipped Defcon 1 test.
  Mechanical and compiler-checked.
- **`decode_action_hex` returns a new kind for hexes that previously answered `unknown`.** Consumers:
  `use-decoded-proposal.ts` (reads only `multisig_update`, unchanged), `use-manual-proposal.ts`
  (**this is the fix**), and `sign-proposal-view.tsx`, which gains a `cancel` arm in commit 6 —
  without it the new kind would have silently removed the raw-hex panel a cancel used to get. One
  path unblocks; none regresses.
- **`inferProposalTypeLabel` gains an arm** read by the dashboard, the detail screen and the sign
  header — but only for `actionType === 'cancel'`, which orchestrator-backed cancels already reach
  through `kind === 'cancel'` on the line above. No existing row changes label.
- **`CancelTargetSummary` changes for every authority**, not only the council: a multisig-update
  target keeps its `changeLabel` and gains the author's title when there is one.
- **No `orchestrator-be` change**, no protocol rule, no new dependency, no new component, no new
  predicate. `git diff --stat develop -- orchestrator-be/` must be empty.

### Debt this phase records rather than fixes

- **A cancel renders no details panel on the sign view.** `sign-proposal-view.tsx:184-192` falls
  through to `null` for the new kind. Cosmetic, and Phase 8's if the manual walk raises it.
- **`manual-sign-collect.tsx:56` hardcodes `kind: 'update'`**, so on the offline route a cancel is a
  cancel only by its `actionType`. Fixed at the pure-label layer here, not in the component.
- **The out-of-band cancel remains indistinguishable** from enactment when the harbour was already
  on — Phase 4 §4.7, unchanged.

## 11. Verification

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cargo test -p alpen-multisig-e2e-tests --test e2e_defcon_probe -- --nocapture
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

Two structural checks that the phase stayed inside its scope:

```bash
git diff --stat develop -- orchestrator-be/            # empty: this phase adds no backend code
git grep -n "fixture.seq_no" e2e-tests/tests/e2e_defcon_probe.rs
```

The second must show the cancel's `fixture.seq_no + 1` — the one number this phase can get wrong
without failing to compile.

`npm run test:unit` discovers by glob (`desktop-app/scripts/run-unit-tests.mjs`), so no `package.json`
or `ci.yml` change is needed; commit 3 adds assertions to existing files, so the file count is
unchanged.

**Manual walk**, from the build plan §5 points 3, 5 and 7:

1. Create a Defcon 3 as a council signer, reach quorum, broadcast: Approved → Awaiting enactment with
   a countdown, harbour off.
2. Open the cancel screen. *"Proposal being cancelled"* **names the Defcon 3** — its title, or
   `Proposal #N - Defcon 3` — not a bare `Proposal #N`.
3. Sign and broadcast the cancel inside the window: the target reads **Canceled**, nothing reads
   *Enacted*, the harbour stays off.
4. Sign the cancel as the second signer: the sign view names it *Cancel* **and shows the update it
   cancels**, so the copy telling the signer to review the action details above points at something.
5. Paste the cancel's `actionHex` into `/manual` with the council authority and its seqno: **it
   imports** — this is the AC 14 regression — the header names it *Cancel*, and it signs and
   broadcasts.
6. A Defcon 1 created in the same session still offers no countdown and no cancel affordance.
