# Product discovery & assumptions — adversarial axis (read-only review)

**Audit date:** 2026-05-14  
**Lens:** `docs/0-prd/`, `docs/2-discovery/`, `docs/3-stories/` vs observable code trajectory

---

## Scope

Cross-check **written product intent** (PRD-backed requirements and story slicing) against **implementation signals** (orchestrator behavior, desktop commands, specs). Adversarial: where does marketing-shaped clarity hide engineering debt?

---

## Top findings (ranked)

1. **Backend PRD demands authenticated access for multisig state and strict non-signer isolation** (`docs/0-prd/02-multisig-backend.md`, §3–4). Codebase invests in sessions + membership checks (see orchestrator handlers/tests), **but verifying full information-leak parity** (timing, enumeration, cross-authority) requires adversarial HTTP tests beyond what this read confirmed — **risk of assumption “we enforced auth ergo PRD §3 satisfied.”**
2. **Walking skeleton slice targets raw export without in-app broadcast** (`docs/3-stories/story-map.md`, Slice 0 table) **while codebase registers `proposals_broadcast` and orchestrator routes** (`desktop-app/src-tauri/src/main.rs`; `orchestrator-be/src/handlers/mod.rs`). **Slice vs shipped surface drift** can confuse stakeholder expectations about POC completion.
3. **UI PRD envisions Ledger + Trezor class coverage** (`docs/3-stories/story-map.md`, US-B1 discovery notes) **against a Tauri command set heavily Trezor-exposed** (`sign_with_trezor`, `get_trezor_info`, … in `desktop-app/src-tauri/src/main.rs`). Hardware parity is likely **upstream-gated**, not laziness — but backlog communication must spell that dependency.
4. **Story map separates multi-authority expansion to later slices** (`docs/3-stories/story-map.md`, Slice 2) **while backend domain enumerates five authorities today** (`orchestrator-be/src/domain/authority.rs` implied by postgres mapping strings). Capability exists before product slice — potential **scope creep temptation** versus minimal skeleton.
5. **Discovery backlog calls out ASM bumps and crate readiness** (see `docs/2-discovery/` files such as `12-upstream-readiness-findings.md`, `15-nightly-dependency-finding.md`). **Risk:** roadmap dates assume crates stable while CI/e2e still pins nightly quirks — assumptions about “upstream done” brittle.

---

## Attack narratives

1. **Stakeholder demos Slice 0** using in-app broadcast because commands exist → compliance story falsely claims thinner scope than contracted walking skeleton artifacts.
2. **Security reviewer reads PRD §3 non-inference clause** literal — finds list endpoints return `404` uniformly or not — **without** curated negative matrix, reviewer blocks release.
3. **Product prioritizes Ledger parity** (`docs/3-stories/story-map.md`) **engineering ships Trezor path** (`main.rs`) — customer expects device support matrix; backlog mismatch surfaces post-pilot.
4. **Operational assumption:** “Orchestrator is HA Postgres-backed per PRD §2 (`docs/0-prd/02-multisig-backend.md`).” Developers run default in-memory (`orchestrator-be/src/main.rs`) — rehearsal incidents never match prod topology.
5. **Manual fallback story (Slice 5)** promises compose-without-backend eventually (`docs/3-stories/story-map.md`). If interim UX hides export paths, **PRD failover clause satisfaction** slips despite technical possibility.

---

## Evidence index

| Topic | Path |
|--------|------|
| Backend PRD (coordination boundaries, failover, ACL) | `docs/0-prd/02-multisig-backend.md` |
| UI PRD reference & external link | `docs/0-prd/01-multisig-ui.md` |
| Discovery index | `docs/2-discovery/README.md` |
| Story map slices vs stories | `docs/3-stories/story-map.md` |
| Non-functional split | `docs/3-stories/non-functional-items.md` |
| Architecture “current state” | `docs/architecture/overview.md` |
| Implemented HTTP routes | `orchestrator-be/src/handlers/mod.rs` |
| Desktop command surface | `desktop-app/src-tauri/src/main.rs` |
| POC / application specs | `docs/specs/application-layer-setup.md`, `docs/specs/poc4-step*.md` |

---

## Smallest vs largest bets

| Size | Bet |
|------|-----|
| **Smallest** | Add a three-row “slice ↔ route/command” reconciliation table into an existing roadmap doc (truth table only, no prose debate). |
| **Largest** | Living compliance matrix tying each PRD SHALL to tests + telemetry signals, fed by UX research transcripts in `docs/2-discovery/` to validate manual fallback realism. |

---

## What would change my mind

- A **printed test matrix screenshot** proving non-signers cannot distinguish pending-empty vs forbidden across authorities (stable status/body/time).
- A **delivery checklist** artifact showing Slice 0 explicitly excludes broadcast paths from acceptance, or updates stories to absorb existing broadcast UX.
- **Customer interview excerpts** validating manual fallback ergonomics aligned with Slice 5, not hypothetical.
