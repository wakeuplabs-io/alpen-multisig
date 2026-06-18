# Assessment folder index

**Audience:** Engineers tracking backlog closure, wave history, and P-ID resolution.

This folder mixes **current trackers** with **historical snapshots** under [`archive/`](./archive/). Do not treat every file here as current truth.

## Current (SSOT)

| File | Role |
|------|------|
| [`deferred-backlog.md`](./deferred-backlog.md) | **SSOT** for open user stories and NFRs deferred after Waves 1–3 |
| [`action-plan-progress.md`](./action-plan-progress.md) | **SSOT** for P-ID / wave closure status on `develop` |

When an item is PASS in `action-plan-progress.md`, a dated assessment row does **not** reopen it unless progress is updated first.

## Audits ([`audits/`](./audits/))

Point-in-time codebase reviews and implementation audits. **Not SSOT** — read resolution banners; open items flow to [`deferred-backlog.md`](./deferred-backlog.md) or delivery specs in [`specs/`](../specs/). See [`audits/README.md`](./audits/README.md).

## Historical / snapshot ([`archive/`](./archive/))

| File | Role |
|------|------|
| [`archive/action-plan-2026-05-14.md`](./archive/action-plan-2026-05-14.md) | May 2025 synthesis; severities and “missing doc” claims are not re-audited |
| [`archive/wave2-exit-gap-review.md`](./archive/wave2-exit-gap-review.md) | Wave 2 sign-off checklist (closed 2026-05-19) |
| [`archive/wave2-human-decisions-pending.md`](./archive/wave2-human-decisions-pending.md) | Human gates resolved at Wave 2 close |
| [`archive/wave2-track-d-followups.md`](./archive/wave2-track-d-followups.md) | Track D deferrals |
| [`archive/wave2-track-e-followups.md`](./archive/wave2-track-e-followups.md) | Track E deferrals |
| [`archive/wave2-track-f-followups.md`](./archive/wave2-track-f-followups.md) | Track F deferrals |
| [`archive/wave2-track-g-followups.md`](./archive/wave2-track-g-followups.md) | Track G deferrals |
| [`archive/wave3-stabilization-execution-playbook.md`](./archive/wave3-stabilization-execution-playbook.md) | Wave 3 execution playbook (closed) |

## Conflict resolution

See [Conflict resolution](../README.md#conflict-resolution) in the internal documentation map — rule #6: `deferred-backlog.md` + `action-plan-progress.md` win over `archive/action-plan-2026-05-14.md` severity tables.
