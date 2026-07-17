# Block Payouts — Estimation Session Prep

> **Status:** Internal working draft — facilitation material for a team estimation session. Not shared with the client,
> not SSOT.
> **Audience:** the engineers who built the ASM governance flow — already in context on the app, the HW signing layer,
> the orchestrator, and the regtest stack. This doc is deliberately terse and technical.
> **Companion:** [`block-payouts-estimate.md`](./block-payouts-estimate.md) holds the numbers and disclaimers. This doc is
> the **per-iteration checklist of things we must not miss when we estimate**.

## Meeting flow (suggested)

1. **Context (5 min)** — what a `block_payout` actually is and why it's *not* an ASM governance action (below).
2. **What we're estimating (2 min)** — end-to-end Payout Administrator flow, PRD §6. UX from scratch. Mock discarded.
3. **Iterate the iterations (bulk)** — for each, walk the *critical considerations* and *concrete probes*, then estimate.
4. **Cross-cutting risks (10 min)** — the things that recur across every iteration.
5. **Land a band** — reconcile per-iteration estimates against the top-down band (11–15 weeks) and name the swing factors.

## Context in one paragraph (for the room)

Unlike everything we built for governance, a `block_payout` is **not** an admin-subprotocol action: no SSZ envelope, no
OP_RETURN, no ASM. It's a **pure Bitcoin spend** of the `AdminBurn` **taproot script-path** leaf of the
`ClaimPayoutConnector` (`threshold_multisig_script`, CHECKSIGADD), witnessed with Schnorr signatures indexed by key
position. The signer set lives in the **bridge script on L1**, not in ASM state. So almost none of our governance signing
path carries over — that's the crux of the estimate.

---

## Iteration 0 — Spike / Feasibility

**Frame for the room:** this is not "research", it's *evidence-gathering that produces a go/no-go*. We assume the pinned
`strata-bridge` is the current rev and we try to break that assumption ourselves before asking Alpen anything.

**Critical considerations — do not estimate the spike without accounting for:**

- **Run an actual end-to-end, not a compile.** The bar for "environment works" is not `cargo build` — it's *we produced a
  connector UTXO in regtest and spent it*. Budget for running `strata-bridge`'s own `functional-tests` / `compose.yml`
  first to confirm the env is real, then for building our own minimal fixture. This is where the days hide.
- **Real hardware verification, not emulator-only.** Speculos/emulator tells us if the firmware *can* sign script-path;
  it does **not** tell us what the signer *sees on screen*. Governance already burned us on "signs, but the on-device
  display doesn't match" (the `fix(hw-wallet)` tail). Budget a physical-device pass that checks the **verify screen**,
  not just a green signature.
- **Miniscript / wallet-policy expressibility is the make-or-break sub-question.** Concretely: is the `AdminBurn` leaf
  expressible as `multi_a` / `sortedmulti_a` in a descriptor, and does Ledger's wallet-policy language accept it? And
  since the tap tree differs per input (unstaking image, possibly N/N), can M distinct policies even be registered, or
  does the app cap/normalize them? If the answer is "not expressible", the whole device story changes — this is the
  single probe most likely to flip the band.
- **Dependency-graph collision is a real timebox, not a footnote.** We pin `asm` at a rev on nightly; the connectors
  crate pulls its own `bitcoin` / `secp256k1` / `musig2` / bitcoind-client stack. Actually attempt the `cargo add` in the
  spike and timebox the resolution — this can silently eat 2–3 days and it's better to discover it in week 1.
- **Confirm the sighash *and* that we can recompute it.** Reuse the `verify_threshold` pattern: the exit criterion is not
  "the device signed" but "we independently recomputed the sighash and the signature verifies against it." POC-5's trap
  was self-verifying against the *wrong* sighash and calling it green.
- **Data contract: derive-or-escalate.** Read `claim_contest.rs` / `contest_counterproof.rs` to see if the false claim
  report and the per-outpoint metadata are derivable from chain/types. Assume nothing from the mock. If proof validation
  (§6.4.1) turns out to be a ZK/sp1 verification, flag it as *unestimable until contract* — don't fold a guess into the
  number.

**Spike outputs (each is either an answer or a named blocker):** device signing verdict + device set · sighash & witness
layout · reproducible connector fixture · report/metadata contract (ours or Alpen's).

---

## Iteration 1 — First real payout, end-to-end

**Frame:** the thinnest full-chain slice — one real payout, real fee input, signed to quorum, broadcast on regtest.

**Critical considerations when estimating:**

- **Mixed input types in one transaction.** A script-path connector input **plus** a key-path Admin Wallet fee input,
  potentially on different derivation paths/devices. BIP-341 commits to **all** prevouts and amounts, so every input's
  UTXO data must be present at signing time. Estimate the plumbing to carry that, not just the happy-path sign.
- **Control block + merkle path assembly is fiddly and silently wrong.** Validate against **`bitcoind` mempool
  acceptance** (`testmempoolaccept`), not against our own verifier — again, the POC-5 lesson. "Self-verify passes" is not
  acceptance.
- **Broadcast ownership.** Per ADR-006 the orchestrator does **not** broadcast — desktop does. Reuse that boundary;
  don't re-plumb it. The orchestrator's job here is signature collection, same shape as governance.
- **Report input can be fixture/CLI-fed** at this stage — decide as a team whether we want a minimal real ingestion UI
  for demo value or a CLI feed for speed. Affects the estimate.

**What we're *proving*, not building:** propose → device-sign → aggregate → broadcast on one tx. No breadth yet.

---

## Iteration 2 — Product design & UX

**Critical considerations when estimating:**

- **The sighash freeze is the UX-defining constraint.** Fee inputs, change and fee rate lock **before the first
  signature**; editing inputs after any signature invalidates all collected signatures. The design has to make "this tx
  is now frozen" legible, and model the pre-sign vs. post-sign edit boundary. This is the state the mock never had.
- **Design against real shapes, not the mock's fictions.** The mock fabricates `rawTx`, hardcodes quorum, and validates
  signatures by string length. Those are not requirements — don't let them anchor the design.
- **New states the mock is missing:** signature-error copy (exact PRD strings), import/export of a signed tx, conflict
  indicators driven by real shared-outpoint data.
- **Reuse decision to make in the room:** the mock is discarded as code but is a valid *visual* reference for
  list/card/modal structure. Fable argued 0.5 week on that basis; we're holding 1 week from-scratch. Decide.

---

## Iteration 3 — Full Payout Administrator flow

**Critical considerations when estimating — this is where the omitted work lives:**

- **vsize / witness estimation with empty placeholders.** The AdminBurn witness carries empty placeholders for
  non-selected keys; fee rate and standardness must be computed against the *final witness size* before signing. Wrong
  estimate → tx rejected or overpaid. This is arithmetic that must be tested, not eyeballed.
- **Pasted-signature validation needs full context.** To validate a pasted Schnorr sig we must recompute the sighash,
  which requires the leaf script + merkle path + all prevouts with amounts. A bare serialized tx can't carry that → we
  need a **versioned PSBT-style envelope**. This also settles the open PRD "raw copy" ambiguity for import/export.
- **Auto-broadcast on quorum is a security edge, not a UX nicety.** PRD §6.2.8.2: the signature that reaches quorum
  triggers an automatic broadcast. An invalid pasted signature that slips validation = rejected tx or burned fee.
  Validation has to be airtight *before* that path fires.
- **Backend gains a chain-watcher.** Expiry (4 days **or** input spent elsewhere) + Confirmed/Unconfirmed status +
  delete-on-expiry. The orchestrator doesn't watch outpoints today — but electrs is already in the stack, so reuse it;
  estimate the scheduler/job, not a new indexer.
- **Fee-UTXO lock across pendings is an economic footgun.** If a proposer's fee input is shared across sibling pendings
  and gets spent, all of them die by the expiry rule. Estimate a cross-pending UTXO lock.
- **Conflict detection** across pendings (shared-outpoint index, live), **rebroadcast**, **exact error strings**.
- **L1 signer-set source.** The authority enum has `PayoutAdmin` but no backing data; membership can't come from ASM.
  This is design + backend + an operational decision (static signed config vs. read from the bridge script).
- **Proof validation (§6.4.1)** stays out unless the spike found a contract; if in, it's a separate line (+4–8w placeholder).

---

## Iteration 4 — Hardening, device matrix, release

**Critical considerations when estimating:**

- **Budget for the HW fix tail explicitly.** The last governance release shipped with ~5 follow-up `fix(hw-wallet)`
  commits for a *mature* flow. This flow debuts a brand-new signing path in two vendors — the tail will be longer, not
  shorter. Estimate it in, don't hope it away.
- **Physical device matrix** on the confirmed device set from Iteration 0 — real devices, verify screens, both vendors.
- **Release pipeline reuse.** Tier-1 reproducible + signing already exists; add payout e2e fixtures. But: **Windows is
  still partial** in the delivery plan — decide whether payout must ship on Windows, because that's a separate,
  upstream-blocked cost that isn't in this band.

---

## Cross-cutting risks (recur in every iteration — raise once, keep visible)

- **Upstream drift.** `strata-bridge` is an active repo with no frozen contract. The ASM bump precedent (Sighash trait
  removed → BIP-137, all collected signatures invalidated) is the base rate. Pin by rev; treat any connector/witness
  change as rework outside our control.
- **`strata-bridge` is a dependency upstream itself avoided** ("written to avoid a painful dependency on strata-bridge";
  audit advisories blocked on its rustls/webpki stack). The dep-graph work in Track C is the concrete manifestation.
- **Signer safety.** Auto-broadcast + device verification + no ASM cross-check means the app is the last line of defense
  before a real Bitcoin spend. Validation and on-device verify-screen fidelity are correctness, not polish.

## Swing factors to name before locking a band

1. Track A verdict: stock-app script-path works ✓, or "custom signing path / device narrowing" (biggest swing).
2. Miniscript/wallet-policy expressibility (drives Track A).
3. Proof validation in-scope or descoped (±4–8 weeks).
4. Windows in-scope for payout or not.
5. UX 1 week from-scratch vs. 0.5 week mock-as-spec.
