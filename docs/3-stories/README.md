# `3-stories/` — Qualified user stories & story map

This folder contains the product-scope definition derived from the client PRDs (`0-prd/`), grounded by discovery findings (`2-discovery/`). It is the handoff from *discovery* to *architecture + per-feature specs* (`architecture/`, `specs/`).

## Contents

| File | Purpose |
|---|---|
| [`story-map.md`](./story-map.md) | Primary deliverable. Jeff-Patton-style story map with backbone (user activities), slicing (releases), and qualified functional user stories. Includes the walking skeleton. |
| [`non-functional-items.md`](./non-functional-items.md) | Non-functional requirements extracted from the PRDs and discovery. These become specs, not user stories. |

## How to read this

1. **Start with the story map.** The backbone (§2) is the user journey; slices (§3) are release increments.
2. **Slice 0 (Walking Skeleton)** is the thinnest end-to-end path proving the system works with the authority+action combo the team has already proven in POC-3/4 (Strata Admin signer update).
3. **Later slices** depend on upstream Alpen work (missing update types, missing authority definitions). Risks and dependencies are surfaced at the end of the story map.
4. **Non-functional items** live in their own document because they carry no end-user behavior.

## Principles applied

- **Story mapping over product spec.** The map expresses scope as activities and slices, not as a static feature list.
- **No "as a developer" stories.** Infra, ops, and security-invariant work goes in `non-functional-items.md`.
- **Minimal and faithful to PRDs.** Acceptance signals are lifted from the PRD; nothing invented.
- **Technical details deferred.** This layer answers *what ships to the user* and *in what order*. *How* is the job of `architecture/` + `specs/`.

## What comes next

1. Review and ratify the map with stakeholders.
2. For each story selected for the next slice, author a minimal spec in `docs/specs/` (one concern per spec).
3. Surface non-functional items that gate the active slice and promote them to specs first (per discovery-informed ordering).
