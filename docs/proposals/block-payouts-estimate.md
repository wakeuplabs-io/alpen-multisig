# Block Payouts — Implementation Estimate

> **Status:** Internal draft — not shared with the client.
> **Scope:** End-to-end implementation of the Payout Administrator flow ([PRD §6](../0-prd/05-prd-payout-admin-block-payouts-update.md)).
> **Team assumption:** 2 full-stack Software Engineers with prior experience on this project (no ramp-up).

## 1. Answer to Alpen

**Partially yes.** The spec plus the [`claim_payout.rs` unit test](https://github.com/alpenlabs/strata-bridge/blob/70cc4e82d13c15285e4ade371499f0a6f31cd239/crates/connectors/src/claim_payout.rs#L296-L314)
are enough to estimate the **signing mechanism**, and that is a real unblock — it closes the gap we previously recorded as
"knowledge of the bridge script spending conditions". It is **not** enough to commit to a delivery date, because three
inputs only Alpen can provide are still missing (§4).

The test tells us what a `block_payout` actually is: the **`AdminBurn` spend path** of the `ClaimPayoutConnector` — a
**taproot script-path** spend (leaf 0) over `threshold_multisig_script(admin_pubkeys, admin_threshold)`, witnessed with
Schnorr signatures indexed by key position, deduplicated, sorted by descending index and truncated to the threshold.

That single fact reframes the work: **signing a `block_payout` means signing a taproot script-path spend on a hardware
wallet**, which nothing in the current system does today.

## 2. Estimate at a glance

| Phase | Duration |
|---|---|
| 0 — Technical spike, go/no-go (three independent unknowns, parallel tracks) | 2.5–3 weeks |
| 1 — First real payout, end-to-end: signed to quorum and broadcast | 3 weeks |
| 2 — Product design and UX (partially overlapped) | 1 week |
| 3 — Full Payout Administrator flow | 4.5–5 weeks |
| 4 — Hardening, physical-device validation, release | 2.5 weeks |
| **Total** | **~12 weeks nominal — realistic band 11–15 weeks** |

The plan is deliberately iterative: Phase 0 buys information on the most expensive risk **before** the bulk of the
calendar is committed, and Phase 1 proves the integration end-to-end on one real transaction before the flow is widened.
Phase 0 is a genuine go/no-go, not a formality — a "no-go" outcome renegotiates scope rather than the schedule.

**How this number was calibrated.** Governance shipped in **11.1 weeks** (2026-04-07 → v0.2.4 on 2026-06-24) with roughly
3.5 FTE — about **38–40 person-weeks**. This estimate is ~24 person-weeks, near 60% of that. Substantial reuse pulls the
number down (release pipeline, Admin Wallet and its coin selection, session auth, UI kit, dockerised regtest stack), but
this scope carries **more** technical risk than governance did: governance had a ready-made upstream crate and signed
*messages*, whereas this work depends on an upstream repository with no frozen revision, a hardware-wallet blocker with no
known workaround, and two data contracts that do not yet exist.

## 3. Disclaimers

1. **Taproot script-path signing on hardware wallets is the primary blocker, with no workaround on our side.** The tap
   tree **differs per input** (the unstaking image, and possibly the N/N key, vary per claim). Ledger's wallet policies
   are registered **per wallet, not per input**, so expressing M distinct trees may be structurally impossible with the
   stock Bitcoin app. Trezor today signs **key-path only**. Nothing in the current signing layer is reusable here. The
   likely spike outcome is a custom signing path or a partial no-go — not "register a policy and move on".
2. **Per-outpoint metadata has no known provenance.** Rebuilding the script and control block for each input requires the
   N/N key and unstaking image **per claim**. There is no published contract for where these come from. This blocks even
   the first end-to-end transaction.
3. **`strata-bridge` is a new dependency that upstream itself avoided.** The project does not depend on it today. Alpen's
   own ASM runner documents being written "to avoid a painful dependency on `strata-bridge`", and the ASM security audit
   configuration carries advisories whose upgrade is "blocked until the strata-bridge dependency updates its
   rustls/webpki stack". Concrete risk: dependency-graph conflicts against our pinned ASM revision.
4. **Upstream has moved the floor before.** The last ASM bump replaced the signing digest and **invalidated every
   signature collected up to that point**. Without a `strata-bridge` revision frozen by Alpen for the duration, the upper
   bound of this band is not enforceable.
5. **The Payout Administrator signer set lives on Bitcoin L1, not in ASM state.** Today no source exists for it: current
   membership is derived from ASM, and while the authority type already exists in the backend, it has no backing data.
   This is design, backend and operational work that has no precedent in the delivered system.

## 4. What we need from Alpen

1. A **frozen revision or tag of `strata-bridge`** for the duration of the project.
2. The **false claim report contract** — format, issuer, and the per-outpoint metadata needed to rebuild each input.
3. **PRD §6.4.1 (cryptographic validation of false claim proofs) descoped, in writing.** We cannot commit to a phase
   called "full flow" while its central validation step is disclaimed. If the proof must be verified in-app, it is not
   estimable until the contract exists; as a placeholder, assume **+4–8 weeks**.
4. Confirmation of the **spend path and sighash type** used by the connector.
5. A **reproducible test environment** for the bridge, or agreement that we build the fixtures ourselves (already priced
   into Phase 0).

## 5. Scope notes

- **The existing UI mock does not reduce the estimate.** It contains no IPC calls, no real signature validation, no raw
  import, no Admin Wallet fee sourcing, and a fabricated raw transaction. It is discarded; UX is estimated from scratch.
- **Sighash coupling freezes the transaction.** Under BIP-341 the signature commits to every input and output, so fee
  inputs, change and fee rate are **locked before the first signature is collected**. Removing individual inputs (§6.4.4.1)
  is therefore a pre-signing operation only; any later edit invalidates every signature already gathered. This is a
  product state that the flow must model explicitly.
- **Signature exchange needs a versioned envelope.** Validating a pasted signature means recomputing the sighash, which
  requires the leaf script, the merkle path and all prevouts with amounts — i.e. a PSBT-style format, not a bare
  transaction. This also settles the open PRD question about what "raw copy" means for import/export (§6.2.4).
- **The backend gains a chain-watching responsibility.** Expiry (4 days, or an input spent elsewhere) and confirmation
  status require monitoring Bitcoin and a scheduled job. Some groundwork exists, but outpoint monitoring and scheduling
  do not.
- **Open PRD ambiguity:** expired payouts are to be deleted, while expired admin proposals are retained. Worth confirming
  the asymmetry is intentional.

## 6. Out of scope

Security audit · auto-update · changes to `strata-bridge` upstream · hardware devices beyond those Phase 0 confirms ·
generation of false claim reports (the application consumes them, it does not produce them).
