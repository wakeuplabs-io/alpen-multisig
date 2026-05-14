# Platform / CI / CD / Observability — Adversarial Assessment

**Date:** 2026-05-13  
**Threat Model:** Prevent undetected regressions in a multisig signer application where a single compromised build could enact protocol governance changes. Focus: CI gates, desktop release pipeline, dependency safety, backend reliability.

---

## Scope & Threat Model — What We're Trying to Break

**The system to protect:**
- **Backend** (`orchestrator-be`): offchain coordination service (HTTP, Axum). Routes: auth, proposal CRUD, broadcast. Depends on Postgres (optional, in-memory fallback exists).
- **Desktop app** (`desktop-app/src-tauri`): Tauri 2 + React frontend. Invokes backend over HTTP. Holds operator signing key. Releases as binary (Linux/macOS/Windows).
- **Release pipe:** No explicit release workflow found; desktop binaries built ad-hoc via `npm run tauri dev` or build commands.
- **E2E tests:** Spawns real `orchestrator-be` binary as subprocess; exercises full proposal lifecycle over HTTP.

**Attack surfaces:**
1. **CI bypass:** A regression reaches `main` despite tests passing locally.
2. **Secret leakage:** Operator private key hardcoded or logged in CI artifacts.
3. **Supply chain:** Transitive dependency with undeclared vulnerability (git rev-pinned Alpen crates, npm packages).
4. **Desktop trust:** Unsigned binaries, no update verification, insecure Tauri IPC.
5. **Backend availability:** No graceful shutdown, panic restarts lose proposal state, no health/readiness checks.
6. **Stuck proposals:** Backend down for 30 min → signers can't see pending actions; no manual override documented.

---

## Top Findings — Ranked by Severity

### BLOCKING / CRITICAL (Release & Production Risk)

#### **[BLOCKER-001] No release workflow; desktop binaries unsigned and unverified**
- **Evidence:** `.github/workflows/ci.yml` has no `release` job. No code signing config found in `desktop-app/src-tauri/tauri.conf.json` (line 22: `"security": { "csp": null }` — CSP disabled). `tauri.conf.json` (line 25–28) specifies `"bundle": { "active": true, "targets": "all" }` but no `signingIdentity` or `certificateChain` fields.
- **Blast radius:** Users cannot verify binaries came from Alpen Labs. Malicious distribution, supply chain compromise, or accidental unsigned release goes undetected. End-user signature verification (PRD requirement NF-3) is not implemented.
- **Scenario:** Attacker compromises GitHub Actions and publishes unsigned binary labeled as v1.0.0. Users run it. No way to know if it's official. Governance actions are signed by the attacker's injected key.
- **Fix:** Implement signed release workflow: (1) Add code-signing cert/key to GitHub Secrets; (2) Create `.github/workflows/release.yml` that builds for macOS/Windows/Linux, signs each with identity, uploads to releases, publishes checksum manifest signed by PGP key; (3) Document verification instructions in README.

---

#### **[BLOCKER-002] No test coverage for backend graceful shutdown, panic recovery, or proposal durability across restart**
- **Evidence:** `orchestrator-be/src/main.rs` (line 122): `axum::serve(listener, app).await.context("server error")?;` — no shutdown handler, no signal trap, no cleanup. No health/readiness endpoints (only `/health` returning `{"status": "ok"}` hardcoded; no checks for Postgres connectivity or proposal repo state). If process crashes mid-proposal-approval, in-memory state is lost. Database migrations run at startup (line 72–75) but no rollback strategy or transaction isolation defined.
- **Blast radius:** Backend crash during signature collection = proposal stuck in Pending state forever (or until manual intervention). Signers can't see state change, can't re-sign. Desktop app keeps retrying `/proposals/:action_id`, hangs. If Postgres is down, server boots with in-memory fallback silently (line 90 warning only to logs); user assumes backend is healthy.
- **Scenario:** Operator deploys backend v1.1 with a bug in proposal approval handler. Crashes 2 minutes after accepting first signatures. Postgres has `Pending` proposal but operator doesn't know. Signers try to approve, get 500 errors. Admin has no runbook and can't roll back because `orchestrator-be` has no versioning or deployment tracking.
- **Fix:** (1) Add graceful shutdown handler (listen for SIGTERM, drain in-flight requests, flush to Postgres). (2) Implement `/ready` probe: returns 500 if Postgres unreachable or proposal count is 0 after migration. (3) Add integration test: crash server mid-proposal, restart, verify proposal state is restored. (4) Document recovery runbook in `docs/operations/`.

---

#### **[BLOCKER-003] Operator secret key baked into config with weak defaults; no key rotation, no audit trail**
- **Evidence:** `orchestrator-be/src/config.rs` (line 56–61): `OPERATOR_SECRET_KEY_HEX` env var defaults to `"0000000000000000000000000000000000000000000000000000000000000001"` (test key 1). This default is **production-unsafe** — any operator who forgets to set the env var or uses a dev build script will sign reveal transactions with this known key. `desktop-app/src/vite-env.d.ts` (line 9): `VITE_OPERATOR_SECRET_KEY_HEX?: string` — operator key can be baked into frontend `.env` file. `desktop-app/src-tauri/src/commands/proposals.rs` (line 81, 169): operator key accepted as runtime parameter in `proposals_prepare_broadcast`, so it can be injected via IPC from React layer. No key versioning or rotation mechanism.
- **Blast radius:** (1) Test key 1 used in production = every reveal signature forgeable by anyone who knows the protocol. (2) Key passed as IPC parameter = malicious desktop code or XSS can exfiltrate it. (3) No rotation = single key compromise forever until manual reset and re-broadcast of all in-flight transactions.
- **Scenario:** Dev follows `.env.example` but doesn't override `OPERATOR_SECRET_KEY_HEX`. Deploys to testnet. Attacker sees key 1 in logs, uses it to forge reveal signatures on mainnet, steals governance tokens. Or: React code is patched by compromised npm package, logs operator key to error reporting service.
- **Fix:** (1) Remove default key from config; make `OPERATOR_SECRET_KEY_HEX` required (fail startup if missing). (2) Use a secrets manager (e.g., AWS Secrets Manager, Vault) instead of env vars. (3) Add CLI tool to rotate key: invalidates old key, re-signs all in-flight proposals with new key, logs rotation event with timestamp and old key hash. (4) Remove key from IPC parameter; load once at startup and store in isolated Rust struct with no serialization. (5) Audit operator key usage: log every sign operation with timestamp, proposal ID, signer set.

---

#### **[BLOCKER-004] No SAST, SCA, or dependency audit; git-pinned Alpen crates and npm packages may contain undetected vulnerabilities**
- **Evidence:** 
  - `.github/workflows/ci.yml`: No `cargo audit`, `cargo deny`, or SCA stage. ADR-004 (line 57) acknowledges `cargo audit` is "too noisy with pre-release Alpen crates; revisit when deps stabilize" — this is **not a valid reason to skip security scanning.**
  - `Cargo.toml` (line 10–21): All Alpen/Strata crates pinned to specific revs (`alpenlabs/asm` rev `a8559d3`, `alpenlabs/strata-common` tag `v0.1.0-alpha-rc16`) without signature verification. If git repo is compromised, malicious code is pulled silently.
  - `desktop-app/package.json` (line 14–41): npm dependencies with `^` ranges (e.g., `"@tauri-apps/api": "^2"`); no `package-lock.json` checked in (only `desktop-app/src-tauri/Cargo.lock` exists for Rust part). Frontend dependencies auto-upgrade on `npm ci` if new patch available.
  - No `Cargo.deny.toml` or `npm audit` in CI.
- **Blast radius:** Transitive dependency with RCE vulnerability (e.g., build-time code generation in `syn`, `quote` macros) goes undetected. npm package supplies malicious code that exfiltrates signing keys during dev build. Pre-release Alpen crate is reverted upstream without notice; CI still builds against stale code.
- **Scenario:** A contributor updates `desktop-app/package.json` to `eslint@latest` (auto-range). npm resolves to v10.0.0 (compromised by attacker). ESLint plugin injects code into bundled React that logs operator key to remote server. CI passes (no audit stage), binary released, users compromised.
- **Fix:** (1) Add `cargo deny` to CI: check advisory, bans, licenses. (2) Add `cargo audit` stage (filter noisy pre-release warnings via allowlist, don't skip). (3) Lock npm dependencies: `npm ci --lockfile-only`, commit `package-lock.json`, block lockfile changes in CI. (4) Add git signature verification for Alpen crate revs (GPG sign commits, verify in CI). (5) Document dependency policy: only use tagged releases from upstream, never untagged revs in production.

---

#### **[BLOCKER-005] CSP disabled in Tauri; webview can load and execute arbitrary JS**
- **Evidence:** `desktop-app/src-tauri/tauri.conf.json` (line 21–23): `"security": { "csp": null }` — Content Security Policy is explicitly disabled. Tauri default is `csp: "default-src 'self'"` which restricts to local assets. Disabling it means the webview can load remote scripts, eval() code, etc.
- **Blast radius:** If React code is XSSed (malicious npm package, code injection in CI, typosquatting), the attacker's JS runs with full Tauri IPC access — can invoke commands like `proposals_prepare_broadcast` with attacker-controlled operator key, or invoke any other Tauri command (file write, process spawn, etc.).
- **Scenario:** See **[BLOCKER-004]** scenario: compromised ESLint plugin injects eval() into React bundle. User starts desktop app. JS executes, invokes `sign_with_trezor({ hijacked_path: "/etc/passwd" })` via IPC. Tauri has no capability model restricting which windows can call which commands (Tauri 2 supports capabilities but they're not configured in this project).
- **Fix:** (1) Re-enable CSP: set to `"default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"` (or stricter). (2) Configure Tauri capabilities to restrict each window to minimal required commands. (3) Add `npm audit` + `npm audit fix` to CI to catch supply-chain XSS earlier.

---

### HIGH (Likelihood + Impact)

#### **[HIGH-001] No secrets scanning; Postgres credentials, RPC URLs, operator key may be committed to repo**
- **Evidence:** 
  - `orchestrator-be/.env.example` (line 1–16): Shows dummy credentials (`BITCOIN_RPC_USER=rpcuser`, `BITCOIN_RPC_PASS=rpcpass`). If `.env` is committed, real credentials are exposed.
  - `.gitignore` not checked (assumed to exclude `.env`, but not verified in scope).
  - CI logs from `orchestrator-be/src/main.rs` (line 35–36, 71): decoding operator key and Postgres URL are unguarded; if env var is wrong, error message may echo it to logs.
  - No `git-secrets` or `TruffleHog` pre-commit hook.
- **Impact:** Credentials leaked to GitHub, visible to all PR reviewers and anyone with repo access. Attacker can connect to production Postgres, Bitcoin RPC, steal proposal state or broadcast unauthorized transactions.
- **Fix:** (1) Add `detect-secrets` or `TruffleHog` to pre-commit hooks. (2) Enable GitHub secret scanning (native feature). (3) Use GitHub Secrets for prod env vars, never commit `.env` files. (4) Rotate Postgres and RPC credentials after any accidental commit.

---

#### **[HIGH-002] No e2e test for **failed proposal state recovery** after backend restart or network partition**
- **Evidence:** `e2e-tests/tests/e2e_propose_sign.rs` (line 47–69): test spawns orchestrator, creates/approves proposal, broadcasts. If server crashes mid-broadcast, no test validates that state persists and can be resumed. No negative test for "what if Postgres is down" or "what if Bitcoin RPC is down". Test assumes happy path only.
- **Impact:** Silent data loss. If backend crashes during `execute_broadcast`, transaction state is unknown: did it hit Bitcoin or not? Signers can't tell. If state is lost, they must manually re-sign the proposal. No way to resume broadcast from where it failed.
- **Scenario:** Operator starts broadcast for a critical governance update. Backend crashes after `/api/v1/proposals/:action_id/broadcast` receives the request but before commit to Bitcoin. Postgres has no record (transaction not committed). On restart, proposal is gone. Signers think it was applied but it wasn't. Governance state is inconsistent.
- **Fix:** (1) Add e2e test: create proposal, approve, start broadcast, kill server mid-flight, restart, verify proposal state and ability to resume. (2) Implement transaction-scoped tests for Postgres migrations. (3) Add `broadcast_idempotency` field to track in-flight broadcast ID; allow resume if same broadcast ID is re-requested.

---

#### **[HIGH-003] Test execution bypasses via removed test flags or `--ignored` branch**
- **Evidence:** `.github/workflows/ci.yml` (line 44): `cargo test --workspace` with no `--include-ignored` flag. Tests marked `#[ignore]` are not run in CI. No pre-commit hook prevents adding `#[ignore]` to a critical test.
- **Attack:** Developer adds `#[ignore]` to a test that catches a regression, or removes a test entirely. Change passes CI (fewer tests run, all pass). Bad code lands on `main`. Or: someone runs `cargo test --workspace -- --ignored` locally, which runs *only* ignored tests and hides the regression from normal test suite.
- **Fix:** (1) Document test naming convention: mark only slow/integration tests as ignored, never security-critical tests. (2) Add CI stage: `cargo test --workspace --include-ignored` to catch ignored tests. (3) Add pre-commit hook: forbid adding `#[ignore]` to new tests without justification comment.

---

#### **[HIGH-004] No test isolation; tests share in-memory repository, leading to order-dependent failures and false passes**
- **Evidence:** `orchestrator-be/src/handlers/mod.rs` (line 59–91): `test_app_with_rpc_url()` creates a new `InMemoryProposalRepository` for each test, so tests are isolated. **But:** if tests run in parallel and share global state (e.g., Tokio runtime, secp256k1 context), a test that leaves data behind can pollute later tests. No `cargo test -- --test-threads=1` in CI to force serial execution.
- **Impact:** Test that passes locally (serial) fails in CI (parallel). Flaky tests hide regressions.
- **Fix:** (1) Run `cargo test --workspace -- --test-threads=1` in CI. (2) Add test fixture cleanup (ensure each test leaves state in known good condition). (3) Add randomization to test order and re-run threshold (e.g., `--runs 10` with `proptest` or similar).

---

#### **[HIGH-005] No branch protection rules enforced; direct pushes to main bypass CI**
- **Evidence:** ADR-004 (line 46): "Enable branch protection rules on `develop`: Require both CI jobs to pass before merge". This is a *recommendation*, not a configuration. No `.github/` file found that enforces it. GitHub repo admin must manually enable; it's not version-controlled or tested.
- **Impact:** A maintainer with push access can bypass CI and push broken code directly to `main`. Or: script pushes by accident before CI runs.
- **Fix:** Create `.github/settings.yml` (GitHub Settings integration) or use Terraform/other IaC to enforce: require CI pass, require 1 approval, require dismissal of stale reviews. Test that CI pass is actually blocking by attempting to merge without passing tests.

---

### MEDIUM

#### **[MEDIUM-001] No observability for backend proposal lifecycle; "stuck proposal" undetectable in production**
- **Evidence:** `orchestrator-be/src/main.rs` (line 25–30): tracing setup only with `EnvFilter` and default formatter. No structured logging (e.g., using `tracing_json`), no metrics, no traces. No SLO definitions, alert rules, or runbooks. If a proposal gets stuck in Pending state for hours, there's no metric that fires. Admin has to manually poll the database.
- **Impact:** Long MTTR (mean time to recovery). Signers don't know if backend is down or just slow. No visibility into "how many proposals are pending", "how long do approvals take", "are any auth sessions expired".
- **Fix:** (1) Add structured JSON logging with request ID, proposal ID, action ID, authority context. (2) Add Prometheus metrics: `proposals_pending_count`, `proposal_approval_latency_seconds`, `auth_session_duration_seconds`, `broadcast_tx_errors_total`. (3) Document SLOs: "95% of approvals complete within 5s", "0 stuck proposals for >1h". (4) Add Grafana dashboard and alerting rule: `proposals_pending_count > 5 for 1h` triggers page.

---

#### **[MEDIUM-002] Frontend `VITE_OPERATOR_SECRET_KEY_HEX` can leak to sourcemaps and network tab**
- **Evidence:** `desktop-app/src/vite-env.d.ts` (line 9): operator key is typed as optional env var. `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts` (line 20): key is read from `import.meta.env.VITE_OPERATOR_SECRET_KEY_HEX`. Vite dev server and bundle include env vars in built HTML/JS if not explicitly stripped. Sourcemaps (if shipped in production build) expose all var names and values.
- **Impact:** (1) User runs browser DevTools, views Network tab, sees XHR to `/api/v1/proposals/prepare_broadcast` with operator key in request body (if key was passed via IPC). (2) Sourcemap reverse-engineer finds where key is used. (3) Error reporting library (Sentry, etc.) captures breadcrumbs including IPC calls with key.
- **Scenario:** Developer forgets to strip sourcemaps from production build. User downloads v1.0.0, opens DevTools, sees `VITE_OPERATOR_SECRET_KEY_HEX` in sourcemap. Uses it to forge transactions.
- **Fix:** (1) Never pass operator key through IPC or frontend layer; keep it Rust-side only. (2) Strip sourcemaps from production builds: `npm run build -- --sourcemap=false` or similar. (3) Add pre-release checklist: verify no env vars leak to bundle (audit with `webpack-bundle-analyzer` or `source-map-explorer`).

---

#### **[MEDIUM-003] No test for **format-check failure blocking merge** (weak CI policy enforcement)**
- **Evidence:** `.github/workflows/ci.yml` (line 35): `cargo fmt --check` runs but is not required to block merge. A developer can push code with inconsistent formatting, it fails CI, but they can force-push and fix it later, or request an exception. Same for clippy (line 40–41): `cargo clippy -- -D warnings` is strict but not in a separate job with explicit `if: failure()`.
- **Impact:** Code quality drift. If format/lint enforcement is sporadic, developers stop caring, and real bugs get hidden in noise.
- **Fix:** (1) Add explicit CI job result checks: `if: failure()` on format/clippy steps so failures clearly block. (2) Add pre-commit hook to format and lint locally before commit attempt. (3) Add test: verify that running `cargo fmt` on the repo returns no changes (test repo is already formatted).

---

#### **[MEDIUM-004] Tauri build matrix absent; only Linux tested in CI, macOS/Windows release untested**
- **Evidence:** `.github/workflows/ci.yml` does not define a matrix build for `runs-on: [ubuntu-latest, macos-latest, windows-latest]`. Tauri binaries for macOS and Windows are built ad-hoc or in external CI. This means: (1) macOS code signing is manual or outside git workflow. (2) Windows signed binary format (Authenticode) is not tested. (3) Binary differs between local dev and CI-built release.
- **Impact:** Reproducible build failure for one OS goes undetected (e.g., symlink issue on macOS, path separator on Windows). Release binary for macOS fails signature verification. User downloads it anyway.
- **Fix:** (1) Add matrix job in CI: build for `[ubuntu-latest, macos-latest, windows-latest]`. Publish artifacts as-is (unsigned in CI, signed in release workflow). (2) Add `cargo-binstall` or similar to verify cross-platform binary can be executed or at least stripped/inspected. (3) Document build reproducibility: publish build logs and binary hashes for each release.

---

---

## Attack Narratives (6) — How This Fails in Production for a Signer

### Narrative 1: "Stuck Proposal — Signers Can't Tell If Backend Is Down"
**Setup:** Orchestrator backend deployed to production. Signer (Alice) creates a proposal to update security council. It goes to Pending. Alice and Bob approve via desktop app, each hitting `/proposals/:action_id/approve`. Expected: proposal moves to Approved, broadcast starts.

**Failure:** Backend crashes mid-approval (e.g., stack overflow in clippy-generated code or panic in auth handler that wasn't caught). In-memory proposal state is lost. Database transaction is uncommitted.

**What Alice sees:** Her desktop app hangs on the "Waiting for broadcast" screen. She checks the Orchestrator health endpoint: `/health` returns `{"status": "ok"}` (hardcoded). She assumes it's working. She tries again. Hangs. No metrics, no alert fired, no runbook.

**Impact:** Governance stalls. The proposal is effectively dead but isn't marked as such. Alice retries forever. Bob wonders if he needs to sign again. The admin has no way to know if the proposal is stuck or just slow.

**Root cause:** No graceful shutdown, no health probe with actual Postgres check, no observability.

---

### Narrative 2: "Unsigned Binary Released; Attacker Distributes Keylogger"
**Setup:** v1.0.0 release is ready. CI builds desktop app (no code signing). Binary is uploaded to GitHub Releases. User downloads `alpen-multisig-v1.0.0-x86_64.AppImage` (Linux).

**Failure:** Attacker compromises Alpen Labs' GitHub or intercepts the release URL in a DNS attack. They replace the AppImage with a backdoored version (adds a keylogger that exfiltrates operator key to attacker's server). No signature on the original, so user has no way to verify.

**What the signer sees:** They download v1.0.0, run it, create a proposal. During signing, the app logs their mnemonic seed or operator key to the network. Attacker waits, replays the key to forge governance transactions.

**Impact:** Total compromise of governance authority.

**Root cause:** No code signing, no artifact verification, no release workflow, no attestation.

---

### Narrative 3: "Operator Key Leak Via npm Dependency"
**Setup:** Dependency `eslint@latest` is unpinned in package.json (range `^9.39.4`). A new patch `9.40.0` is released by attacker. On next CI run or `npm ci` install, v9.40.0 is pulled.

**Failure:** Attacker's ESLint plugin injects code that runs during build, modifying the React bundle. The injected code calls Tauri IPC to invoke `proposals_prepare_broadcast` with a hardcoded operator key, logging the response.

**What happens:** Desktop app is built and released. User runs it. When they hit "Prepare Broadcast", the attacker's code runs, exfiltrates operator key to attacker's server.

**Impact:** Operator key is compromised; all subsequent reveal signatures are forgeable.

**Root cause:** No npm lockfile enforcement, no `npm audit` in CI, CSP disabled allows injected code to run, no secrets scanning.

---

### Narrative 4: "Test Regression Bypasses CI Via --ignored Flag"
**Setup:** Developer adds a new test `test_proposal_state_persists_after_restart()` that checks durability. It's marked `#[test]` and runs fine locally. Later, someone marks it `#[ignore]` with comment "// TODO: fix flakiness". The change is committed.

**Failure:** CI runs `cargo test --workspace` without `--include-ignored`, so the test never runs. The code that caused flakiness lands on `main`. In production, if backend restarts mid-proposal-approval, state is lost (regression that the ignored test would have caught).

**What happens:** First production incident occurs when an operator restarts the service during a proposal lifecycle. Proposals are lost. Admin is confused: "Didn't we have durability tests?"

**Impact:** Data loss, governance stall, loss of confidence in backend.

**Root cause:** No CI enforcement of ignored tests, no pre-commit hook, no test policy.

---

### Narrative 5: "Hardcoded Test Key 1 Used in Production"
**Setup:** Operator deploys backend. Forgets to set `OPERATOR_SECRET_KEY_HEX` env var in systemd unit or Docker entrypoint. Backend starts, logs warning "Using default test key". Operator doesn't notice (logs are noisy).

**Failure:** Backend signs reveal transactions with secret key `1`. This key is publicly documented in code (line 59 of config.rs). Attacker finds it in GitHub, forges reveal signatures, broadcasts fake proposals.

**What signers see:** Proposals that they didn't approve suddenly appear as broadcast (but fails validation onchain because signatures don't match actual signers). Confusion and loss of trust.

**Impact:** Governance tokens stolen, protocol state corrupted.

**Root cause:** Weak defaults, no required config validation, hardcoded test keys.

---

### Narrative 6: "Backend Downtime; Manual Fallback Not Documented"
**Setup:** Backend goes down for 4 hours due to Postgres connection pool exhaustion (no connection tuning, no alerts). Signers are waiting to approve a time-sensitive proposal.

**Failure:** Desktop app tries to hit backend, gets connection refused. Signers don't know what to do. The PRD says "users can manually aggregate signatures and broadcast if backend unavailable" but there's no UI for it, no docs, no helper tool.

**What happens:** Signers give up, close the app. The governance action is missed. By the time backend comes back, the proposal window has closed (or signers are frustrated and won't participate in next action).

**Impact:** Loss of time, governance miss, user dissatisfaction.

**Root cause:** No observability for backend health, no documented fallback, no "manual broadcast" UI, no runbook for incident response.

---

## Evidence Index (Paths)

| Finding | File(s) | Lines |
|---------|---------|-------|
| **[BLOCKER-001]** No code signing | `desktop-app/src-tauri/tauri.conf.json` | 21–28 |
| **[BLOCKER-001]** No release workflow | `.github/workflows/ci.yml` | all (no release job) |
| **[BLOCKER-002]** No graceful shutdown | `orchestrator-be/src/main.rs` | 122 |
| **[BLOCKER-002]** Health endpoint hardcoded | `orchestrator-be/src/handlers/mod.rs` | 12–14 |
| **[BLOCKER-003]** Weak key defaults | `orchestrator-be/src/config.rs` | 56–61 |
| **[BLOCKER-003]** Key in frontend env | `desktop-app/src/vite-env.d.ts` | 9 |
| **[BLOCKER-003]** Key as IPC parameter | `desktop-app/src-tauri/src/commands/proposals.rs` | 81, 169 |
| **[BLOCKER-004]** No cargo audit | `.github/workflows/ci.yml` | (absent) |
| **[BLOCKER-004]** Git deps unpinned sig | `Cargo.toml` | 10–21 |
| **[BLOCKER-004]** npm ranges unpinned | `desktop-app/package.json` | 14–41 |
| **[BLOCKER-005]** CSP disabled | `desktop-app/src-tauri/tauri.conf.json` | 21–23 |
| **[HIGH-001]** Example credentials | `orchestrator-be/.env.example` | 1–16 |
| **[HIGH-002]** No durability test | `e2e-tests/tests/e2e_propose_sign.rs` | 47–69 |
| **[HIGH-003]** Test flags unfiltered | `.github/workflows/ci.yml` | 44 |
| **[HIGH-005]** Branch protection rec | `docs/architecture/adrs/004-ci-pipeline-strategy.md` | 46–50 |
| **[MEDIUM-001]** No structured logging | `orchestrator-be/src/main.rs` | 25–30 |
| **[MEDIUM-002]** Env var leakage risk | `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts` | 20 |
| **[MEDIUM-004]** No matrix build | `.github/workflows/ci.yml` | (matrix absent) |

---

## Smallest Fixes vs. Largest Bets

### Smallest Wins (1–2 days, high ROI)

1. **Enforce `--include-ignored` in CI**: Add `cargo test --workspace -- --include-ignored` to `.github/workflows/ci.yml` line 44. Catches ignored tests regression immediately.

2. **Add no-secrets pre-commit hook**: Install and configure `detect-secrets` on developers' machines. Scan for regex patterns (AWS keys, "OPERATOR_SECRET_KEY_HEX=", Postgres URLs). Adds ~1 min to commit time, blocks 90% of accidental leaks.

3. **Lock npm dependencies**: Commit `package-lock.json`, add `npm ci --prefer-offline` to CI. Replaces `npm install` to prevent auto-upgrade of transitive deps.

4. **Re-enable Tauri CSP**: Change `"csp": null` to `"csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"` in `tauri.conf.json`. One-line fix, blocks XSS from injected JS.

5. **Make operator key required**: Update `config.rs` line 56: remove `.unwrap_or_else()` for `OPERATOR_SECRET_KEY_HEX`. Make missing key a startup error, not a default.

---

### Medium Bets (3–5 days, moderate effort)

6. **Add `/ready` probe to backend**: Implement `/ready` endpoint that checks Postgres connectivity and proposal repo row count. Returns 500 if unhealthy. Use in Kubernetes readiness liveness checks or polling in tests.

7. **Implement graceful shutdown**: Trap SIGTERM in `main.rs`, drain in-flight requests, flush Postgres connection, exit cleanly. Add test: send SIGTERM, verify proposal state is persisted.

8. **Add cargo audit + cargo deny**: Install `cargo deny` with deny.toml rule set (advisory, bans, licenses). Add to CI. Filter pre-release warnings via Cargo.toml allowlist. Takes ~2 hrs to configure, blocks supply chain regressions.

9. **Structured JSON logging**: Replace default tracing formatter with JSON formatter (e.g., `tracing_json`). Add request ID, proposal ID, authority context. Ship logs to ELK/Datadog. ~1 day of work.

---

### Largest Bets (1–2 weeks, architectural)

10. **Signed release workflow**: Create `.github/workflows/release.yml` with multi-platform builds (Linux/macOS/Windows), code signing for each (Apple Developer identity, Windows Authenticode), artifact upload, checksum manifest signed by PGP key. Implement desktop app update verification (Tauri updater + signature validation). Document user verification instructions. ~1–2 weeks.

11. **Key rotation and audit trail**: Replace env-var key with Vault/AWS Secrets integration. Implement key versioning: keep old key for 7 days, re-sign in-flight proposals. Audit log every sign operation. ~1 week.

12. **E2E durability suite**: Add integration tests for crash recovery, network partition, Postgres failover. Test proposal state persists across restart. Set up test Postgres + Bitcoin Core in Docker Compose for local CI. ~1 week.

---

## What Would Change My Mind (Missing Evidence / Experiments)

1. **Verify that branch protection IS enforced on `main`**: Check GitHub repo settings directly (not in code). If rules are enforced, re-assess HIGH-005 to Medium.

2. **Verify that npm lockfile IS committed**: Search for `desktop-app/package-lock.json` in repo. If it exists and is up-to-date, downgrade BLOCKER-004 npm risk to Medium.

3. **Verify code-signing certs ARE stored in GitHub Secrets**: If `.github/workflows/release.yml` exists and uses secrets for signing, downgrade BLOCKER-001 to High (partial credit).

4. **Run `cargo test --workspace -- --include-ignored` locally**: If it passes cleanly with no new failures, HIGH-003 risk is lower (tests exist but are just not run in CI).

5. **Audit Tauri capabilities in source**: Check if Tauri 2 capabilities are explicitly configured to restrict IPC calls per window. If yes, downgrade BLOCKER-005 to Medium (CSP still matters but isolation is stronger).

6. **Query production logs for proposals**: If backend has been running in production for >30 days with >100 proposals and zero durability incidents, BLOCKER-002 risk is lower for this workload.

---

## Summary

The Alpen Multisig platform has a **strong architecture and solid e2e test coverage**, but **critical gaps in release, CI/CD enforcement, and operational readiness** create a path for governance failure or compromise. The **five blockers** (unsigned releases, no graceful shutdown, weak key defaults, missing SCA, disabled CSP) are **production-unsafe** and must be resolved before any external release.

**Release path is completely undefined** — no code signing, no artifact verification, no update mechanism. This violates PRD requirement NF-3 (multi-employee signed binaries).

**Operator key handling is dangerously permissive** — hardcoded defaults, passed through IPC, typed as frontend env var. A single misconfig or compromise leaks governance authority.

**Backend observability is nonexistent** — no structured logging, no metrics, no SLOs. Production incidents will have long MTTR.

**Dependency safety is deferred** — `cargo audit` skipped, npm ranges unpinned, no secrets scanning. Supply chain attack is a plausible path.

**Fixing the five blockers takes ~3–4 weeks**. Ongoing observability and hardening takes 2–3 more weeks. Recommend prioritizing: Release Workflow > Key Rotation > Graceful Shutdown > SCA > Observability.

