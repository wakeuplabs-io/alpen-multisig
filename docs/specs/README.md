# Specs folder index

**Audience:** Engineers implementing or reviewing feature behavior.

## Contract layer order (highest authority first)

When documents disagree about **what the system should do**, use this order:

| Layer | Location | Purpose |
|-------|----------|---------|
| 1. Functional contract | `specs/<feature>.md` | **SSOT** for feature behavior and acceptance |
| 2. Implementation notes | `specs/<feature>-implementation.md`, phase plans | How the slice was built; may lag the functional spec |
| 3. DELIVER delta | [`archive/features/<name>/feature-delta.md`](../archive/features/) | nWave execution delta; not a substitute for layer 1 |
| 4. Archived summary | [`evolution/`](../evolution/) | Post-merge narrative; historical |

See [Conflict resolution](../README.md#conflict-resolution) rule #3: functional spec wins over implementation spec.

## Admin Wallet (special case)

| Need | SSOT |
|------|------|
| PRD §4 PASS / FAIL / PARTIAL | [`admin-wallet-prd-compliance.md`](./admin-wallet-prd-compliance.md) |
| Phase / release engineering checklist | [`admin-wallet-implementation-plan.md`](./admin-wallet-implementation-plan.md) |

Compliance matrix wins over phase checkmarks ([conflict rule #4](../README.md#conflict-resolution)).

## Historical POC specs

Files matching `poc*.md` are bannered **historical** — walking-skeleton evidence from Phase 1. Do not use them for current architecture or behavior; see [`architecture/overview.md`](../architecture/overview.md).

## Related indexes

- DELIVER artifacts: [`archive/features/README.md`](../archive/features/README.md)
- Archived features: [`evolution/README.md`](../evolution/README.md)
