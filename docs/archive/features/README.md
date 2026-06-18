# Feature delivery records (archive)

**Audience:** Engineers reviewing historical delivery execution artifacts.

Each subdirectory holds **execution records** — not the functional contract for the feature.

## What lives here

| Artifact | Purpose |
|----------|---------|
| `deliver/roadmap.json` | Phased execution plan for the feature goal |
| `deliver/execution-log.json` | Step-by-step execution record |
| `deliver/.develop-progress.json` | In-flight progress snapshot |
| `feature-delta.md` | Summary of what changed vs. the baseline spec |

## SSOT for behavior

**Do not** treat `feature-delta.md` or roadmap JSON as the feature contract. Read:

1. [`specs/<feature>.md`](../../specs/) — functional SSOT
2. [`specs/README.md`](../../specs/README.md) — full layer order
3. [`archive/evolution/`](../../archive/evolution/) — post-merge summaries

If `feature-delta.md` contradicts the functional spec, the spec wins.

See also [`archive/README.md`](../README.md).
