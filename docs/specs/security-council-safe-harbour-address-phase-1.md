# V4 Phase 1 — From the screen to `Enacted`

> **Functional contract:** [`security-council-safe-harbour-address.md`](./security-council-safe-harbour-address.md)
> — SSOT for *what* V4 must do. This document never overrides it.
> **Build plan:** [`security-council-safe-harbour-address-implementation.md`](./security-council-safe-harbour-address-implementation.md)
> §4 Phase 1. This document is that phase at implementation detail, and §9 records where it
> supersedes it.
> **Ticket:** [#547](https://github.com/wakeuplabs-io/alpen-multisig/issues/547).
> **Closes:** AC 1, 1a, 2, 3, 3a, 3b, 3c, 4, 5, 6, 7, 7a, 7b, 8, 11, 12; Constraints 1-7.

## 1. The change in one sentence

A Strata Administrator can author a safe harbour address update from a Taproot address, see the
exact value their device will display while filling the form in, and the proposal reaches `Enacted`
only when the **bridge's** address actually equals the one proposed.

## 2. What this phase is not

It is not the cancel and it is not the e2e — those are Phase 2, and `e2e-tests/` has **zero diff**
here.

It is not a new signing message. The six canonical lines resolve through the same Rust renderer the
device signs over, and this phase adds a tripwire, not a renderer (§7.3).

It is not a protocol rule. Nothing added here refuses an action the chain would accept: the
already-activated harbour is stated, never blocked
([Constraint 1](./security-council-safe-harbour-address.md#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)).

It is not a change to the desktop's copy of `asm_enactment.rs` (§4.6), and it is not a change to any
generic mechanism — depth, cancelability, authorization and the lifecycle already answer for tx type
14, and this phase only adds the tests that pin that
([Constraint 7](./security-council-safe-harbour-address.md#7-everything-generic-already-answers-and-gets-tests-rather-than-changes)).

## 3. Why the screen and the enactment ship together

The obvious split — "make it creatable", then "detect enactment" — leaves a `develop` where a signer
can author a rotation, collect a quorum, pay commit and reveal fees, and watch the proposal sit at
`Approved` forever, because `is_proposal_enacted_on_asm` answers
`BadRequest("not implemented yet")` for it (`asm_enactment.rs:169-171`) and `reconcile_one` turns
every `Err` into a per-proposal warning that never resolves.

V1, V2 and V3 each refused that state for the same reason, and each ordered its phases so the
predicate landed **before** the create flow. This phase inverts the order — the screen comes first,
by request — and pays for it by keeping both inside one pull request. Commits 6 and 7 of §8 must
never be split across pull requests.

## 4. Design

### 4.1 The domain learns a destination, not a descriptor format

`desktop-app/src-tauri/src/domain/action.rs` gains `SafeHarbourDescriptor`, shaped like `EvenPubKey`
(`:17-57`) — a newtype over `[u8; 32]` holding the **x-only key of the P2TR output**, with a typed
error per rejection:

```rust
pub struct SafeHarbourDescriptor([u8; 32]);

pub enum SafeHarbourDescriptorError {
    Address(String),          // not parseable as a Bitcoin address
    WrongNetwork { expected: Network, found: Network },
    NotP2tr(String),          // parsed, but the output is not a taproot key-path output
    Hex(String),
    WrongLength(usize),
    NotP2trTag(u8),           // BOSD hex whose type tag is not 0x04
    InvalidPoint(String),
}
```

- `from_address(&str, Network)` — parse, require the network, require P2TR, keep the 32-byte
  program.
- `from_hex(&str)` — accept the BOSD form (`04` + 32 bytes), for the import path and for tests.
- `to_bosd_bytes() -> [u8; 33]` and `to_address(Network) -> String`.

**Why the domain does not hold a `Descriptor`.** `action_codec.rs` is by module contract *"the only
module that imports `strata_asm_*` / `strata_crypto` crates"*, and `bitcoin-bosd` belongs to that
same category. Holding the raw key keeps the protocol type on one side of the boundary and gives a
second benefit for free: `Descriptor::new_p2tr` takes `&[u8; 32]`, so no `bitcoin::Address` ever
crosses between our `bitcoin` and whichever one `bitcoin-bosd` was built against. The conversion
cannot break on a version bump because there is no shared type in it.

**Why validation lives in the domain and not in the form.** The form's job is to explain; the
domain's is to decide. Upstream decides last, via `SafeHarbourAddress::try_from`
([Constraint 5](./security-council-safe-harbour-address.md#5-p2tr-and-nothing-else)) — three gates,
narrowing, with the authoritative one at the end.

`Action::SafeHarbourAddressUpdate(SafeHarbourDescriptor)` joins the enum. Rust's exhaustive matches
name the rest.

### 4.2 The network is the process's, resolved once

`network_from_env()` (`infrastructure/network_env.rs:31-34`) is the canonical resolution in this
repository — its own doc comment says the active network is process-wide and deliberately **not** in
`NodeConfig`, so that commands do not each re-read the env var with subtly different fallbacks. The
builder command calls it once and passes it down.

A wrong-network address is refused
([Constraint 4](./security-council-safe-harbour-address.md#4-network-is-a-signal-not-a-protection))
even though it is not dangerous: BOSD carries no network, so the same key yields the same descriptor
bytes everywhere and only the HRP differs. It is refused because it is near-conclusive evidence that
the operator took the address from the wrong wallet.

### 4.3 The codec is two arms and one error

Encode: `Descriptor::new_p2tr(key)` → `SafeHarbourAddress::try_from` →
`UpdateAction::SafeHarbourAddress(SafeHarbourAddressUpdate::new(..))`. Decode: the inverse,
replacing the `UnsupportedVariant("SafeHarbourAddress")` at `:281-283`.

`Descriptor::new_p2tr` is fallible (it validates the key on the curve) and `try_from` is fallible
(it enforces P2TR), so `CodecError` gains one variant naming which one refused. Both are unreachable
for a value the domain built — the domain already validated the point and the type — which is the
point: the codec's gate exists so a hex arriving from outside a form cannot bypass it.

### 4.4 The form shows what the device will show

Ticket [#547](https://github.com/wakeuplabs-io/alpen-multisig/issues/547): *"Next to the address, the
form shows the exact value the signer's device will display, so the two can be compared before
signing."*

The mechanism already exists, and it is the one Defcon uses. `defcon-form-fields.tsx:38-56` resolves
the action hex, feeds `useDeviceSigningMessage(seqNo, actionHex)`, and renders the canonical message
inside the form. V4 reuses the pair with one difference — the hex depends on what the signer typed:

```ts
// domain/create-proposal/hooks/use-safe-harbour-action-hex.ts
export function useSafeHarbourActionHex(address: string): SafeHarbourActionHex
```

shaped like `use-defcon-action-hex.ts`, with two rules taken from it verbatim:

- **the state is cleared before each resolve**, so a message resolved for one address is never shown
  under another;
- **the failure is returned, not swallowed**, so a message that never resolved reads as broken and
  not as "you have not finished typing".

And one rule of its own: **it only calls the builder when the address parses.** A round trip per
keystroke would be an IPC storm; a round trip per *valid address* is one call per meaningful change,
and while the address is invalid the field's own error is what the signer needs to see anyway.

`useDeviceSigningMessage` already carries the pairing guard (`messageForInputs`) that refuses to
return a message resolved for different inputs, so a stale message cannot appear under a new
address even for one frame.

**What this phase deliberately does not copy from Defcon: the mirror into a form value.**
`defcon-form-fields.tsx:47-49` writes the resolved message into `defconMessage` so that "the signer
can see what they are signing" gates submission. That exists because the Defcon form has **no other
field that can fail** — without the mirror, an unresolvable message would not stop a signature. Here
the address field is already that gate: no valid address, no action hex, no submit. Adding a second
gate for this action and not for `vk_update` or `signer_update` — which have the same shape and no
such mirror — would be an asymmetry with no reason behind it.

### 4.5 The current destination is read once, and is load-bearing

`SafeHarbourStatusDto` (`commands/asm_state.rs:20-24`) gains `address_hex` and `address`;
`fetch_safe_harbour_activated` (`infrastructure/asm_status_rpc.rs:120-125`) becomes
`fetch_safe_harbour` and returns all three, from the same single `strata_asm_getStatus` it already
makes. The address is rendered with `network_from_env()`, so the form shows the destination the way
this deployment writes addresses.

`safeHarbourStatusSchema` (`api/ipc-schemas.ts:179-181`) declares both new fields. **Zod strips what
it does not declare**, so adding them in Rust alone would drop them in silence — the same failure the
`title` field's comment records at `:47`.

`useSafeHarbourActivated` keeps its current shape and its deliberate degradation to `false`: a node
that cannot answer must not stand between the council and the emergency lever. The **address**, in
contrast, is load-bearing: without it neither the Before/After nor the no-op rule of
[Constraint 6](./security-council-safe-harbour-address.md#6-rotating-to-the-address-already-installed-enacts-and-changes-nothing)
can answer, so the create form blocks submission while it is unavailable, with a message naming what
could not be read. Two reads, two different failure policies, because they answer two different
questions.

### 4.6 The desktop's copy of `asm_enactment.rs` is not touched, and that is a decision

The build plan says the two copies move together or desktop and backend disagree. That was V3's
situation, where both answered the same question about multisig updates. It is not this one:

- the desktop copy's two public functions are consumed by **nobody in the desktop app** — not
  `commands/`, not `application/`. Its only caller in the repository is
  `e2e-tests/tests/e2e_enactment_predicate.rs`, which uses it for multisig updates;
- `UpdateAction::SafeHarbourAddress(_)` is **already** in the `None` arm of
  `multisig_config_update_target` (`:128`), so it answers `Ok(false)` before the authorization
  guard — an ordering that module already tests on purpose (`:496-515`).

Teaching it a predicate nothing calls would be code with no reader. The production answer comes from
the orchestrator, and Phase 2's e2e is what proves it against a chain. Recorded here so review reads
this as a decision rather than an omission.

### 4.7 The enactment predicate

The arm follows Defcon 3's shape (`asm_enactment.rs:139-168`): decode bridge and admin, find the
action in `queued()` by `UpdateAction` equality, read `last_seqno` off `Role::StrataAdministrator`,
delegate to a free function beside `defcon3_enacted`:

```rust
fn safe_harbour_address_enacted(
    last_seqno: u64,
    seq_no: u64,
    still_queued: bool,
    current_descriptor: &[u8],
    proposed_descriptor: &[u8],
) -> bool {
    last_seqno >= seq_no && !still_queued && current_descriptor == proposed_descriptor
}
```

Three things about that signature are deliberate:

- **`&[u8]`, not `SafeHarbourAddress`.** `bridge.safe_harbour().address().as_descriptor().to_bytes()`
  is a chain of inherent methods, so nothing has to be named — which is why `orchestrator-be` needs
  **no new dependency at all** (§9.1). It also makes the truth table writable with byte literals and
  no protocol types.
- **`>=` on the seqno**, for the reason Defcon 3 records at `:256-260`: the action carries a non-zero
  depth, so a later administrator action may jump `last_seqno` past this proposal before it matures,
  and `==` would mark a successfully enacted rotation `Superseded`.
- **The address term is what makes it this proposal's answer**, and it is what makes
  [Constraint 1](./security-council-safe-harbour-address.md#1-a-rotation-with-the-harbour-already-activated-is-accepted-and-changes-nothing)
  hold: a rotation the bridge swallowed leaves the queue with the seqno consumed and the address
  unchanged, so this returns `false` and the proposal resolves as `Superseded`, never `Enacted`.

`action_needs_chain_tip` (`:296-302`) is **not** touched: this post-condition never compares against
the tip, so no other action starts paying that RPC.

### 4.8 Where the compiler insists, and where it does not

Insists — five sites, each a `Record`, a `switch` with no default, or an exhaustive match:
`validators/index.ts:10`, `action-type-from-decoded.ts:19`,
`manual-proposal/model/authorizing-authority.ts:10` (a `switch` whose missing arm makes the function
lack a return, which is how it fails), the `const unhandled: never` at
`use-create-proposal.ts:126-129`, and Rust's matches over `Action` and `UpdateAction`.

Does not — five, three of which a signer sees:

| Site | If missed |
|---|---|
| `create-proposal-form.tsx:440` — the fields ternary ends in `VkUpdateFormFields` | the safe harbour form renders **VK fields** |
| `create-proposal-preview.tsx:180` — same chain, same `else` | the signer reviews a rotation under **"New Verification Key"** |
| `sign-proposal-view.tsx:203-213` — the details switch | the sign view falls back to the raw-hex block |
| `lib/proposal-type-label.ts:3-15` — an `if` chain ending in `'Unknown'` | the dashboard says *Unknown* |
| `api/ipc-schemas.ts:48` — a closed `z.enum`, checked at runtime | one such proposal fails the parse of **every proposal in the same list** |

The last is why commit 2 precedes commit 4 as a rule, not a preference: no compile error crosses the
Rust↔TypeScript border (V3 Phase 1 §3).

And one site with no compiler and no list: `commands/invoke.rs` carries **two** handler registries,
`attach_production` (`:16-83`) and `attach_with_dev_signing` (`:87-163`), chosen by
`dev_mnemonic_signing_ipc_enabled()`, which is **always true in debug builds**. Registering the new
command in the dev list only makes it work in `npm run tauri dev` and fail in release — the mode
nobody exercises while developing.

### 4.9 Copy

The card's title is `'Safe Harbour address update'`. It must not contain the exact substring
`"Signer update"`: three WebDriver specs select the administrator's card by that text and
`ActionTypeCard` renders title and description as two `<p>` inside the button (V3 Phase 3 §6). It
does not.

The entry goes **last** in `ACTION_TYPES_BY_AUTHORITY.strata_admin`, so `getDefaultActionType` —
which returns the first entry — keeps answering `signer_update`.

The already-activated note reuses `SafeHarbourNote` with its own wording, on the form, the preview
and the sign view. Amber, never red: it is a fact about the chain, not an error by the signer.

**`screens/__tests__/safe-harbour-note-gating.test.ts` scans `screens/` sources only.** The new
render sites live under `domain/`, where `defcon-form-fields.tsx` already sits unscanned. This phase
widens that test's reach to cover both rather than adding a third unguarded site — the value of that
test is precisely that it covers *every* site, and it silently stopped doing so.

## 5. Tests

The rule: what can regress and break something real. Every test below is a pure function over
scalars or over a schema — **no mocks, no DOM, nothing that pins a phrasing or a language
guarantee.**

### 5.1 The conversion table — `src-tauri`, domain

The highest-value test of the phase: this is the step that can send funds elsewhere, and it is ours.
One table, one row per rejection, plus the two acceptances:

| Input | Expected |
|---|---|
| valid P2TR address on the active network | accepted; the 32 bytes equal the address's witness program |
| the same address on another network | `WrongNetwork`, naming both |
| a P2WPKH address | `NotP2tr` |
| valid BOSD hex (`04` + 32 bytes) | accepted; round-trips back to the same address |
| BOSD hex with a non-`04` tag | `NotP2trTag` |
| 32 bytes that are not on the curve | `InvalidPoint` |

Plus one property worth its own name: `from_address` and `from_hex` of the same destination produce
the same 32 bytes. That is the claim the two entry paths make together, and neither test alone makes
it.

### 5.2 The codec — `src-tauri`

Round-trip both directions, and a tripwire that the encoded action's `update_tx_type()` is still
`UpdateTxType::SafeHarbourAddressUpdate`. Modelled on
`defcon_1_round_trips_and_encodes_upstreams_defcon_1_tx_type` (`action_codec.rs:346-380`): the
round-trip alone would survive an upstream reordering of the SSZ union, which is exactly the failure
that matters and the one nobody would find until a transaction was rejected on chain.

### 5.3 The signing-message tripwire — `src-tauri`, `--bin`

Run out of the **builder**, not out of a hand-built `Action`, so the test covers the mapping the
device actually signs over. It asserts on `message.lines()` rather than `contains()` over the whole
string — a renderer that joined two lines would pass a `contains` and fail a signer — and pins that
the destination line carries the **descriptor hex**, not an address.

It lives in `commands/action_builder.rs` because the builder does, and `commands/` compiles only
into the binary crate (`main.rs:3`); a test that needs it cannot run from the library's test tree.
That also means it does **not** run under `cargo test -p desktop-app --lib`.

### 5.4 The enactment truth table — `orchestrator-be`

Beside `defcon3_enacted`'s tests, in the same style: bare `assert!` over a free function with byte
literals for the two descriptors. Two rows carry their own names, because they are the two ways this
predicate can be wrong in opposite directions:

- `safe_harbour_not_enacted_when_the_seqno_advanced_but_the_address_did_not` — AC 8, the swallowed
  rotation. This is the defect the whole slice turns on.
- `safe_harbour_not_enacted_when_the_address_matches_but_the_seqno_has_not_reached_it` — AC 7b, the
  rotation somebody else installed.

Plus the queue term, and `>=` accepting a jumped seqno (the counter-case to `==`).

### 5.5 Authorization and depth — `orchestrator-be`

One test over one tx-14 action, both directions: `StrataAdmin` accepted, `SecurityCouncil` refused.
The second half is the segregation invariant at its sharpest — the authority that triggers the sweep
must not choose where it lands — and nothing pins it for this action today.

Depth: tx 14 against tx 10, modelled on `two_actions_of_one_authority_resolve_to_their_own_depths`.
Both are created by the administrator, which makes them exactly the pair an authority-shaped mapping
cannot separate.

### 5.6 TypeScript — `model/__tests__/safe-harbour-address-update.test.ts`

House style: top-level `assert` from `node:assert/strict`, no runner, no mocks, relative imports
with explicit `.ts` extensions, discovered by filesystem
(`scripts/run-unit-tests.mjs`), one claim per assertion message tagged with its AC.

1. **AC 1** — `getActionTypeOptions('strata_admin')` contains the new entry **last**, and
   `getDefaultActionType('strata_admin')` is still `'signer_update'`.
2. **AC 1a** — no other authority is offered it, and the schema refuses it for `security_council`,
   `sequencer_manager` and `alpen_admin`. The council case is the one that matters.
3. **AC 3a/3b** — the validator over the five address forms.
4. **AC 3c** — the address already installed is refused; a different one is not; and the rule is off
   when `currentSafeHarbourAddress` is null, which is safe only because §4.5 blocks submission in
   that state.

### 5.7 Not tested, deliberately

The form wiring. There is no DOM runner in this repository, and a test that reads a component's
source with `readFileSync` pins a phrasing rather than a behaviour. The mitigation is structural —
the decisions live in pure, tested functions and the wiring is a handful of lines — and the honest
substitute is the manual walk in §10.

No ASM-backed integration test inside `orchestrator-be`: it would be the flakiest test in the
repository and would re-prove what Phase 2's e2e proves against a real chain.

## 6. Test-run discipline

Per commit, only what that commit touched:

```bash
cargo test -p desktop-app --lib domain::action           # commit 3
cargo test -p desktop-app --lib action_codec             # commit 4
cargo test -p desktop-app --bin desktop-app action_bui   # commit 4 — see §5.3
cargo test -p orchestrator-be --lib asm_enactment        # commit 7
cd desktop-app && npm run test:unit                      # commits 2, 6
```

The full `AGENTS.md` checklist runs **once**, before committing the series and before pushing — not
per commit. `npm run test:unit` is absent from that checklist and present in CI; it runs too.

## 7. Migration — eight commits, each atomic

None repairs the one before it. Every one compiles, lints and leaves the suite green.

> **Commits 6 and 7 ship together.** Between them the menu offers an action whose enactment answers
> `BadRequest`. They belong to one pull request, and this phase ships as one (§3).

| # | | Contents |
|---|---|---|
| 0 | 📄 | This document. The phase is designed before it is built, and reviewed before it is implemented. |
| 1 | 🟢 | `bitcoin-bosd` (git, tag `v0.11.0`) and `strata-asm-proto-bridge-v1-types` (rev `b84eb28…`) in `[workspace.dependencies]` and in `desktop-app/src-tauri`. No behaviour change. |
| 2 | 🟢 | TypeScript **read-side** vocabulary: the transport `ActionType`, the `z.enum`, the `decodedActionSchema` member, `DecodedAction`, `ACTION_TYPE_BY_KIND`, the authorizing-authority `switch`, the label. Inert — nothing emits the value yet. The **form's** `ActionType` is a different union and stays in commit 6: widening it makes `ACTION_TYPE_OPTIONS` and `actionValidators` fail to compile, and both belong with the card and the validator. Same split V3 made between its Phase 1 and Phase 3. |
| 3 | 🔴🟢 | `SafeHarbourDescriptor` and its conversion table (§5.1). |
| 4 | 🟢 | The codec both ways, `build_safe_harbour_address_update_hex` registered in **both** `invoke.rs` lists, `action_type_from_hex`, the `DecodedAction` variant, and the tests of §5.2 and §5.3. |
| 5 | 🟢 | The bridge's current destination: `fetch_safe_harbour` → DTO → Zod schema → `api/asm-state.ts`. |
| 6 | 🟢 | Menu entry, validator with the no-op rule, `use-safe-harbour-action-hex.ts`, the fields component, the preview arm, the sign-view arm, the note on all three surfaces, the widened gating test (§4.9), and §5.6. |
| 7 | 🟢 | The enactment arm, `safe_harbour_address_enacted`, its truth table, §5.5, and the stale comment at `asm_enactment.rs:113-116`. |

## 8. Blast radius, and debt recorded rather than fixed

1. **The form wiring is untested** (§5.7) — the same debt V1, V2 and V3 accepted; the manual walk
   covers it.
2. **`ActionValidatorContext` grows a third field.** `currentSafeHarbourAddress` joins
   `currentMultisigSigners` and `currentMultisigThreshold`, and the three could in principle diverge.
   They cannot in practice — one call site, one optional chain each. Collapsing them into one
   snapshot object touches every validator and is not worth this phase's diff. Recorded in V3's
   Phase 3 §10.4 already.
3. **Every existing `draft` fixture gains a field.** The test drafts enumerate the whole schema
   (e.g. `council-signer-update-retarget.test.ts:28-41`), so the new form field has to be added to
   each. Mechanical, and invisible in any file list.
4. **A resolved signing message is not a submission gate here** (§4.4), unlike Defcon. Consistent
   with `vk_update` and `signer_update`; if that gap is ever worth closing it should close for all
   three at once, not for this action alone.
5. **The desktop's `is_proposal_enacted_on_asm` stays wrong for this action** (§4.6) — it answers
   `Ok(false)`. Nothing calls it in production. If a desktop surface ever needs an enactment answer,
   that is the moment to fix it, and this note is the pointer.
6. **Two rotations to the same destination** satisfy each other's post-conditions. The same ambiguity
   every config-carrying action has; the seqno term bounds it. Already in the contract's edge cases.

## 9. Where this phase departs from the build plan, and why

**9.1 `orchestrator-be` gets no new dependency.** The build plan implies both crates need
`bitcoin-bosd` and `strata-asm-proto-bridge-v1-types`. The backend only *reads*, and the read is a
chain of inherent methods whose result is `Vec<u8>`; with the predicate taking `&[u8]`, no protocol
type is ever named there. The two dependencies stay confined to `desktop-app/src-tauri`, and within
it to `action_codec.rs` — so the module that was already the only importer of protocol crates stays
that way.

**9.2 The desktop's enactment copy is not touched** (§4.6), against the build plan's "both move
together". The plan's reason does not apply to an action that copy never answers for and no desktop
caller asks about.

**9.3 The gating test is widened** (§4.9). Not in the build plan at all: it is a hole this phase
would otherwise widen, so this phase closes it.

**9.4 Eight commits, not one.** The build plan asks for one atomic commit per phase, and its reason
is about what reaches `develop`. One pull request satisfies that reason exactly as well while making
a large change reviewable in steps. §7 carries the constraint that actually matters — commits 6 and
7 are inseparable — rather than the proxy for it.

## 10. Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop-app && npm run format:check && npm run lint && npm run build && npm run test:unit
```

**The manual walk**, with the local stack up (`./scripts/local-stack.sh --clean` if any state
predates the ASM pin bump):

1. A Strata Administrator sees the new entry and reaches it; a Security Council signer sees neither
   it nor a route to it.
2. The form shows the bridge's current destination as an address **and** as its descriptor hex.
3. A P2WPKH address, an address of another network, and the address already installed are refused —
   with three different messages.
4. With a valid address entered, the form shows the canonical message, and it matches the signer's
   screen byte for byte, including the `New Safe Harbour Address:` line.
5. Quorum and broadcast → `Approved`, then `Awaiting enactment` with a countdown to `reveal + 30`,
   and the bridge's destination unchanged.
6. Mine 30 blocks → `Enacted`, and `strata_asm_getSafeHarbour` shows the new address with
   `activated` unchanged (AC 7a).
7. With the harbour already activated: the note appears on the form, the preview and the sign view;
   the action can still be signed; and it resolves as `Superseded`, never `Enacted` (AC 8, AC 11).
8. An administrator signer update created in the same session still behaves as before — the
   regression that proves the menu and validator changes did not leak.
9. The manual bundle exports and imports the action without reporting it as unknown (AC 5).

**Close-out** — three edits that do not update themselves:
`security-council-safe-harbour-address-implementation.md`'s `Status:` header and its `| 1 ✅ |` row,
and the contract's `Status:` header.
