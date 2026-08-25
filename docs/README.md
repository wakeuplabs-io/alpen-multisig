# Internal documentation map

**Audience:** WakeUp Labs engineers and agents working in this repository.

Client-facing deliverables live in [`external/`](./external/README.md). Frozen client inputs are in [`0-prd/`](./0-prd/) and [`1-proposal/`](./1-proposal/) — do not modify.

## Document flow

```mermaid
flowchart LR
  PRD["0-prd / 1-proposal"]
  DISC["2-discovery"]
  STORIES["3-stories"]
  ARCH["architecture + ADRs"]
  SPECS["specs"]
  ARCHIVE["archive/"]
  EXT["external/"]

  PRD --> DISC --> STORIES --> ARCH --> SPECS
  SPECS --> ARCHIVE
  SPECS --> EXT
```

## Where to look (SSOT by topic)

| If you need… | Read first | Status |
|--------------|------------|--------|
| Client PRD (latest snapshot) | [`0-prd/06-prd-hardware-signer-and-block-payouts-update.md`](./0-prd/06-prd-hardware-signer-and-block-payouts-update.md) | Current |
| System architecture (internal) | [`architecture/overview.md`](./architecture/overview.md) + [`architecture/adrs/`](./architecture/adrs/) | Current |
| Client-facing architecture | [`external/architecture-overview.md`](./external/architecture-overview.md) | Client |
| Coordination boundary (backend vs desktop) | [ADR-006](./architecture/adrs/006-backend-coordination-boundary.md) | Current |
| Admin Wallet PRD §4 compliance | [`specs/admin-wallet-prd-compliance.md`](./specs/admin-wallet-prd-compliance.md) | Current |
| Capability / coverage pointers | [`architecture/overview.md`](./architecture/overview.md#capability-status-where-to-look), [`3-stories/README.md`](./3-stories/README.md#capability-status-where-to-look) | Current |
| Open backlog (user stories + NFRs) | [`assessment/deferred-backlog.md`](./assessment/deferred-backlog.md) | Current |
| Wave / P-ID closure tracking | [`assessment/action-plan-progress.md`](./assessment/action-plan-progress.md) | Current |
| Ops (runbook, local stack) | [`operations/runbook.md`](./operations/runbook.md), [`operations/README.md`](./operations/README.md) | Current |
| Release program (internal tracking) | [`operations/executable-delivery-plan.md`](./operations/executable-delivery-plan.md) | Current |
| Verify releases / reproducible builds (client steps) | [`external/verifying-releases.md`](./external/verifying-releases.md), [`external/reproducible-builds.md`](./external/reproducible-builds.md) | Client |
| Security model | [`operations/threat-model.md`](./operations/threat-model.md), [`specs/signer-safety-model.md`](./specs/signer-safety-model.md) | Current |
| Security Council (Defcon, safe harbour, council signer update) | [`specs/security-council.md`](./specs/security-council.md) | Current |
| Security Council — Defcon 1 functional contract (V1) | [`specs/security-council-defcon.md`](./specs/security-council-defcon.md) | Current |
| Phase 1 research evidence | [`2-discovery/README.md`](./2-discovery/README.md) | Historical / reference |
| POC / walking-skeleton specs | [`archive/poc-specs/`](./archive/poc-specs/) | Historical |
| Codebase reviews and implementation audits | [`assessment/audits/`](./assessment/audits/) (see resolution banners) | Historical / reference |

## SSOT vs. historical competitors

Use this table when two internal docs seem to disagree. **SSOT** is where current truth lives; **do not use** lists common lookalikes that are historical, phased, or superseded.

| Domain | SSOT (read for current truth) | Do not use for current truth | Why |
|--------|-------------------------------|------------------------------|-----|
| System architecture | [`architecture/overview.md`](./architecture/overview.md), [`architecture/adrs/`](./architecture/adrs/) | [`archive/poc-specs/`](./archive/poc-specs/) | POC specs are pre-production snapshots |
| Admin Wallet PRD §4 | [`specs/admin-wallet-prd-compliance.md`](./specs/admin-wallet-prd-compliance.md) | [`specs/admin-wallet-implementation-plan.md`](./specs/admin-wallet-implementation-plan.md) | Plan tracks engineering slices; PASS/FAIL lives only in the compliance matrix ([conflict rule #4](#conflict-resolution)) |
| Feature contract | `specs/<feature>.md` | `*-implementation.md`, [`archive/features/`](./archive/features/) `feature-delta.md`, [`archive/evolution/`](./archive/evolution/) | See [`specs/README.md`](./specs/README.md) and [`archive/README.md`](./archive/README.md) ([conflict rule #3](#conflict-resolution)) |
| Architecture decisions | [`architecture/adrs/`](./architecture/adrs/) | [`2-discovery/`](./2-discovery/), dated assessments | Discovery is Phase 1 evidence; accepted decisions are ADRs only |
| Backlog & closure | [`assessment/deferred-backlog.md`](./assessment/deferred-backlog.md), [`assessment/action-plan-progress.md`](./assessment/action-plan-progress.md) | [`assessment/archive/`](./assessment/archive/) wave snapshots | See [`assessment/README.md`](./assessment/README.md) ([conflict rule #6](#conflict-resolution)) |
| Operations | [`operations/runbook.md`](./operations/runbook.md) | [`assessment/archive/action-plan-2026-05-14.md`](./assessment/archive/action-plan-2026-05-14.md) P-051 row | Runbook exists; 2026-05-14 table is a historical snapshot |
| Security | [`operations/threat-model.md`](./operations/threat-model.md), [`specs/signer-safety-model.md`](./specs/signer-safety-model.md) | — (pair is joint SSOT) | Threat model = assets/risks; signer-safety = UX principles; read both |
| Security Council | [`specs/security-council.md`](./specs/security-council.md) | [`2-discovery/08-alpen-crate-prd-coverage.md`](./2-discovery/08-alpen-crate-prd-coverage.md), [`2-discovery/19-asm-bump-impact-assessment.md`](./2-discovery/19-asm-bump-impact-assessment.md) | Both discovery notes predate the pin bump and call the council blocked on upstream; the spec carries the upstream evidence and the current stage board |

Folder indexes: [`assessment/README.md`](./assessment/README.md), [`specs/README.md`](./specs/README.md), [`operations/README.md`](./operations/README.md), [`archive/README.md`](./archive/README.md).

## Conflict resolution

When documents disagree, use this order:

1. **Protocol:** SPS-50, SPS-51, SPS-65 (via Alpen crates and PRDs).
2. **Architecture decisions:** `architecture/adrs/` (e.g. ADR-006 over legacy assessment claims).
3. **Feature contract:** `specs/<feature>.md` (functional spec wins over implementation spec if they conflict).
4. **Admin Wallet PRD status:** `admin-wallet-prd-compliance.md` over phase checkmarks in `admin-wallet-implementation-plan.md`.
5. **Client deliverables:** `docs/external/` over internal stubs in `operations/`.
6. **Backlog truth:** `deferred-backlog.md` + `action-plan-progress.md` over historical `assessment/archive/action-plan-2026-05-14.md` severity tables.

## Directory index

| Path | Purpose |
|------|---------|
| `0-prd/`, `1-proposal/` | Frozen client inputs. PRD snapshots are numbered (`01`…`06`); the highest number is the current one, earlier ones are kept as history |
| `2-discovery/` | Phase 1 research (historical); includes [`crate-inventory.md`](./2-discovery/crate-inventory.md) |
| `3-stories/` | Story map and non-functional items |
| `architecture/` | Overview + ADRs |
| `specs/` | Per-feature contracts — see [`specs/README.md`](./specs/README.md) |
| `assessment/` | Backlog SSOT, [`audits/`](./assessment/audits/), [`archive/`](./assessment/archive/) — see [`assessment/README.md`](./assessment/README.md) |
| `operations/` | Runbook, threat model, delivery plan — see [`operations/README.md`](./operations/README.md) |
| `archive/` | Historical delivery records, evolution, POC specs — **not SSOT** |
| `external/` | Client deliverables (canonical for delivery) |

## Agent rules

**SSOT:** [`.cursor/rules/`](../.cursor/rules/) (Cursor). [`.claude/rules/`](../.claude/rules/) is a maintained mirror for Claude Code — keep in sync when editing agent conventions.

| Rule file | Scope |
|-----------|--------|
| `general.mdc` | Global conventions (minimal; see also root `AGENTS.md`) |
| `typescript-standards` | `desktop-app/src/**/*.{ts,tsx}` |
| `react-frontend-patterns` | React screens and hooks |
| `rust-backend-standards` | `orchestrator-be`, `desktop-app/src-tauri` |
| `backend-api-conventions` | `orchestrator-be` HTTP handlers |

Project commands and architecture summary: [`AGENTS.md`](../AGENTS.md).
