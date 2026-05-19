# Wave 2 Track F — follow-up backlog

PR [#141](https://github.com/wakeuplabs-io/alpen-multisig/pull/141) merged **P-051** onto `develop`:

| Artifact | Path |
|----------|------|
| Ops runbook | [runbook.md](../operations/runbook.md) |
| Threat model | [threat-model.md](../security/threat-model.md) |
| Signer safety model | [signer-safety-model.md](../specs/signer-safety-model.md) |
| README links | Root [README.md](../../README.md) § Repository Layout |

## Not in #141 (Wave 2 plan remainder)

| P-ID | Status | Notes |
|------|--------|-------|
| **P-005** | open | On-device verification UX — implement after G digest usability findings |
| **P-006** | open | HW vs software signing payload parity / verify gate |
| **P-055** | blocked | SPS-50/51/65 excerpts under `docs/specs/sps-reference/` — [gate §4](wave2-human-decisions-pending.md) |

## Dependencies

- **P-006** ← [wave2-p053-digest-usability.md](../2-discovery/wave2-p053-digest-usability.md) sessions (Track G execution).
- **P-055** ← Alpen legal sign-off on §4.
