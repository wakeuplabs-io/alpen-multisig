# Testing strategy — adversarial axis (read-only review)

**Audit date:** 2026-05-14  
**Lens:** Automated tests, negative paths, IPC coverage, test-suite economics (optimizer mindset)

---

## Scope

**In scope:** Rust workspace tests (`orchestrator-be`, `desktop-app/src-tauri`, `e2e-tests`), CI expectations, gap between HTTP/Tauri IPC surfaces and executable tests, duplication vs missing negative coverage. **Out of scope:** Formal verification of SPS-65 (protocol crates own that), manual QA calendars.

---

## Top findings (ranked)

1. **HTTP integration tests bind to in-memory repos and mock-ish RPC URLs, not Postgres or real ASM membership.** Every handler test builds `InMemoryProposalRepository` (`orchestrator-be/src/handlers/mod.rs`, `#[cfg(test)] mod tests`, `test_app_with_rpc_url`), so **SQL-specific failures** (constraints, transaction isolation, migration drift) are largely unobserved in handler tests.
2. **Desktop frontend has no `npm test` / component harness** — only `lint`, `format:check`, `build` (`desktop-app/package.json`). Regressions in React flows are caught by typecheck + human runs, not automated UI tests.
3. **Tauri IPC surface is wide** (`desktop-app/src-tauri/src/main.rs` `generate_handler![...]`) **while Rust unit tests concentrate in modules** (`desktop-app/src-tauri/src/application/proposals.rs` carries multiple `#[tokio::test]`). Cross-command sequencing (auth → propose → approve → broadcast) is under-tested relative to IPC cardinality.
4. **E2E coverage is precious but narrow and nightly-weighted.** Workspace houses multiple `e2e-tests/tests/*.rs` files; CI spec still frames e2e as a separate cargo invocation (`docs/specs/ci-pipeline.md`). Optimizer lens: risk of **parallel suites** asserting overlapping happy paths while rare IPC negatives stay absent.
5. **Negative auth tests exist at HTTP layer** (e.g., non-member verify) (`orchestrator-be/src/handlers/mod.rs`, `test_auth_verify_non_member_rejected`), **but equivalence across desktop bridge** depends on mirrored behavior in `desktop-app/src/api/tauri-bridge.ts` without a mirrored automated contract test.

---

## Attack narratives

1. **Migration passes, query fails:** A column rename diverges from `SELECT_PROPOSAL_COLS` in `postgres_repo.rs` — unit tests green, production 500 on list proposals.
2. **IPC argument skew:** Frontend passes a renamed field to `invoke`; TypeScript builds (loose boundary) — **first failure is user runtime**, not CI, because no IPC schema test pins command payloads.
3. **E2E green, desktop broken:** E2e exercises libraries + worker harness paths (`e2e-tests/`) while Tauri command wiring regressions slip until manual `npm run tauri dev`.
4. **Test duplication inflation:** Parallel tests copy fixture setup from `handlers/mod.rs` and `application/proposals.rs`, raising maintenance cost — optimizer-target for parametrize/consolidate without dropping assertions.
5. **Broadcast race blind spot:** Postgres `claim_broadcast` uses conditional `UPDATE` (`postgres_repo.rs`); without concurrent integration tests, two operators might still stress UI-level double-submit unpredictably.

---

## Evidence index

| Topic | Path |
|--------|------|
| CI expectations (rust + e2e + frontend) | `docs/specs/ci-pipeline.md`, `.github/workflows/ci.yml` |
| Handler tests | `orchestrator-be/src/handlers/mod.rs` |
| Application-layer proposal tests | `orchestrator-be/src/application/proposals.rs` |
| Desktop proposal unit tests | `desktop-app/src-tauri/src/application/proposals.rs` |
| Orchestrator client tests | `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs` |
| E2E tests | `e2e-tests/tests/*.rs`, `docs/specs/e2e-tests-workspace-integration.md` |
| Frontend scripts (no jest/vitest) | `desktop-app/package.json` |
| Tauri command registration | `desktop-app/src-tauri/src/main.rs` |
| Frontend invoke wrapper | `desktop-app/src/api/tauri-bridge.ts` |

---

## Smallest vs largest bets

| Size | Bet |
|------|-----|
| **Smallest** | One `docker-compose` Postgres + `#[tokio::test]` migration smoke that runs `sqlx::migrate!` and a round-trip insert/list against `PostgresProposalRepository`. |
| **Largest** | Full IPC contract tests (generate JSON schema from commands), Playwright-driving Tauri, and coverage-guided pruning via a formal test-optimization pass (`nw-test-optimizer` pattern) once baselines stabilize. |

---

## What would change my mind

- CI running **repository integration tests** against ephemeral Postgres on every PR.
- A **minimal Playwright/Vitest slice** proving one proposal flow through real `invoke` or a harness fake.
- A **duplicate-test audit** showing e2e + unit tests rarely re-encode the same bytes-level fixtures without adding distinct assertions.
