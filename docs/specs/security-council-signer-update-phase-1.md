# Security Council — Signer Update (V3), Phase 1: `council_signer_update` is a readable type

**Functional contract:** [`security-council-signer-update.md`](./security-council-signer-update.md) —
SSOT for *what* V3 must do. This document never overrides it.

**Build plan:** [`security-council-signer-update-implementation.md`](./security-council-signer-update-implementation.md)
§4 Phase 1. This document is that phase at implementation detail, and §6 records the one place it
supersedes it.

**Closes:** [AC 5](./security-council-signer-update.md#5-the-action-is-distinguishable-from-an-administrator-signer-update),
and the Rust half of [Constraint 2](./security-council-signer-update.md#2-the-target-comes-from-the-action-never-from-the-session).
It is also a prerequisite of Phases 2, 3 and 4, each of which needs `council_signer_update` to be a
legal `ActionType` merely to write a fixture.

## 1. The change in one sentence

`council_signer_update` becomes a legal value everywhere a proposal is **read** — the codec, both IPC
boundaries, the type label and the offline bundle — while no path through the UI can author one.

## 2. What this phase is not

It is not the create flow. No menu entry in `ACTION_TYPES_BY_AUTHORITY`, no widening of the builder's
`role` union (`api/action-builder.ts:6`), no validator, no form retarget, no no-op rule. Those are
Phase 3, and §7 shows why their absence is structural rather than a matter of remembering.

It is not enactment. `is_proposal_enacted_on_asm` still resolves one role from the proposal's
authority, so a broadcast council rotation parks at Approved — Phase 2. It is not the cancel or the
e2e — Phase 4. `orchestrator-be/` and `e2e-tests/` are untouched by this phase.

## 3. Why the commit order runs TypeScript before Rust

The build plan says emitter and acceptor "cannot be split across two PRs". That is true, and the
reason is on the wire rather than in either compiler: **no compile error crosses the Rust↔TypeScript
border.** A Rust-only commit builds, passes `cargo test --workspace`, and leaves `npm run build`
green.

| Order | Consequence |
|---|---|
| Rust before TypeScript | **Broken.** `actionType` is a closed `z.enum` (`ipc-schemas.ts:48-56`) and the listing parses with `z.array(proposalSchema)` (`api/proposals.ts:120`), so one `council_signer_update` row fails the parse of **every proposal in the same list**, not just its own. |
| TypeScript before Rust | Inert, but unverifiable — the enum only widens, and nothing emits the value yet. |

So commit 3 precedes commit 5 as a **rule**, not a preference. §5's table orders every commit by which
side tolerates the other's absence.

**What this phase does *not* fix, contrary to a tempting reading:** nothing is broken today. A council
rotation hex fails to decode, so `action_type_from_hex`'s `Err(_)` arm
(`src-tauri/src/commands/proposals.rs:185`) answers `"unknown"` — a value the closed enum accepts. An
externally created council rotation lists as an *Unknown* row with its raw payload; it never emptied a
list. The wire argument above orders *this phase's* commits; it does not describe a live outage.

## 4. Where the compiler insists, and where it does not

Adding a fourth authority to the codec is **not** an `E0004`. The inner `match update.role` inside
`to_strata_action` (`action_codec.rs:107-124`, catch-all at `:121-123`) ends in a catch-all:

```rust
other => Err(CodecError::UnsupportedAuthority(format!(
    "encoding not implemented for authority `{other:?}`"
))),
```

The *outer* `match action` has no wildcard, but this phase adds no `Action` variant (§5.1), so it
never fires. That is the shape of this whole phase: **almost nothing here is compiler-forced, and
every unforced step is therefore carried by a named test.**

| Site | Net | Owner |
|---|---|---|
| `action_codec.rs:107-124` encode arm | none — catch-all `other =>` | T1, T2 |
| `action_codec.rs:244-246` decode arm | none — the placeholder is a valid `Err` | T1, T3 |
| `commands/proposals.rs:175` `action_type_from_hex` | none — it matches `Action::MultisigUpdate(_)`, ignoring the role | T7, T8 |
| `lib/proposal-type-label.ts:3-14` | none — an `if`-chain ending in `return 'Unknown'` | T5 |
| `manual-proposal/model/action-type-from-decoded.ts:16-26` | none — the `Record` is exhaustive over `kind`, and `kind` does not carry the role | T6 |
| `api/proposals.ts:18-26` / `ipc-schemas.ts:48-56` | `tsc` **does** insist downstream: comparing an un-widened union against the new literal is a TS2367, which is why commits 4 and 5 require commit 3 | T4 |

## 5. Design decisions

### 5.1 No new `Action` variant — a council rotation is a `MultisigUpdate`

`Authority::SecurityCouncil` already exists (`src-tauri/src/domain/authority.rs:9-15`) with the wire
string `"security_council"`, and `get_multisig_config` has mapped it to
`AuthRole::StrataSecurityCouncil` since V1 (`commands/asm_state.rs:46`). So
`Action::MultisigUpdate(MultisigUpdate { role: Authority::SecurityCouncil, … })` is representable
today; only the codec refuses it.

A distinct `Action::CouncilSignerUpdate` would duplicate `MultisigUpdate`'s four fields, add a second
`threshold_config_update_from_domain` call site, and fork `to_strata_action` into two matches over the
same payload — which is the crossed-arms hazard T1 exists to catch, made structural. It would buy no
protocol-level safety: upstream discriminates by SSZ union selector, which is derived from `role`
either way, and upstream's own type is a bare `ThresholdConfigUpdate`
(`asm/…/updates/strata_security_council_multisig.rs:14`).

### 5.2 No new `DecodedAction` kind — the target travels in `role`

A council rotation decodes to `{ kind: 'multisig_update', role: 'security_council', addKeys,
removeKeys, newThreshold }`. `decode_action_hex` already emits `update.role.as_str()`
(`commands/action_builder.rs`), so this needs no Rust change beyond §5.1.

This is the contract's own mechanism.
[Constraint 2](./security-council-signer-update.md#2-the-target-comes-from-the-action-never-from-the-session)
says *"`MultisigUpdate.role` means the authority being modified"*, and Scope excludes *"any notion of a
target authority on the `Proposal` row … the target lives in the action"*.

**A new kind would relocate the unchecked branch, not eliminate it.** The one argument for it is the
compiler net that `ACTION_TYPE_BY_KIND` was written to provide — *"a fifth decoded kind is a compile
error rather than a silent `multisig_update`"*. But to emit a new kind, `decode_action_hex` would need
`if update.role == Authority::SecurityCouncil` inside its existing `Ok(Action::MultisigUpdate(update))`
arm, and that `if` is not compiler-forced either, precisely because §5.1 adds no `Action` variant. The
check moves from TypeScript to Rust; it does not become checked.

What a new kind *would* cost is five narrowing sites that work today — `sign-proposal-view.tsx:184`,
`multisig-update-changes.ts:3` (`Extract<DecodedAction, { kind: 'multisig_update' }>`),
`use-decoded-proposal.ts:50,95`, `use-manual-proposal.ts:107` — and with them the
`MultisigUpdateDetails` panel, which is authority-agnostic (it renders `addKeys`/`removeKeys`/
`newThreshold` from the action alone) and which the contract calls *the reviewable artifact*. V2's
Phase 1 accepted a council action with no details panel because writing an honest one was destructive
copy it did not own; here the honest panel already exists. Keeping it is decision 5.2's dividend, and
it is why `sign-proposal-view.tsx` needs no edit in this phase at all.

`ACTION_TYPE_BY_KIND` stays exhaustive, so a genuinely new *kind* is still a compile error. It simply
stops being the whole answer, and T6 guards the part it no longer owns.

### 5.3 The Before/After signer table is suppressed, not retargeted

`use-decoded-proposal.ts:41` reads `getMultisigConfig(proposal.authority)` and builds the Before/After
signer table from it. For a council rotation the proposal's authority is `strata_admin`, so the table
would render the **administrator's** signers under a "Before" heading — a *false* display, and this
phase is what makes it reachable (§10: after commit 2, `/manual` stops refusing a council bundle).

V2's Phase 1 froze the rule this violates: each site must degrade to a missing affordance or a missing
warning, **never a false one**. So deferring is not available.

Retargeting is not available either, because it is not a retarget. At both affected sites the *same*
config read also feeds `allSigners` (`use-decoded-proposal.ts:47`, `use-manual-proposal.ts:105`), which
`ApprovalsList` uses for *who still has to sign* — the **authorizing** authority. Pointing that read at
the council would replace one false display with a worse one. A correct display therefore needs a
**second** config read at each site, ~100 lines across three hooks and one new async ordering, for a
proposal shape nothing can produce until Phase 3 — which already owns that rule
([Constraint 3](./security-council-signer-update.md#3-the-form-validates-against-the-targets-config-never-the-sessions)).

So Phase 1 suppresses, with one pure function and three guards:

```ts
// desktop-app/src/lib/multisig-update-target.ts
import type { DecodedAction } from '@/api/signing'

/**
 * The authority a decoded action modifies — the target, never the session (Constraint 2).
 *
 * `null` for every action that has no target authority. For the three self-rotating updates this
 * equals the proposal's own authority; for tx type 15 it does not, which is the whole of slice V3.
 */
export function multisigUpdateTargetAuthority(action: DecodedAction | null): string | null {
	return action !== null && action.kind === 'multisig_update' ? action.role : null
}
```

It returns the authority rather than a boolean deliberately: it is the same value Phase 3's read-side
retarget will hand to `getMultisigConfig`, so that phase changes the call sites and not this function.

The three guards — **`allSigners` is untouched in all three**:

> **One redundancy is compiler-forced.** `multisigUpdateTargetAuthority` returns `string | null`
> rather than acting as a type predicate, so TypeScript cannot narrow a `DecodedAction` through it
> to the variant carrying `addKeys`/`removeKeys`/`newThreshold`. The two hooks therefore keep an
> explicit `kind === 'multisig_update'` check beside the target check. It is logically implied by a
> matching target authority and exists only for the narrowing; both sites say so in a comment, so it
> does not read as a belt-and-braces condition a later reader might "simplify" away.

1. **`use-decoded-proposal.ts:50,95`** — introduce
   `const tableApplies = actionRes.ok && multisigUpdateTargetAuthority(actionRes.data) === proposal.authority`.
   The building branch becomes `tableApplies && configRes.ok`; the blanking branch becomes
   `actionRes.ok && !tableApplies`. Keeping `actionRes.ok` in the blanking condition preserves the rule
   documented at `:100-103`: a failed decode or a failed config read must **not** blank the table.
2. **`use-manual-proposal.ts:107`** — the existing early return inside the `signerSetChange` IIFE
   becomes `if (!actionRes.ok || multisigUpdateTargetAuthority(actionRes.data) !== importData.authority) return null`.
3. **`sign-screen.tsx:79-82`** — only the `enabled` argument of `useCurrentThreshold` changes, to
   `multisigUpdateTargetAuthority(decodedAction) === authorityFromRole(selectedRole)`. Argument 1 is
   left alone; retargeting *it* is Phase 3's. `currentThreshold` then stays `null`, `showThreshold`
   becomes `true`, and the threshold row is always shown — the direction
   `multisig-update-changes.ts:20-23` documents as the safe one, since *"hiding a value we cannot prove
   is unchanged would withhold information from the signer"*.

`useCurrentThreshold` belongs in this list even though its name suggests otherwise: its only consumer
compares it against `action.newThreshold` (`multisig-update-changes.ts:26`), so it is the **target's**
threshold, not the signing threshold, and it is subject to exactly the same rule.

**The guard is a no-op for every authority shipped so far.** For the three self-rotating updates
`role === proposal.authority` always holds. That zero-regression property is why the change belongs in
this phase rather than the next.

### 5.4 The label

`inferProposalTypeLabel` gains one arm returning **`'Security Council signer update'`** — PRD §5.5's
own term, parallel to the `'Signer update'` its `multisig_update` arm already answers for
`strata_admin`. The contract requires that a Strata Administrator never have to infer which of two
signer-update entries they are looking at, and the label is the surface where that is decided for a
proposal already created.

## 6. Where this phase departs from the build plan, and why

The build plan's §4 Phase 1 names *"the decoded-action schema"* among the sites to touch, and its
Tests paragraph names *"the new decoded-action kind"*. Decision 5.2 says there is no new decoded-action
kind, so `api/signing.ts`'s `DecodedAction` and `ipc-schemas.ts`'s `decodedActionSchema` are both
unchanged.

[`docs/specs/README.md`](./README.md) sets the order: layer 1 (the functional contract) outranks layer
2 (the implementation plan) — *"functional spec wins over implementation spec"*. The functional
contract never asks for a new kind; it asks (AC 5) that the action *"reports as a council signer
update, not as a generic `multisig_update`, everywhere the action type is shown — list, detail, sign
view and manual bundle"*. That is closed by `actionType` for the first three and by a role-aware
`actionTypeFromDecoded` for the fourth.

Recorded here rather than left as drift: V1 and V2 each needed a close-out PR for exactly this kind of
silent divergence.

## 7. Every site that sees a `council_signer_update`, and the phase that owns it

Each degrades to a missing affordance, never a false one — decision 5.3 is what makes that sentence
true rather than aspirational.

| Site | Behaviour with a council rotation | Phase |
|---|---|---|
| `lib/proposal-type-label.ts` | names it; without the arm it would read *Unknown* | 1 (T5) |
| `sign-proposal-view.tsx:184` details panel | the full add/remove/threshold panel, unchanged | — (decision 5.2) |
| `sign-screen.tsx:79` threshold row | always shown, never compared against the administrator's | 1 (§5.3) |
| `use-decoded-proposal.ts` Before/After table | absent | 1 suppresses; a later phase restores |
| `use-manual-proposal.ts` Before/After table | absent | same |
| `components/approvals-list.tsx` via `allSigners` | correct — the authorizing authority, which is what it means | — |
| `use-manual-proposal.ts:203,281` import gate | stops refusing the bundle (§10) | — |
| `proposal-status.ts:78,96` | `'approved'` and a countdown, which is what the contract's State Model wants — both predicates exclude only `defcon_1` | — correct by construction |
| `lib/safe-harbour-redundancy.ts:23` | not harbour-activating | — correct by construction |
| `orchestrator-be` enactment | parks at Approved. `asm_enactment.rs` returns `BadRequest` and `application/proposals.rs` `warn!`s and returns `Ok(())` **per proposal**, so it does not poison the reconciliation of anything else | 2 |
| `create-proposal/` | no menu entry, no validator, no schema value | 3 |

`src-tauri/src/infrastructure/signing.rs` does not participate: `render_signing_message` hands raw hex
to `SigningMessage::for_action`, and upstream already renders tx type 15's nine canonical lines today
(`asm/…/strata_security_council_multisig.rs:52-75`). The tripwire that the message differs from an
administrator signer update's is **Phase 3's by assignment, not by capability** — it is cheap here, but
this phase changes nothing on that path for it to catch.

## 8. Tests

Nine claims, each pinned where it lives. All of them are pure functions or the real codec — **no
mocks, no I/O, no clock.**

| # | Claim | Assertion | Where |
|---|---|---|---|
| T1 | The bytes are the **council's**, not a codec that merely agrees with itself | round-trip preserves `role: Authority::SecurityCouncil`; `update.update_tx_type() == UpdateTxType::StrataSecurityCouncilMultisigUpdate`; and `assert_ne!` against the **identical** `ThresholdConfigUpdate` encoded as a `StrataAdminMultisig` | `action_codec.rs`, beside `defcon_3_round_trips_and_encodes_upstreams_defcon_3_tx_type:338` |
| T2 | Our bytes are upstream's council encoding, positively | `encode(sample)` equals `MultisigAction::Update(UpdateAction::StrataSecurityCouncilMultisig(StrataSecurityCouncilMultisigUpdate::new(cfg))).as_ssz_bytes()` | `action_codec.rs`, beside `test_alpen_admin_multisig_encode_matches_direct_strata_ssz:419` |
| T3 | The decoded boundary names the target in `role`, with no new kind | `decode_action_hex(hex)` is `DecodedAction::MultisigUpdate { role: "security_council", … }` | `action_builder.rs`, beside `decode_defcon_3_names_the_action:252` |
| T4 | The zod boundary accepts it, **and one row does not take the list down** | `proposalSchema.safeParse({…, actionType: 'council_signer_update'})`; `z.array(proposalSchema)` parses a list mixing it with a `multisig_update` row | `api/ipc-schemas.test.ts`, extending the Defcon 3 block |
| T5 | The label names the **target**, and does not come from the authority | `inferProposalTypeLabel(proposal('council_signer_update', 'update', 'strata_admin'))` is `'Security Council signer update'`; `notEqual 'Unknown'`; `notEqual 'Signer update'`; the file's existing `multisig_update` asserts stay green | `src/lib/__tests__/proposal-type-label.test.ts` (exists) |
| T6 | `actionTypeFromDecoded` distinguishes by role | a `multisig_update` with `role: 'security_council'` is `'council_signer_update'`; with `role: 'strata_admin'` it is `'multisig_update'`; the file's existing `role: 'r'` assert stays green | `manual-proposal/model/__tests__/action-type-from-decoded.test.ts` (exists) |
| T7 | The DTO boundary names it | `action_type_from_hex(&None, &hex) == "council_signer_update"` | `commands/proposals.rs`, beside `action_type_from_hex_names_defcon_3:1110` |
| T8 | …and still discriminates | the same helper over a `role: StrataAdmin` update still answers `"multisig_update"` | same module, immediately after T7 |
| T9 | The target is read from the action, and only tx type 15 diverges | council update → `'security_council'`; administrator update → `'strata_admin'`; `defcon_3`, `vk_update`, `unknown` and `null` → `null` | `src/lib/__tests__/multisig-update-target.test.ts` (**new**) |

**T1's third assertion is the one with real value.** Unlike the two Defcon payloads, which are empty
unit structs, a council rotation and an administrator rotation carry a *byte-identical*
`ThresholdConfigUpdate` and are separated only by the SSZ union selector. A codec with the two encode
arms crossed would round-trip happily and send the wrong authority to a hardware signer.

**T5's `authority: 'strata_admin'` fixture is the highest-value single assertion in the table.** A
council rotation's proposal authority *is* `strata_admin`, and the existing `multisig_update` arm
derives its label from `proposal.authority`. The fixture proves the new label comes from `actionType`
instead. The test file's helper defaults `authority` to `'security_council'`, so the third argument
must be passed explicitly.

**T8 is not a duplicate of T7.** T7 proves the new value is emitted; T8 proves the old one still is.
The change binds a previously ignored field, so the regression it guards is real.

**Not tested, deliberately:**

- A separate `..._roundtrip_bytes` for the fourth authority. It would pin `hex::encode`/`decode`
  bijectivity — a language guarantee the contract's Test Plan excludes. T1 owns the round trip.
- A `decodedActionSchema.safeParse` assertion inside T4. Under decision 5.2 it is tautological:
  `role` is `z.string()` and the object already parses. It is replaced by a comment saying so, which
  is where §6's amendment is pinned at the code level.
- `proposalDisplayStatus` and `showsActivationCountdown`. Both already answer correctly because they
  exclude only `defcon_1`; asserting it would restate a negative already true for every other
  non-Defcon action type. V2 pinned the analogous claim in its Phase 6, not its Phase 1.
- Anything end to end, and any DOM or component test. There is no producer and no DOM runner, and
  hand-writing a hex fixture to assert one would restate what T1 owns.

## 9. Verification

The full [`AGENTS.md`](../../AGENTS.md) checklist, plus `npm run test:unit`, which CI runs
(`.github/workflows/ci.yml:155-156`) and the AGENTS.md snippet omits:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

`npm run test:unit` discovers `src/**/*.test.ts(x)` from disk (`scripts/run-unit-tests.mjs`), so the new
file needs no registration — confirm the final `N/N test files passed` count went up by **one** (T9 is
the only new file; T4, T5, T6 extend existing ones). Add no `test:*` script to `package.json`: that
enumerated list is the debt the runner exists to kill.

Structural checks that the phase stayed inside its scope:

```bash
# The creation vocabulary is untouched — turning its Records into compile errors drags Phase 3 in.
git diff --stat -- desktop-app/src/domain/create-proposal/                    # empty
grep -rn "council_signer_update" desktop-app/src/domain/create-proposal/      # nothing
grep -n "security_council" desktop-app/src/api/action-builder.ts              # nothing

# Constraint 2's Rust half landed, and the placeholder is gone rather than shadowed.
grep -n "a role can only modify its own config" desktop-app/src-tauri/src/domain/action.rs   # nothing
grep -rn 'UnsupportedVariant("StrataSecurityCouncilMultisig")' desktop-app/src-tauri/        # nothing

# Enactment is Phase 2; the e2e is Phase 4.
git diff --stat -- orchestrator-be/ e2e-tests/                                # empty

# The approvals list still reads the authorizing authority (§5.3's trap).
grep -n "setAllSigners\|allSigners:" \
  desktop-app/src/domain/proposal-detail/hooks/use-decoded-proposal.ts \
  desktop-app/src/domain/manual-proposal/hooks/use-manual-proposal.ts
```

**No manual walk**, and the reason is narrower than V2's. `build_admin_multisig_update_hex`
(`commands/action_builder.rs`) resolves its role through `Authority::from_wire` with no allow-list, so
once commit 2 lands the Tauri command *is capable* of producing a council-rotation hex. What no longer
exists is a **UI path** to it: `ACTION_TYPES_BY_AUTHORITY` answers `['defcon_1','defcon_3']` for
`security_council` and `['signer_update', …]` for `strata_admin` with the *session* authority passed to
the builder, `create-proposal.schema.ts:20`'s enum refuses the value, and `api/action-builder.ts:6`'s
union is closed. So the honest claim is *no UI path can author one* — not V2's stronger *nothing in the
application can produce one*, which is false here.

## 10. Blast radius

- **No new enum, no dead code, no `#[allow]`.** `Authority::SecurityCouncil` and the desktop's ability
  to read the council's config both predate this phase.
- **Nothing pins `UnsupportedVariant("StrataSecurityCouncilMultisig")`** as expected behaviour, so
  flipping the decode arm to `Ok` goes red nowhere.
- **A council rotation created outside the app** — by another orchestrator, or pasted through
  `/manual` — used to list as an *Unknown* row showing its raw hex. It now lists, labels and decodes as
  itself, and shows its full add/remove/threshold panel on the sign view. That is the user-visible
  gain.
- **`/manual` stops refusing a council bundle.** `use-manual-proposal.ts:203` and `:281` reject
  a decode of kind `unknown` with *"Unknown action kind — cannot decode this hex"*, which is what a
  council hex produced until now. Intended — and it is the reason §5.3's guard belongs in this phase
  rather than the next.
- **`sign-proposal-view.tsx` needs no edit**, unlike V2's Phase 1. Decision 5.2's dividend.
- **The Before/After signer table is absent** for a council rotation on the detail and manual screens,
  and the sign view's threshold row is always shown. Both by construction, both restored by whichever
  phase does the read-side retarget. No signer-set comparison is not an AC failure: AC 13's manual
  fallback is Phase 4's and does not require one.
- **A broadcast council rotation parks at Approved** with a per-proposal `warn!` until Phase 2.
- **One intermediate window inside the branch:** between commits 2 and 6 a council bundle imports and
  renders a false Before/After table. The phase squash-merges as a single commit, so that state never
  exists on `develop` — which is what the build plan's *"one atomic commit"* rule buys.

## 11. Two findings this phase records rather than fixes

Both came out of the review of this phase's own diff. Neither is repaired here, and both name the
phase that owns the repair.

### 11.1 `/manual` does not tie a bundle's authority to the action's authorizing role

`AUTHORITIES` (`use-manual-proposal.ts:27`) admits `security_council`, and nothing checks a bundle's
declared authority against the role upstream would require for the action it carries. So a
council-rotation bundle authored with `"authority": "security_council"` — the natural-looking label
for an action that rotates the council — imports cleanly: the membership gate passes (a council
member *is* a council signer), and this phase's guard passes too, because the decoded target and the
declared authority agree. Signatures are then collected from the council for an action whose
`authorized_role()` is `StrataAdministrator`, and the reveal is rejected on chain.

**Severity is bounded by the artifact the signer actually reviews.** Upstream renders
`Authorized By: Strata Administrator` on the same screen the device displays
(`strata_security_council_multisig.rs:52-75`), so a council signer is told, in the one place that
counts, that this is not theirs to authorize. The cost is a wasted transaction, not a signature over
something unreviewable — and the Before/After table in that scenario is *correct*, since the config
read and the target coincide.

**The hole is pre-existing and not council-specific** — no action type has its bundle authority
checked against its authorizing role. What this phase changes is that tx type 15 now reaches the
flow at all, which is why it is recorded here.

**Owner: Phase 4**, which holds AC 13 (the manual fallback). The honest fix derives the authorizing
role from the decoded action rather than trusting the bundle, and that derivation is exactly what
**Phase 2** builds for enactment ([Constraint 1](./security-council-signer-update.md#1-enactment-reads-two-roles-not-one)).
Writing it here would mean writing it twice.

### 11.2 The encode arm removes the last backstop on `build_admin_multisig_update_hex`

`build_admin_multisig_update_hex` (`commands/action_builder.rs:82-113`) resolves its role through
`Authority::from_wire` with no allow-list. Until commit 2 the codec's catch-all refused
`security_council` with `UnsupportedAuthority`; now it encodes. The Tauri command is therefore
*capable* of building a council rotation, and the only remaining barriers are on the TypeScript side
(§9).

One of those barriers is a cast rather than a type: `use-create-proposal.ts:94` writes
`authorityFromRole(selectedRole) as 'strata_admin' | 'sequencer_manager' | 'alpen_admin'`, which is
already untrue today because `selectedRole` can be the council. It is unreachable only because
`ACTION_TYPES_BY_AUTHORITY.security_council` offers no signer update.

**Owner: Phase 3**, and it is already in that slice's contract — its Verification checklist requires
that *"no `as` cast decides which authority a multisig update targets"*
([Constraint 2](./security-council-signer-update.md#2-the-target-comes-from-the-action-never-from-the-session)).
Recorded here because this phase is what made the cast load-bearing rather than merely untidy.
