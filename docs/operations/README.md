# Operations folder index

**Audience:** Engineers running, releasing, and securing the multisig stack.

## Current (SSOT)

| File | Role |
|------|------|
| [`runbook.md`](./runbook.md) | **SSOT** for backend ops (health, env, local stack) |
| [`threat-model.md`](./threat-model.md) | **SSOT** for assets, trust boundaries, top risks (pair with [`specs/signer-safety-model.md`](../specs/signer-safety-model.md)) |
| [`executable-delivery-plan.md`](./executable-delivery-plan.md) | **SSOT** for internal release program (D1–D8); concluded 2026-06-05 |
| [`asm-pin-bump-reset.md`](./asm-pin-bump-reset.md) | **SSOT** for resetting orchestrator DB, ASM runner and regtest state after a wire-breaking ASM pin bump |

## Client-facing (canonical for delivery steps)

| Topic | Document |
|-------|----------|
| Verify releases | [`external/verifying-releases.md`](../external/verifying-releases.md) |
| Reproducible builds | [`external/reproducible-builds.md`](../external/reproducible-builds.md) |
| Build and release process | [`external/build-and-release-process.md`](../external/build-and-release-process.md) |

## Internal pointers and research

| File | Purpose |
|------|---------|
| [`reproducible-builds.md`](./reproducible-builds.md) | Pointer → `external/reproducible-builds.md` + internal evidence links |
| [`reproducible-builds-research.md`](./reproducible-builds-research.md) | D4 research and tier analysis |
| [`release-signing-mvp.md`](./release-signing-mvp.md) | Internal release-signing notes |
| [`desktop-build-linux.md`](./desktop-build-linux.md) | Local build prerequisites (D1) |
| [`platform-code-signing-requirements.md`](./platform-code-signing-requirements.md) | Code-signing requirements |
| [`multi-employee-signing-requirements.md`](./multi-employee-signing-requirements.md) | Multi-signer release policy |
| [`windows-build-incompatibilities.md`](./windows-build-incompatibilities.md) | Windows build notes |
| [`windows-portability-upstream-issues.md`](./windows-portability-upstream-issues.md) | Upstream Windows issues |
