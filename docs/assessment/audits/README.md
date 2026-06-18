# Assessment audits

**Audience:** Engineers tracing gap findings from dated reviews and implementation audits.

These files are **not SSOT** for current behavior or backlog. They record what was observed at a point in time. Read resolution banners at the top of each file before acting on findings.

## Authority order

| Need | Read |
|------|------|
| Current backlog | [`../deferred-backlog.md`](../deferred-backlog.md) |
| P-ID / wave closure | [`../action-plan-progress.md`](../action-plan-progress.md) |
| Delivery spec for a gap | [`../../specs/`](../../specs/) |
| Architecture decisions | [`../../architecture/adrs/`](../../architecture/adrs/) |

Audits do not override `specs/`, ADRs, or the assessment SSOT trackers above.

## Contents

| File | Type | Notes |
|------|------|-------|
| [`2026-05-22-review-comprehensive.md`](./2026-05-22-review-comprehensive.md) | Codebase review | Full-stack audit on `develop` (2026-05-22); open items → deferred backlog |
| [`2026-05-09-broadcast-audit.md`](./2026-05-09-broadcast-audit.md) | Targeted audit | Historical; superseded by [ADR-006](../../architecture/adrs/006-backend-coordination-boundary.md) + commit/reveal spec |
| [`proposal_status_lifecycle_audit.md`](./proposal_status_lifecycle_audit.md) | Implementation audit | PRD §3–4 lifecycle gaps; delivery spec → [`proposal-lifecycle-expiry-and-status-completion.md`](../../specs/proposal-lifecycle-expiry-and-status-completion.md) |
