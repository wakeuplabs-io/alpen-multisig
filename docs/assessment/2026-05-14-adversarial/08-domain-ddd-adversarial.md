# Domain / DDD — Adversarial Assessment (re-audit)

**Date:** 2026-05-14  
**Lens:** DDD (bounded contexts, aggregates, consistency boundaries, ubiquitous language)  
**Method:** Read-only; contrasts code and ADRs with classical DDD expectations and product language in repo rules.

---

## Scope & threat model

**What we are trying to break:**

- **Bounded context leakage:** “Proposal” in orchestrator means something different from onchain `MultisigAction` lifecycle — confusion causes wrong invariants in UI and API.
- **Aggregate integrity:** Invariant enforcement scatter (handler vs application vs repo) lets invalid states exist transiently or durably.
- **Ubiquitous language drift:** `BroadcastStatus` vs `ProposalStatus` vs protocol “enacted” language — mismatches confuse requirements tracing (SPS/PRD vs code).
- **False DDD expectations:** Stakeholders expect event sourcing or rich aggregates because “domain/” exists; actual style is anemic models + transaction scripts (not wrong — but **mis-communicated risk**).

**In scope:** `orchestrator-be/src/domain/*`, `application/proposals.rs`, ADR-002 (explicit non-DDD posture), ADR-005 layering, workspace rules citing coordination-only backend.

---

## Top findings (ranked by severity)

### CRITICAL: DD1 — `Proposal` is a data bag; invariants are enforced in application transaction scripts and repository operations, not inside a closed aggregate API

**Risk:** Future contributors add fields or transitions without a single **consistency boundary** — regressions slip because there is no `Proposal::approve(sig)` that refuses illegal moves by construction.

**Evidence:** `domain/proposal.rs` — types + `compute_action_id`; approval/quorum/broadcast rules live in `application/proposals.rs` (`create_update_action`, `approve_action`, `require_approved`, `broadcast_commit_then_reveal`). Repository methods (`add_signature`, `claim_broadcast`) perform mechanical updates.

**Failure scenario:** New status `Canceled` partially implemented in one adapter only; Postgres and memory diverge because rules sit in duplicated SQL vs Rust branches.

**Smallest fix:** Centralize transition table (`ProposalStateMachine::apply(event)`) with exhaustive match; repos store outcomes only.

**Largest bet:** Full aggregate root with version optimistic concurrency per `action_id`.

---

### HIGH: DD2 — Bounded context boundary between **offchain coordination** and **onchain ASM/Bitcoin** is implicit — carried by strings (`action_hex`) and SSZ decode at broadcast time

**Risk:** Domain model at rest is **not** the protocol aggregate; invalid SSZ can enter `Pending` and survive until broadcast — “late failure” surprises product flows.

**Evidence:** `create_update_action` stores `action_hex: String` without decoding to `MultisigAction` at creation (`application/proposals.rs`); SSZ decode appears later in broadcast path (`application/proposals.rs` ~367–371 in `do_broadcast`).

**Failure scenario:** Signers collect threshold on malformed payload; user expends political capital before learning action is unbroadcastable.

**Smallest fix:** Optional decode + validate at create time behind feature flag; store hash of canonical bytes.

**Largest bet:** Shared `multisig-core` crate consumed by desktop + backend for single validation implementation.

---

### HIGH: DD3 — Authority is a type, but **no explicit bounded context map** explains relationship between `Authority`, ASM RPC membership, and session binding

**Risk:** Ubiquitous language gap: “authority” in API vs onchain role sets vs signer index ordering — integration bugs (wrong ordered keys) hide until broadcast.

**Evidence:** `domain/authority.rs` (not fully expanded in audit pass — enum present); `infrastructure/asm_role_membership.rs` used from handlers + broadcast; ADR-005 mentions five authorities but no context map diagram in-repo.

**Smallest fix:** One-page context map: **Coordination BC** ↔ **Signer membership BC (ASM read)** ↔ **Bitcoin execution BC** with integration patterns (ACL / OHS).

**Largest bet:** Anti-corruption layer types wrapping ASM DTOs, not passing URLs through every function.

---

### MEDIUM: DD4 — ADR-002 explicitly rejects tactical DDD patterns; ADR-005 uses “layered” language — product team may still say “aggregate” in meetings

**Risk:** Misaligned expectations during design reviews — auditors score “poor DDD” while engineers followed deliberate “transaction script” style.

**Evidence:** `docs/architecture/adrs/002-application-layer-strategy.md` — “What we explicitly avoid: … aggregate roots, value objects as distinct types …”.

**Failure scenario:** Wasted sprint on event storming when problem fit is CRUD coordination; or opposite — missing state machine rigor because “no aggregates” was read as “no rules”.

**Smallest fix:** Glossary in `docs/architecture/` tying **Proposal**, **ActionId**, **BroadcastStatus** to PRD/SPS terms (user-requested doc only).

**Largest bet:** Selective tactical DDD only on payout admin divergence (per ADR-002 table).

---

### MEDIUM: DD5 — Duplicate prevention is **identity-based** (`ActionId`), not **business uniqueness** of governance intent

**Risk:** Two legitimate competing proposals with different encodings of “same intent” hash differently — governance process duplication, not caught by model.

**Evidence:** `compute_action_id(seq_no, action_hex)` — purely syntactic identity.

**Smallest fix:** Optional human `title` / `rationale_id` from PRD if product requires dedup by intent.

**Largest bet:** Content-hash governance graph linking offchain deliberation to onchain action.

---

## Attack narratives

1. **The “Pending but not enactable” proposal:** Malicious or buggy client submits garbled `action_hex`; signatures accumulate; broadcast fails at SSZ decode. **Outcome:** DDD lens says **wrong aggregate boundary** — invalid protocol commands should be rejected at aggregate creation, not at side-effectful context.

2. **The “two adapters, two rules”** maintenance: Engineer adds `Expired` transition in SQL repo but forgets memory repo. **Outcome:** e2e passes on default memory in CI while production Postgres diverges — classic **missing aggregate root** problem.

3. **The “aggregate theater” requirement:** Compliance asks for event log per state change; codebase has none. **Outcome:** expensive retrofit because transaction scripts did not emit domain events.

---

## Evidence index (paths)

| Area | Path |
|------|------|
| Domain types / ActionId | `orchestrator-be/src/domain/proposal.rs` |
| Authority enum | `orchestrator-be/src/domain/authority.rs` |
| Transaction scripts | `orchestrator-be/src/application/proposals.rs` |
| Repository port (persistence) | `orchestrator-be/src/application/traits.rs` |
| ADR: deliberate non-DDD | `docs/architecture/adrs/002-application-layer-strategy.md` |
| ADR: layering / desktop domain | `docs/architecture/adrs/005-layered-architecture.md` |
| Workspace rules (coordination only) | `AGENTS.md`, `.cursor/rules/general.mdc` |

---

## Smallest fixes vs largest bets

| Finding | Smallest fix | Largest bet |
|---------|--------------|-------------|
| DD1 | Explicit state machine module | Aggregate + optimistic concurrency |
| DD2 | Validate SSZ at create | Shared `multisig-core` validation |
| DD3 | Context map doc + ACL types | Bounded context per deployment unit |
| DD4 | Glossary + ADR banners | Workshop alignment |
| DD5 | Optional `intent_id` field | Offchain deliberation linkage |

---

## What would change my mind

- **DD2:** Product explicitly accepts **late validation** (offchain hash-only commitments) as threat-model trade-off — document as intentional **two-phase** consistency boundary.
- **DD1:** Demonstrated test suite that **mutates states only through** repository transactions and proves parity across adapters — functionally equivalent to aggregate enforcement.

---

## Conclusion

**Honest DDD read:** The backend is a **coordination context** with **transaction-script** style over **anemic** `domain` records — consistent with ADR-002 and AGENTS.md “coordination only.” The main **DDD adversarial** concerns are **late protocol validation** (DD2), **invariant scatter** (DD1), and **missing explicit context map** for ASM/Bitcoin adjacency (DD3). This is not a call to introduce event sourcing; it **is** a call to strengthen **state machine clarity** and **validation boundaries** if the product treats offchain proposals as authoritative precursors to irreversible governance.
