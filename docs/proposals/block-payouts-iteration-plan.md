# Block Payouts — Iteration Plan (working doc for team discussion)

> **Status:** Internal working draft — for team discussion. Not shared with the client, not SSOT.
> **Companion to:** [`block-payouts-estimate.md`](./block-payouts-estimate.md) (the numbers) — this doc is the *reasoning*
> behind each iteration.
> **How to read this:** every iteration answers four questions — **what** we deliver, **why now**, **how** we'd approach
> it, and **what it adds vs. what it deliberately does not add yet** (scope-note style). Numbers are ranges, meant to be
> argued, not defended.

## Guiding principles

- **Self-serve first, escalate only true blockers.** We don't start by asking Alpen for things. Inside the spike we
  actively try to answer each open question ourselves — stand up their environment, read their code, build fixtures — and
  only escalate what we genuinely cannot resolve. Each escalation becomes a **named spike output** ("we need X from
  Alpen"), not a vague dependency.
- **Frozen bridge = current revision.** We assume the pinned `strata-bridge` is the current rev
  (`70cc4e82d13c15285e4ade371499f0a6f31cd239`). That assumption is itself validated in the spike (does it build into our
  workspace, does it produce a connector output we can spend).
- **Buy information before committing calendar.** The order of iterations front-loads the expensive unknowns. We spend
  ~3 weeks proving feasibility before we spend ~10 weeks building the flow.
- **One real transaction before breadth.** We prove the whole chain end-to-end on a single payout before we widen into
  the full PRD §6 surface.

---

## Iteration 0 — Spike / Feasibility (2.5–3 weeks, both engineers, parallel tracks)

**What we deliver:** a go/no-go report with an answer (or a named blocker) for each of the four unknowns below, plus a
throwaway proof that a `block_payout` can be signed on a real device. Not production code — evidence.

**Why now:** three of the four unknowns can each independently sink or reshape the whole project (device signing, data
contract, environment). Discovering any of them in week 8 is the failure mode the whole plan exists to avoid.

The spike is **four parallel investigation tracks**. Each has a self-serve path and an explicit escalation trigger.

### Track A — Signing: taproot script-path on hardware wallet *(the #1 risk)*
- **Question:** can Ledger and/or Trezor produce a valid Schnorr signature for the `AdminBurn` **script-path** leaf of
  the `ClaimPayoutConnector`, given the tap tree differs per input?
- **How we approach it:** reproduce the exact tap tree in a throwaway harness; attempt the spend on the emulators first
  (Speculos is already automated in our repo, so this is cheap), then confirm on a physical device. Test the per-input
  problem specifically: Ledger wallet policies are per-wallet, so we need to know whether M distinct trees are
  expressible at all.
- **Escalation trigger → spike output:** if neither device supports it with the stock app, the output is a decision, not
  a request: "supported device set narrows to X" or "a custom signing path is required (+cost)" or "no-go, needs Alpen /
  firmware change".
- **Exit criteria:** a signed, verifiable script-path spend on at least one device, **or** a documented impossibility
  with the narrowing it implies.

### Track B — Connector semantics: spend path + sighash *(self-serve, low escalation risk)*
- **Question:** which sighash type does the connector use for `AdminBurn`, what exactly is committed, and what does the
  witness stack look like (order, placeholders)?
- **How we approach it:** read `claim_payout.rs` and `test_utils.rs` directly (both are in the repo we pin), and confirm
  against the cited unit test. This is investigation, not negotiation — we expect to answer it ourselves.
- **Escalation trigger:** only if the code is ambiguous or contradicts the test. Low probability.
- **Exit criteria:** a written, code-referenced statement of the sighash type and witness layout, feeding Iteration 1.

### Track C — Environment: stand up `strata-bridge` and produce a real connector output *(self-serve first)*
- **Question:** can we generate, in regtest, a real `ClaimPayoutConnector` output that we then spend via `AdminBurn`?
- **How we approach it:** `strata-bridge` ships `compose.yml`, a `.justfile`, and `functional-tests/`. First attempt is
  fully self-serve: build the connectors crate into (or alongside) our workspace, resolve the dependency-graph conflict
  against our pinned `asm` (bitcoin/secp256k1/musig2 versions), and use their test utilities to mint a connector output.
- **Escalation trigger → spike output:** if the env can't be reproduced locally or the crate can't coexist with our
  graph, we ask Alpen a *specific* question ("how do you produce a claim connector output in regtest / can you share a
  fixture"), not "please give us an environment".
- **Exit criteria:** a regtest UTXO on a `ClaimPayoutConnector`, reproducible by a script — **or** a named environment
  blocker.

### Track D — Data contract: false claim report + per-outpoint metadata + proof validation
- **Question:** do we actually have (or can we derive) the false claim report format and the per-outpoint metadata
  (N/N key, unstaking image) needed to rebuild each input's script and control block? And what does PRD §6.4.1
  "verify the proof" actually require?
- **How we approach it:** read the bridge's contest/disprove code (`claim_contest.rs`, `contest_counterproof.rs`) to see
  whether the report and metadata are already derivable from on-chain data or an existing type. Assume nothing from the
  mock (its `proof` field is a placeholder).
- **Escalation trigger → spike output:** if the report contract or the metadata provenance isn't derivable, request it
  from Alpen as a **blocker**. Separately, if proof validation implies verifying a ZK proof, flag it as **out of scope
  pending a signed decision** — it's not estimable until the contract exists.
- **Exit criteria:** either "we can derive the inputs ourselves, here's how" or a precise list of what we need from Alpen.

**What the spike explicitly does NOT do:** no production UI, no backend endpoints, no persistence, no multi-input
optimization. Throwaway code is fine and expected.

---

## Iteration 1 — First real payout, end-to-end (3 weeks, both engineers)

**What we deliver:** one real `block_payout`, created from a real connector input + a fee input from the Admin Wallet,
signed to quorum across signers, and broadcast on regtest. The thinnest possible full-chain slice.

**Why now:** integration risk lives in the seams, not the parts. Proving propose → sign (device) → aggregate → broadcast
on *one* transaction flushes out the hard couplings (control block construction, fee-input mixing, witness assembly)
before we invest in breadth.

**How we approach it:** reuse aggressively — Admin Wallet coin selection and change, session auth, the orchestrator's
signature-collection shape, the regtest stack. Add only what the payout needs: connector input construction, per-leaf
sighash, script-path witness assembly, control block.

**What it ADDS:** the real signing path from Iteration 0 promoted into non-throwaway code; a minimal
propose/sign/broadcast wired through the backend for a single tx.

**What it does NOT add yet:** no false-claim-report ingestion UI (inputs can be fed from a fixture/CLI), no multi-input
selection, no expiry, no conflict detection, no import/export, no standardness enforcement, no polished UX. Hardcoded
quorum is acceptable here.

**Expected outcome:** a demo — "here is a payout we built, signed on a device, and confirmed on regtest." This is the
real de-risking milestone; if Iteration 0 said "go", this proves it.

**Dependencies/risks:** consumes the answers from Tracks B/C/D. If Track D escalated, the report ingestion stays fixture-
based until Alpen responds — the skeleton still proceeds on the signing/broadcast axis.

---

## Iteration 2 — Product design & UX (1 week, partially overlapped)

**What we deliver:** the UX for the full Payout Administrator flow, designed against the real data shapes learned in
Iterations 0–1 (not against the mock's assumptions).

**Why now:** by this point the transaction's real constraints are known — most importantly the **sighash freeze** (fee
inputs, change and fee rate lock before the first signature). The UX must model states the mock never did: frozen tx,
signature-error states, import/export.

**How we approach it:** the discarded mock is thrown away as *code*, but it remains a usable *visual reference* for the
list/card/modal structure. We design from scratch what's genuinely new (report ingestion, freeze state, error copy) and
reuse the existing app's component language for the rest.

**What it ADDS:** screens/flows for the full §6 surface.
**What it does NOT add:** implementation — this is design feeding Iteration 3.

**Note for the team:** Fable's review argued this could be 0.5 week using the mock as spec. We're holding it at 1 week
per the from-scratch instruction; worth deciding together whether that's the right call.

---

## Iteration 3 — Full Payout Administrator flow (4.5–5 weeks)

**What we deliver:** PRD §6 implemented — the parts that were out of scope in the skeleton.

**Why now:** breadth is only cheap once the spine (Iteration 1) holds.

**How we approach it:** incrementally, each sub-area shippable on its own. The heavy, previously-omitted items:

- **False claim report ingestion** + input derivation (real, not fixture) — pending Track D's contract.
- **Multi-input selection** with the **standardness / vsize budget**, estimating the witness (including AdminBurn's empty
  placeholders) so the fee rate and size limit are honored *before* signing.
- **Sighash-freeze UX**: input editing is pre-signing only; any edit invalidates collected signatures.
- **Versioned PSBT-style envelope** for import/export and pasted-signature validation (leaf script + merkle path +
  prevouts with amounts are all required to recompute the sighash). This also answers the open PRD "raw copy" question.
- **Backend chain-watching**: expiry (4 days or input spent elsewhere) + Confirmed/Unconfirmed status + delete-on-expiry.
- **Conflict detection** across pendings, rebroadcast, exact error strings, and a **fee-UTXO lock** across pendings so
  one proposer can't kill sibling transactions by spending the shared fee input.
- **L1 signer-set source** for the Payout Admin authority (no ASM backing exists today).

**What it does NOT add:** cryptographic proof validation (§6.4.1) **if** it was descoped in the spike; if it stays in,
it's a separate, currently-unestimable line (+4–8 weeks placeholder).

**Expected outcome:** feature-complete against §6 minus any signed descope.

---

## Iteration 4 — Hardening, device matrix, release (2.5 weeks)

**What we deliver:** validated on physical devices, e2e-tested with new connector fixtures, packaged into a signed
release.

**Why now:** the signing path is brand-new in two vendors; the project's own history shows a tail of post-release
`fix(hw-wallet)` commits even for a *mature* flow. We budget for that tail rather than pretend it away.

**How we approach it:** reuse the existing Tier-1 reproducible release pipeline; add payout-specific e2e fixtures and a
physical-device pass on the confirmed device set from Iteration 0.

**What it ADDS:** production confidence and a shippable artifact.
**What it does NOT add:** new features — anything discovered here that isn't a defect goes to a backlog, not into this
iteration.

---

## Deliberately deferred (parking lot)

- Cryptographic false-claim-proof validation, unless/until Alpen provides the contract.
- Hardware devices beyond those Iteration 0 confirms.
- Generation of false claim reports (we consume, we don't produce).
- Auto-update, security audit — out of scope for this work entirely.

## Open questions to settle as a team

1. UX at 1 week from scratch vs. 0.5 week using the mock as visual spec — which do we commit to?
2. If Track A returns "custom signing path required", do we price it into the band now or treat it as a hard renegotiation
   gate?
3. Do we want Iteration 1's report input to be CLI/fixture-fed (faster) or a minimal real UI (closer to demo-able)?
4. How do we want to source and rotate the L1 signer set operationally — static signed config, or something read from
   the bridge script? This is a design decision with operational weight.
