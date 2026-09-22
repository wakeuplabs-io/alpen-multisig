# Security Council — per-phase delivery specs (archive)

**Historical only — not SSOT.** These are the detail specs each delivery phase was built from, for
the four Security Council slices. They record what a phase set out to do, the decisions taken while
doing it, and the mutation tables and review findings behind them. They are kept because the
functional contracts and build plans cite them as evidence, not because they describe the current
system.

| Slice | Phase specs | Functional contract (SSOT) | Build plan |
|---|---|---|---|
| V1 — Defcon 1 | `security-council-defcon-phase-1.md` … `-phase-7.md` | [`security-council-defcon.md`](../../../specs/security-council-defcon.md) | [`security-council-defcon-implementation.md`](../../../specs/security-council-defcon-implementation.md) |
| V2 — Defcon 3 and its cancel | `security-council-defcon-3-phase-1.md` … `-phase-7.md` | [`security-council-defcon-3.md`](../../../specs/security-council-defcon-3.md) | [`security-council-defcon-3-implementation.md`](../../../specs/security-council-defcon-3-implementation.md) |
| V3 — Security Council signer update | `security-council-signer-update-phase-1.md` … `-phase-4.md` | [`security-council-signer-update.md`](../../../specs/security-council-signer-update.md) | [`security-council-signer-update-implementation.md`](../../../specs/security-council-signer-update-implementation.md) |
| V4 — Safe Harbor address update | `security-council-safe-harbor-address-phase-1.md` … `-phase-4.md` | [`security-council-safe-harbor-address.md`](../../../specs/security-council-safe-harbor-address.md) | [`security-council-safe-harbor-address-implementation.md`](../../../specs/security-council-safe-harbor-address-implementation.md) |

Scope, staging and status for the whole feature live in
[`specs/security-council.md`](../../../specs/security-council.md). If anything here contradicts a
contract, the contract wins.

File paths and line numbers quoted inside these specs were true when each phase shipped and have
moved since.
