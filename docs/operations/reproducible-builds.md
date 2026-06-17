# Reproducible builds — internal pointer

**Client-facing guide (canonical):** [`docs/external/reproducible-builds.md`](../external/reproducible-builds.md)

Use the external document for verification steps, tier definitions, and PRD §1.2 traceability.

## Internal-only references

| Document | Purpose |
|----------|---------|
| [`executable-delivery-plan.md`](./executable-delivery-plan.md) | D4 deliverable status and closure (internal SSOT for the release program) |
| [`reproducible-builds-research.md`](./reproducible-builds-research.md) | Evidence, tier analysis, and trade-offs that informed D4 scope |
| [`desktop-build-linux.md`](./desktop-build-linux.md) | Local build prerequisites (D1) |

Implementation anchors: `scripts/verify-reproducible-build.sh`, `.github/workflows/release.yml`, root `Cargo.toml` (`trim-paths`), `rust-toolchain.toml`, `.nvmrc`.
