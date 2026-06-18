# Feature folder index (nWave DELIVER)

**Audience:** Engineers reviewing nWave execution artifacts.

Each subdirectory (`admin-wallet-*`, `fix-*`, etc.) holds **DELIVER wave outputs** — not the functional contract for the feature.

## What lives here

| Artifact | Purpose |
|----------|---------|
| `deliver/roadmap.json` | Phased execution plan for the feature goal |
| `deliver/execution-log.json` | Step-by-step execution record |
| `deliver/.develop-progress.json` | In-flight progress snapshot |
| `feature-delta.md` | Summary of what changed vs. the baseline spec |

## SSOT for behavior

**Do not** treat `feature-delta.md` or roadmap JSON as the feature contract. Read:

1. [`specs/<feature>.md`](../specs/) — functional SSOT
2. [`specs/README.md`](../specs/README.md) — full layer order

If `feature-delta.md` contradicts the functional spec, the spec wins.

## After merge

Post-merge summaries move to [`evolution/`](../evolution/) — see [`evolution/README.md`](../evolution/README.md).
