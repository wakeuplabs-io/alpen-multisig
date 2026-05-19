# Wave 2 Track G — follow-up backlog

PR [#142](https://github.com/wakeuplabs-io/alpen-multisig/pull/142) merged **P-053 planning** onto `develop`:

| Artifact | Path |
|----------|------|
| Signer interview plan | [wave2-p053-interview-plan.md](../2-discovery/wave2-p053-interview-plan.md) |
| Digest verification usability protocol | [wave2-p053-digest-usability.md](../2-discovery/wave2-p053-digest-usability.md) |

Human gate log lives on `develop`: [wave2-human-decisions-pending.md](wave2-human-decisions-pending.md) (not part of #142 diff).

## Execute (product / research)

| Item | Owner | Output |
|------|--------|--------|
| Recruit 5–8 signers | Product | Scheduled sessions per interview plan |
| Run interviews | Product | Findings doc in `docs/2-discovery/` |
| Digest usability sessions | UX + Product | Pass/fail vs ≥80% criteria in usability doc |
| US-H5 tabletop | Product + Eng | **After gate §3 decision** — scenario in interview plan §4 |

## Feeds other tracks

| Consumer | Use |
|----------|-----|
| **P-006** (Track F) | On-device verification copy from usability failure modes |
| **Track E** | E2E negative matrix; orchestrator-down scope **blocked on gate §3** |
| **P-052 / Wave 3** | Slice plan + walking-skeleton scope for US-H5 |

## Still pending (not resolved by merging G)

| Gate | Blocks |
|------|--------|
| §3 US-H5 scope | Track E full fallback WDIO |
| §4 P-055 legal | Track F SPS archive import |
