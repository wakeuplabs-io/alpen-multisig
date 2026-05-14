# Platform / CI / CD / Observability — Adversarial Assessment (re-audit)

**Date:** 2026-05-14  
**Lens:** Platform architect (pipeline completeness, secrets, deployment path, observability, supply chain)  
**Method:** Read-only review of repository state; evidence paths cite current files.

---

## Scope & threat model

**What we are trying to break:**

- **CI as safety net:** Malicious or mistaken changes merge to `develop` / `main` without catching regressions, insecure deps, or broken builds.
- **Supply chain:** Compromised git-pinned Alpen crates or npm workspace packages ship in release artifacts; no advisory or deny gates catch it.
- **Operational blindness:** Production incidents cannot be triaged — no SLOs, no structured redaction policy, no release provenance.
- **Secret hygiene:** Operator keys, RPC passwords, or session material leak via defaults, logs, artifacts, or misconfigured desktop/web bundles.
- **Deployability:** No documented or automated path from green CI to signed, verifiable desktop binaries; backend has no graceful shutdown or real readiness.

**In scope (evidence-bound):** `.github/workflows/`, `docs/architecture/adrs/004-ci-pipeline-strategy.md`, root `Cargo.toml`, npm workspace layout, `orchestrator-be` bootstrap (`main.rs`, `config.rs`), `desktop-app` security-related config, `tauri.conf.json`.

**Out of scope:** Alpen protocol correctness (SPS); deep frontend UX (covered in other axes).

---

## Top findings (ranked by severity)

### BLOCKER: F1 — Documented security scanning explicitly deferred; CI has no `cargo audit` / `cargo deny` / npm advisory gate

**Risk:** Known Rust or npm vulnerabilities ship unchecked; ADR rationalizes skipping `cargo audit` for “noisy” pre-release deps rather than gating with allowlists.

**Evidence:** `docs/architecture/adrs/004-ci-pipeline-strategy.md` (Alternatives: “`cargo audit` for security — Too noisy with pre-release Alpen crates; revisit when deps stabilize”); `.github/workflows/ci.yml` runs fmt, clippy, test, frontend lint/build only — no advisory or license/deny stage.

**Failure scenario:** A transitive crate with a published RCE advisory is picked up on the next lockfile refresh; PR merges because CI never queries `RustSec` or npm advisories.

**Smallest fix:** Add `cargo deny` + `cargo audit` with documented allowlist for known-noisy Alpen paths; add `npm audit --production` (or `pnpm audit` equivalent) in the frontend job; fail on high/critical unless waived in a checked-in config with expiry.

**Largest bet:** Centralized SBOM generation per release, signed attestations, and policy-as-code in CI (Org-level).

**Disconfirming probe:** Search CI YAML for `audit` / `deny` — absent.

---

### BLOCKER: F2 — No release / signing / provenance workflow; desktop trust model unresolved at platform layer

**Risk:** End users cannot distinguish official binaries from tampered builds; governance UI runs inside an unsigned or unverifiable distribution channel.

**Evidence:** `.github/workflows/ci.yml` — only `pull_request` / `push` to `develop` and `main`; no `release`, artifact upload, code signing, or checksum publication. `desktop-app/src-tauri/tauri.conf.json` — `"bundle": { "active": true, "targets": "all" }` but no signing identities configured in-repo; `security.csp` is `null` (amplifies webview trust issues — see F3).

**Failure scenario:** Attacker publishes a forked binary; documentation points users to “build from source” only — no release hygiene, no verification story aligned with multisig risk.

**Smallest fix:** Add `release.yml`: build matrix, store artifacts, publish SHA256 manifest, integrate platform code-signing secrets; document verify steps in a single operations-facing doc (user-requested).

**Largest bet:** Full update channel (Tauri updater), binary transparency log, and org key management (HSM).

---

### CRITICAL: F3 — Content Security Policy disabled in Tauri; CI does not exercise `tauri build` / bundle

**Risk:** Webview can load or execute broader content than a stricter CSP would allow; supply-chain XSS or misloaded remote assets gain higher blast radius against IPC bridges.

**Evidence:** `desktop-app/src-tauri/tauri.conf.json` — `"security": { "csp": null }`. CI `frontend` job runs Vite build only (`.github/workflows/ci.yml`); no step builds the Tauri bundle that will actually ship.

**Failure scenario:** Compromised frontend dependency injects script; CSP is not a backstop; capabilities not reviewed here — platform review treats this as release-blocker class.

**Smallest fix:** Set a non-null CSP appropriate for local asset loading; add a CI job or conditional step for `tauri build` on Linux (accept longer runtime) or `cargo build -p` Tauri crate with feature parity checks.

**Largest bet:** Capabilities-based IPC hardening per ADR-005 desktop layering follow-through.

---

### HIGH: F4 — npm cache key in CI points at non-existent `desktop-app/package-lock.json`; lockfile lives at workspace root

**Risk:** Cache restore is ineffective (constant cold `npm ci`), lengthening CI and encouraging “fix forward” without noticing dependency drift; misleading maintainer mental model.

**Evidence:** `.github/workflows/ci.yml` — `key: node-modules-${{ hashFiles('desktop-app/package-lock.json') }}`; repository has `package-lock.json` at workspace root (workspace includes `desktop-app`), not under `desktop-app/`.

**Failure scenario:** Repeated CI noise and skipped cache benefits; harder to reason about reproducible installs.

**Smallest fix:** Change `hashFiles` to `package-lock.json` at repo root (or `${{ hashFiles('**/package-lock.json') }}` with documented primary).

**Largest bet:** Dedicated `pnpm` / `yarn` policy with immutable installs and deterministic CI images.

---

### HIGH: F5 — Backend observability is `TraceLayer` + default env filter; no readiness beyond static `/health`; no graceful shutdown

**Risk:** Orchestrators cannot tell “serving traffic but unhealthy” (e.g., Postgres chosen at boot, later broken); deploys cause abrupt connection drops; incidents lack request/authority/action correlation guarantees.

**Evidence:** `orchestrator-be/src/main.rs` — `TraceLayer::new_for_http()`, `EnvFilter` default `info`, `axum::serve(...).await` with no shutdown signal handling; `orchestrator-be/src/handlers/mod.rs` — `/health` returns fixed `{ "status": "ok" }` with no datastore or RPC checks. `DATABASE_URL` unset logs warning and uses in-memory repo (`main.rs` ~90) — “healthy” from `/health` while non-durable.

**Failure scenario:** Kubernetes marks pod ready; signers write proposals that vanish on restart; logs lack structured fields required for paging on authority/action_id.

**Smallest fix:** Add `/ready` that checks repo connectivity (and optionally RPC); structured fields via tracing spans; graceful shutdown with `with_graceful_shutdown`.

**Largest bet:** OpenTelemetry export, SLOs on approval latency and broadcast success, burn-rate alerts.

---

### MEDIUM: F6 — CORS `allow_origin(Any)` on coordination API

**Risk:** Browser-hosted attack pages can invoke the API from user networks where the backend is exposed; combines badly with any cookie-like future auth (currently bearer). Lower severity if production only binds to localhost/VPN — still defaults-in-code smell.

**Evidence:** `orchestrator-be/src/main.rs` — `CorsLayer::new()...allow_origin(Any)`.

**Smallest fix:** Configurable allowed origins; default deny in production profile.

**Largest bet:** Mutual TLS or OAuth-bound browser clients with strict origin policy.

---

## Attack narratives

1. **The “green CI, bad dependency” merge:** A contributor bumps a transitive crate; `cargo audit` is policy-deferred per ADR-004. CI stays green; advisory is public for 48h before exploitation. **Outcome:** attacker triggers reachable panic or logic bug in coordination path.

2. **The “unsigned release” social engineering:** No `release.yml`; users download binaries from chat links. **Outcome:** fake desktop app exfiltrates operator/session material; no org-signed checksum to contradict the package.

3. **The “healthy but volatile” deploy:** `/health` OK while `DATABASE_URL` was accidentally unset. **Outcome:** proposals stored in RAM; rolling restart loses state; operators discover data loss post-incident with no metric alerting.

---

## Evidence index (paths)

| Area | Path |
|------|------|
| CI workflow | `.github/workflows/ci.yml` |
| ADR CI policy (audit deferral) | `docs/architecture/adrs/004-ci-pipeline-strategy.md` |
| Workspace / git deps | `Cargo.toml` |
| Backend boot / CORS / tracing / serve | `orchestrator-be/src/main.rs` |
| Health route | `orchestrator-be/src/handlers/mod.rs` |
| Config defaults (secrets surface) | `orchestrator-be/src/config.rs` |
| Root npm lockfile | `package-lock.json` |
| Desktop CSP / bundle | `desktop-app/src-tauri/tauri.conf.json` |

---

## Smallest fixes vs largest bets

| Finding | Smallest fix | Largest bet |
|---------|--------------|-------------|
| F1 | Allowlisted `cargo deny` + `cargo audit`; `npm audit` in CI | Org SBOM + signed attestations per release |
| F2 | `release.yml` + checksum doc | Update channel + transparency log |
| F3 | CSP non-null; optional `tauri build` CI | Full capabilities review + webview hardening |
| F4 | Fix `hashFiles` path | Immutable install policy + image pins |
| F5 | `/ready` + graceful shutdown | OTel + SLO alerting |
| F6 | Configurable CORS | mTLS for exposed deployments |

---

## What would change my mind

- **F1:** Evidence that another automated pipeline (not in-repo) runs `cargo audit` / `cargo deny` on every merge with enforced waiver process — cite config with owner.
- **F2:** Published release process outside GitHub that still produces verifiable artifacts — cite how users validate (signing keys, checksums).
- **F5:** Demonstration that orchestration platform only uses `/ready` and deployment blocks on DB connectivity failures — with metrics dashboards linked.

---

## Conclusion

**Platform posture:** PR CI is solid for **lint/build/test** on Linux, but **external validity for production** is weak: security scanning is explicitly postponed in ADR-004, **no release/signing/provenance** automation exists, **observability and readiness** are minimal, and **desktop CSP** is disabled. The npm **cache key path bug** is a concrete CI hygiene defect. Treat **F1–F3** as merge/release blockers for anything marketed as multisig production software until mitigated or superseded by an documented external control.
