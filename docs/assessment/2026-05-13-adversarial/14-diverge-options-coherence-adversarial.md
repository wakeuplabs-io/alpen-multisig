# Diverge / Options Coherence — Adversarial Assessment

## Scope & threat model (what we're trying to break)

This assessment audits the Alpen Multisig codebase for **implicit divergence** — places where competing design approaches coexist without documented trade-offs, where the chosen path forecloses unstated alternatives, or where feature flags / multiple implementations are tested unequally. The repo has a **narrow domain** (5 fixed authorities, signature collection, proposal lifecycle) but this constraint actually makes divergence more visible: when you can't justify multiple approaches via feature flags, you must justify them via ADRs.

**Key threat vectors:**
1. **Error handling divergence** — two or more error patterns in the codebase with no documented reason
2. **RPC strategy fragmentation** — ASM state fetching done differently (with/without mocking) in different layers
3. **Authority handling incoherence** — 5 authorities defined, but only StrataAdmin tested in new code paths
4. **Signing path multiplicity** — at least three ways to derive/sign (software, Trezor HW, local sighash), with unequal test coverage
5. **Missing ADRs for implicit decisions** — choices that close off alternatives without documentation

---

## Top findings (ranked) — Blocking/High | Medium | Low

### BLOCKING: Dual error types + conversion gap (`backend vs. desktop`) 

| Category | Backend | Desktop |
|----------|---------|---------|
| **Error enum** | `AppError` (thiserror + Axum `IntoResponse`) | String (all endpoints return `Result<T, String>`) |
| **Location** | `orchestrator-be/src/error.rs` | Every application module (`application/*.rs` lines 53, 137, 227, etc.) |
| **Consequence** | Structured error → HTTP 401/404/409/500 | Flat string → all errors look the same to Tauri |
| **Testing** | HTTP status code validation possible | Requires string parsing to assert error kind |

**Adversarial claim:** The desktop app's global `Result<T, String>` pattern is a maintenance footgun. Errors from three different subsystems (orchestrator client, auth crypto, hardware wallet) all serialize as untyped strings, making it impossible to:
- Distinguish "user not authorized" from "device disconnected" from "malformed input" in the UI without string matching
- Write typed error tests (must regex against error strings)
- Add structured retry logic (you can't pattern-match on error kind)

The backend uses the correct pattern (`thiserror` enum + Axum conversion), proving the team knows better. **Why wasn't this applied to Tauri?** No ADR explaining the divergence.

**Maintenance nightmare scenario:** A signer's Trezor disconnects mid-operation and returns `"Err("USB connection lost")"`. The UI receives this as `Result::Err("USB connection lost")`. Six months later, a product change demands: "Show a 'reconnect device' button if the error contains USB, HID, or timeout keywords." Three engineers must now grep the codebase for every possible device error string. Meanwhile, the backend team never faces this because they pattern-match on `AppError::Internal(_)` → circuit-break gracefully.

**Recommendation:** Extract a `DesktopError` enum (parallel to `AppError`). Keep Tauri serialization simple (flatten to JSON), but force all result-returning functions to use `Result<T, DesktopError>` internally. This is **not optional** — it's a blocking refactor before the app scales beyond StrataAdmin.

---

### HIGH: ASM role membership coupling + incomplete coverage

| Layer | Location | Coverage |
|-------|----------|----------|
| **Backend** | `orchestrator-be/src/infrastructure/asm_role_membership.rs` | `fn is_signer_member_for_authority()` + `fn ordered_keys_for_authority()` + `fn last_seqno_for_authority()` + `fn threshold_for_authority()` + **mock_* helpers** — tests all 5 authorities |
| **Desktop** | `desktop-app/src-tauri/src/infrastructure/asm_role_membership.rs` | `fn ordered_keys_for_authority()` **ONLY** — no signer membership check, no last_seqno, no threshold, **only StrataAdmin mocked** |

**Adversarial claim:** The backend and desktop apps **re-implement overlapping ASM state queries but with different completeness**. The backend:
- Fetches signer membership (line 12)
- Fetches ordered keys (line 40)
- Fetches last seqno (line 59)
- Fetches threshold (line 84)
- All have mock fallbacks that support **any authority** (lines 125–170)

The desktop app:
- Fetches ordered keys only (line 13)
- No signer membership check
- No last_seqno (this is handled at Tauri command level instead — less encapsulation)
- No threshold fetch (relies on orchestrator to provide it)
- Mocks **hardcoded to StrataAdmin only** (line 30: `Authority::StrataAdmin =>`)

**Why is this a problem?**
1. The desktop app **cannot independently verify** whether a signer belongs to an authority — this authority validation is entirely delegated to the orchestrator. If the orchestrator is compromised or goes offline, the desktop app is blind.
2. The incomplete mock in the desktop app means **PayoutAdmin, SecurityCouncil, SequencerManager, and AlpenAdmin cannot be tested locally** in the desktop app. You must connect to a real ASM RPC to test anything other than StrataAdmin.
3. This creates **hidden coupling:** the desktop and backend have split the "fetch role membership" responsibility, but this split is **implicit** (no ADR explaining it). If the team later decides "desktop should validate authority offline," they will duplicate backend logic or create a shared library, both of which are expensive refactors.

**Test consequence:** `e2e-tests` runs both backend and desktop in-process (via Tauri's test harness), but the desktop's mock only recognizes StrataAdmin. This means **e2e tests for PayoutAdmin proposal flows will silently skip or fail** because `ordered_keys_for_authority()` will hit the real RPC, not the mock.

**Recommendation:** 
- Create an ADR explaining the authority validation boundary (backend validates, desktop trusts orchestrator, or vice versa).
- If desktop trusts orchestrator, remove all ASM RPC calls from desktop and assert this in tests.
- If desktop must work offline, implement full authority validation with **mocks that cover all 5 authorities**.

---

### HIGH: SSZ codec isolated to one file, but signing path uses direct codec

**Locations:**
- `desktop-app/src-tauri/src/infrastructure/action_codec.rs` (lines 1–6: "ONLY module that imports strata_asm_*")
- `desktop-app/src-tauri/src/infrastructure/signing.rs` (line 57: `let action = MultisigAction::from_ssz_bytes(...)`)

**Adversarial claim:** The team documented a codec abstraction layer:

> "This is the **only** module that imports `strata_asm_*` / `strata_crypto` crates. Everything else in the desktop application talks in domain types."

But `signing.rs` **directly imports and uses the SSZ codec** at line 10:
```rust
use strata_asm_txs_admin::actions::{MultisigAction, Sighash};
use ssz::Decode;
```

And at line 57, it calls:
```rust
let action = MultisigAction::from_ssz_bytes(&action_bytes)
```

**This violates the stated abstraction boundary.** The comment says "only action_codec.rs" but signing.rs imports strata_asm crates directly.

**Why does this matter?**
- If Alpen releases a breaking change in SSZ format or `MultisigAction`, you must update both `action_codec.rs` AND `signing.rs`.
- If you want to test signing without `strata_asm` crates (e.g., with a mock), the mock must be in two places.
- The stated invariant (single codec module) is **aspirational, not enforced**. Future contributors will not see the comment and will add strata_asm imports elsewhere.

**Recommendation:** Either:
1. Enforce the boundary: move all SSZ decode logic to `action_codec.rs`, have `signing.rs` call `codec::decode_action()` instead of `MultisigAction::from_ssz_bytes()`.
2. Or update the comment to be honest: "Strata-facing codec lives in action_codec.rs and signing.rs."

---

### HIGH: Configuration environment fallbacks create implicit defaults with no documented rationale

**Location:** `orchestrator-be/src/config.rs` lines 56–73

```rust
operator_secret_key_hex: std::env::var("OPERATOR_SECRET_KEY_HEX").unwrap_or_else(
    |_| {
        // Deterministic test key (32 bytes, value = 1); override in production.
        "0000...0001".to_string()
    },
),
bitcoin_magic_bytes_hex: std::env::var("BITCOIN_MAGIC_BYTES_HEX")
    .unwrap_or_else(|_| "414c504e".to_string()),
```

**Adversarial claim:** The backend uses **unvalidated defaults for cryptographic secrets and protocol parameters**:
- If `OPERATOR_SECRET_KEY_HEX` is not set, the operator key is hardcoded to `0x00...01` (the comment says "test key").
- If `BITCOIN_MAGIC_BYTES_HEX` is not set, it defaults to `"414c504e"` (ASCII "ALPN").

**Why is this dangerous?**
1. **Accidental deployment to production with test keys.** A developer running the server locally without a `.env` file gets the test key. If they commit a screenshot or log output, the test key is exposed. If the deploy script fails to set `OPERATOR_SECRET_KEY_HEX` at deploy time, the server runs in production with the test key. The broadcast logic will then use this test key to sign reveal transactions, invalidating signatures.
2. **Magic bytes hardcoding.** The config accepts magic bytes as a hex string, but there's no validation that it's 4 bytes, no validation that it matches the ASM deployment, and no error if someone typos the env var. The default "ALPN" is correct for the current deployment, but if the ASM switches magic bytes (e.g., for a testnet migration), the config must be updated. No warning if it's not.

**Missing ADR:** There should be an ADR explaining:
- Which settings are optional vs. mandatory
- Why certain settings have defaults
- What happens if a deploy-time setting is missing (fail fast vs. use default)

Currently, the code pattern "use env var, fall back to default" is inconsistent with the principle "explicit > implicit" and creates a silent footgun.

**Recommendation:** 
- Operator secret key should be mandatory (no default). Fail at startup if not set.
- Magic bytes could have a default if explicitly documented. Add a `Config::validate()` method that checks: (1) operator key is set, (2) magic bytes are 4 bytes, (3) magic bytes match expected value for this deployment (read from an ADR or a separate validation file).

---

### HIGH: Multiple error patterns in handlers (some delegate, some inline validation)

| Handler | Pattern | Location |
|---------|---------|----------|
| `auth.rs` | Thin: delegates to `application/auth.rs` | `handlers/auth.rs` |
| `proposals.rs` (create) | Inline validation: some checks before calling application | `handlers/proposals.rs` line ~50 |
| `proposals.rs` (approve) | Delegates to application | `handlers/proposals.rs` line ~80 |

**Adversarial claim:** The backend handlers are **inconsistently thin**. Some handlers (e.g., `auth.rs`) are purely passthroughs, while others (e.g., `proposals.rs`) do inline validation. No ADR documenting where the boundary is.

**Example from `handlers/proposals.rs`:**
```rust
// Inline check before calling application
if !is_valid_hex(&action_hex) {
    return Err(AppError::BadRequest("invalid action hex".to_string()));
}

// Then delegate to application for business logic
let proposal = application::create_update_action(
    &repo,
    session,
    seq_no,
    &action_hex,
    &sig,
    required_signatures,
).await?;
```

vs. `handlers/auth.rs`:
```rust
// Pure passthrough
let session = application::start_session(...).await?;
```

**Why does this matter?**
- **Testing is asymmetric.** Auth tests must run through the handler (to test handler thinness). Proposal tests can mock the application layer because some validation is in the application. This inconsistency makes it hard to know which tests belong in the handler layer vs. application layer.
- **Error handling is unclear.** Is `BadRequest("invalid hex")` a handler concern or application concern? If it's a handler concern, why isn't this validated in auth handlers too?
- **Future contributor confusion.** When adding a new endpoint, should validation go in the handler or the application? The codebase doesn't answer this.

**Recommendation:** ADR-006 (no ADR exists for this): Define handler responsibilities:
- Handlers ONLY: deserialize from HTTP, extract session, call application, serialize response.
- Application ONLY: validation, business logic, data access.
- No handler should call `is_valid_hex()` or equivalent — validation is an application concern.

---

### MEDIUM: Implicit network partition resilience assumption (backend vs. desktop coordination)

**Location:** Desktop app's `orchestrator_client.rs` + backend handlers

**Adversarial claim:** The desktop app's proposal flow is entirely dependent on the orchestrator backend being reachable. If the orchestrator is down or the network is partitioned:

```rust
// desktop-app/src-tauri/src/application/proposals.rs line 261
pub async fn create_update_action(
    client: &dyn OrchestratorClient,  // ← MUST be reachable
    ...
) -> Result<Proposal, String> {
    client.create_update_action(...).await  // ← Fails if orchestrator unreachable
}
```

But the PRD says (from context):
> "Manual fallback: Users can aggregate signatures and broadcast if backend unavailable."

**There is no code path for the manual fallback.** The desktop app:
1. Requires orchestrator to create a proposal
2. Requires orchestrator to get the last seqno (ADR-003 acknowledges this)
3. Requires orchestrator to broadcast

If all three are network-partitioned, the signer is locked out. The PRD's "manual fallback" exists only in concept.

**Why is this an implicit divergence?**
- **No feature flag.** There's no `#[cfg(feature = "offline-mode")]` for a manual aggregation flow.
- **No ADR.** There's no documented decision: "We prioritize online coordination (orchestrator always available)" vs. "We support manual fallback."
- **Test coverage gap.** The e2e tests do not cover orchestrator-down scenarios.

**Recommendation:** Write ADR-006 (Backend Uptime Dependency) explaining:
- Assumption: Orchestrator is always available. Signers do NOT have local broadcast capability.
- Why: Coordination is easier with a centralized backend. Removing it would require P2P sync, which adds complexity.
- Trade-off: Loss of offline resilience (acknowledged in PRD as "manual fallback," but not implemented).
- Future: If offline mode is required, we must extract signing/broadcast logic into a shared library and ship a CLI for manual aggregation.

---

### MEDIUM: Mock injection inconsistency (hard-coded `mock_*()` functions vs. dependency injection)

**Locations:**
- `orchestrator-be/src/infrastructure/asm_role_membership.rs` lines 125–170 (hard-coded mocks via URL prefix matching)
- `desktop-app/src-tauri/src/infrastructure/asm_role_membership.rs` lines 14–17 (same pattern)

**Pattern:**
```rust
pub async fn ordered_keys_for_authority(
    rpc_url: &str,
    authority: Authority,
) -> Result<Vec<String>, String> {
    if let Some(keys) = mock_ordered_keys(rpc_url, authority) {  // ← Hard-coded check
        return Ok(keys);
    }
    // Else hit real RPC
}

fn mock_ordered_keys(rpc_url: &str, authority: Authority) -> Option<Vec<String>> {
    if rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1") {
        // Return hardcoded mock data based on authority
    }
    None
}
```

**Adversarial claim:** The codebase uses **hard-coded mock detection** (checking if the URL contains "localhost") instead of dependency injection. This creates **tight coupling between test configuration and production code**:

1. **Mock is non-deterministic across environments.** If a developer accidentally connects to a localhost service in production (typo in env var), the code silently falls back to the hardcoded mock instead of failing loudly.
2. **Mock data is not versioned.** If a mock needs to change (e.g., add a 6th authority), you must edit the function in both backend and desktop.
3. **Test coverage is implicit.** There's no explicit list of "what does the mock support?" You have to read the function to know that only 5 authorities are mocked.

**Why is this an implicit divergence?**
- The backend's `memory_repo.rs` uses a proper trait (`ProposalRepository` implementing trait), allowing tests to inject a fake repository.
- But `asm_role_membership.rs` does NOT follow this pattern — it has hard-coded mock logic baked into the business function.
- No ADR explaining why the two strategies diverge.

**Recommendation:** Extract an `AsmStateRpc` trait:
```rust
pub trait AsmStateRpc {
    async fn get_status() -> Result<AnchorState, String>;
}

pub struct RealAsmStateRpc { rpc_url: String }
pub struct MockAsmStateRpc { /* mock data */ }
```

Inject this trait into `ordered_keys_for_authority()`. This allows tests to inject a mock without checking URL strings.

---

### MEDIUM: Authority enum duplication (backend vs. desktop)

**Locations:**
- `orchestrator-be/src/domain/authority.rs` (5 authorities: AlpenAdmin, StrataAdmin, SequencerManager, SecurityCouncil, PayoutAdmin)
- `desktop-app/src-tauri/src/domain/authority.rs` (same 5 authorities)
- `desktop-app/src-tauri/src/infrastructure/action_codec.rs` (conversion between domain and Strata Role)

**Adversarial claim:** The `Authority` enum is **redefined independently in two crates**, with no shared type. When Alpen adds a 6th authority or renames SequencerManager → SequenceMaintainer, you must update:
1. Backend domain
2. Desktop domain
3. Codec conversion function (`to_strata_role()`)
4. All pattern matches in both codebases

**There is no shared crate.** ADR-005 says: "If backend and desktop domain types diverge significantly, we may extract a shared `multisig-types` crate. Not needed yet."

**Why is this risky?**
- As soon as PayoutAdmin logic lands, you'll have handlers, application logic, domain validation, all per-authority. A missing authority in desktop will silently fail or compile-error obscurely.
- The codec conversion is **single-sourced** in desktop only. The backend never converts Authority → Strata Role — it uses Strata's Role directly in some places (e.g., `asm_role_membership.rs` line 24).

**Future problem:** If a second Alpen network (testnet, L2-ish variant) adds authorities, you'll have two options:
1. Create a new `Authority2` enum (bad naming, confusing)
2. Make `Authority` generic over network (complex)
3. Extract a shared library (correct, but currently not done)

**Recommendation:** Create ADR-006 (Authority Type Consistency):
- Decision: Extract a shared `alpen-types` crate with `Authority`, `ProposalStatus`, `Signature` types. Both backend and desktop consume from this crate.
- Rationale: Single source of truth for authority evolution. No divergence between backend and desktop enums.
- Timeline: Before PayoutAdmin (Slice 5) lands, extract this crate and migrate both codebases.

---

### MEDIUM: Sighash computation path (three different implementations)

| Implementation | Location | Purpose |
|---|---|---|
| **Direct upstream call** | `signing.rs` line 59: `action.compute_sighash(seqno)` | Desktop app's signing command |
| **Strata sighash library** | `strata_crypto` / `strata_asm` (upstream) | Referenced in comments |
| **Backend uses upstream directly** | `orchestrator-be` only touches it in tests | Backend does NOT recompute sighash — it trusts the desktop app's sighash |

**Adversarial claim:** There are **at least two ways to compute a sighash** in this codebase, and no test validates they're identical.

The `compute_sighash()` function in Strata is upstream. The desktop app calls it:
```rust
let sighash = action.compute_sighash(seqno);
```

But there's no test in the desktop app that:
1. Computes a sighash
2. Verifies it against a known-good hash (from Alpen's protocol spec)
3. Verifies two different implementations produce the same hash

The backend never computes sighashes — it receives them from the desktop. So if the desktop's sighash computation is wrong, the backend will accept the wrong signature.

**Why is this an implicit divergence?**
- **No ADR explaining the sighash boundary.** Is the desktop responsible for computing it? Can the backend double-check? Should they both compute and assert equality?
- **No test fixtures.** There's no golden sighash test (e.g., "for MultisigAction X and seqno Y, sighash must be Z"). If Alpen breaks sighash compatibility, the test suite won't catch it.

**Recommendation:** Add ADR-007 (Sighash Validation):
- Decision: Desktop computes sighash (it has Strata crates). Backend trusts it (it doesn't re-verify).
- Why: Avoid duplicate verification logic. Desktop is the source of truth.
- Guard: E2E tests must include a sighash golden-value test fixture. If upstream Strata changes `compute_sighash()` in a breaking way, the test fails loudly.

---

### LOW: Feature flag mismatch in desktop app Cargo.toml

**Location:** `desktop-app/src-tauri/Cargo.toml` line 46:
```toml
[features]
custom-protocol = ['tauri/custom-protocol']
```

**Observations:**
- Feature is defined but **never used** (`#[cfg(feature = "custom-protocol")]` doesn't appear in the code)
- Tauri documentation says custom-protocol is needed for desktop app security (prevents localhost XSS)
- No CI build checks if the feature is enabled
- No documentation explaining when to use it

**Adversarial claim:** This is a **vestigial feature flag** — likely copy-pasted from a Tauri template and never integrated. If someone reads "custom-protocol" in the Cargo.toml, they might assume it's active, but it's not.

**Recommendation:** Either:
1. Enable it by default (if it's necessary for security)
2. Delete it (if it's not needed)
3. Document it + add a CI step that builds with `--features custom-protocol`

---

### LOW: Incomplete error handling in broadcast path

**Location:** `orchestrator-be/src/infrastructure/broadcast_tx.rs` (not fully read, but referenced in multiple places)

**Observation:** The `broadcast_status` and `broadcast_error` fields exist in `Proposal`, but the logic for handling broadcast failures is not visible in the handlers. The config has `confirm_poll_interval_ms` and `confirm_timeout_ms`, suggesting polling, but no visible polling loop in the application layer.

**Recommendation:** Add clarity:
- ADR-008 (Broadcast Polling Strategy): Explain how broadcast confirmation works, where polling happens (backend? separate job?), what happens on timeout.

---

## Attack narratives (3–6): "How this fails in production / for a signer / for maintainers"

### Narrative 1: Signer uses desktop app, network partitions, no recovery path

**Scenario:**
1. Signer initiates proposal in desktop app.
2. Desktop app calls orchestrator: `POST /proposals/create`.
3. Network drops (or orchestrator is down).
4. Signer sees `Result::Err("Failed to create proposal: timeout")`.
5. Signer waits. Network recovers.
6. Signer retries. Desktop app has no "resume" or "check if proposal exists" flow because it's entirely stateless and online-dependent.
7. Signer broadcasts his signature manually (as the PRD suggests), but there's no UI for this. The manual fallback code does not exist.

**Consequence:** Signer thinks the proposal wasn't created, repeats it, or gives up. If the first proposal actually succeeded on the backend (and the network dropped before the response), the signer now has two identical proposals pending.

**Root cause:** ADR-006 (Backend Uptime Dependency) doesn't exist. The team decided online-only coordination but didn't document it. The PRD mentions "manual fallback" (SPS-65 allows offline aggregation) but the code path was never implemented.

---

### Narrative 2: Authority enum drift creates silent-fail for PayoutAdmin

**Scenario:**
1. PayoutAdmin feature (Slice 5) lands.
2. Backend is updated: Authority enum now has 6 variants. All handlers, validation, and tests updated.
3. Desktop app's **Authority enum is not updated** (someone forgets, or the change is silently missed in a rebase).
4. E2E tests run with a mock StrataAdmin signer (hard-coded in `asm_role_membership.rs` desktop mock).
5. Tests PASS because they never hit the PayoutAdmin code path.
6. On merge, the CI passes.
7. In production, a PayoutAdmin signer uses the desktop app. `authority_to_role()` in `action_codec.rs` has no arm for `Authority::PayoutAdmin`. **Compile error? No — the desktop app doesn't compile PayoutAdmin because it's not in the enum.**

**Consequence:** The desktop app won't compile if built after a backend Authority addition, but if the build is stale or cached, the binary will be out-of-sync with the backend. A PayoutAdmin signer can't use the app without recompiling.

**Root cause:** Authority enum is duplicated with no shared library. No CI check ensures both are in sync. The e2e test setup doesn't create PayoutAdmin sessions, so the gap isn't caught.

---

### Narrative 3: Test key leaks to production, signer trusts unsigned reveal transaction

**Scenario:**
1. A developer runs the backend locally: `cargo run -p orchestrator-be`.
2. `.env` file is missing (maybe git-ignored, developer just checked out the repo).
3. Backend starts with `OPERATOR_SECRET_KEY_HEX` defaulting to `0x00...01` (the test key).
4. Developer tests a proposal flow, broadcast succeeds, reveal txid appears in logs.
5. Developer screenshots the success screen and posts it to Slack for QA.
6. The post includes the logs, which show the operator key (`00...01`) and the reveal txid.
7. Attacker uses this txid to trace the proposal on Bitcoin testnet, sees the operator's "signature," realizes it's the test key, fakes signatures using the test key, and broadcasts a cancel transaction.
8. The reveal tx is orphaned. The proposal state machine breaks.

**Root cause:** No validation that the operator key is set. No deployment guard that prevents test keys from reaching production (the deploy process might fail to set the env var, and the server starts anyway with the test key).

---

### Narrative 4: Desktop app mock hardcoded to StrataAdmin, SequencerManager signer fails silently

**Scenario:**
1. E2E test for SequencerManager authority is written.
2. Test creates a SequencerManager session.
3. Test calls `ordered_keys_for_authority(rpc_url, Authority::SequencerManager)`.
4. The mock function in desktop checks `if rpc_url.contains("localhost")` → true. But then:
   ```rust
   fn mock_ordered_keys(...) -> Option<Vec<String>> {
       if rpc_url.contains("localhost") {
           match authority {
               Authority::StrataAdmin => Some(["key1", "key2"].to_vec()),
               _ => None,  // ← SequencerManager not mocked!
           }
       }
       None
   }
   ```
5. Returns `None` → falls through to real RPC call.
6. Real RPC is not running in the test environment.
7. Test times out or returns connection error.
8. Test flakes randomly (depends on whether a test RPC is available).
9. CI sometimes passes, sometimes fails, with no clear reason.

**Root cause:** Mock is incomplete (only StrataAdmin) but the incompleteness is implicit. No test explicitly checks "all 5 authorities are mocked."

---

### Narrative 5: SSZ codec boundary violation makes future Alpen breakage harder to fix

**Scenario:**
1. Alpen releases a new version of `strata_asm_txs_admin` with a breaking change to `MultisigAction::from_ssz_bytes()`.
2. The team updates `Cargo.toml` to the new version.
3. `cargo build` fails in desktop-app's `signing.rs` (line 57).
4. Also fails in `action_codec.rs` (because both import strata_asm).
5. Developer must now fix two places instead of one.
6. If the fix is non-trivial (e.g., new mandatory field in MultisigAction), the fixes in `signing.rs` and `action_codec.rs` might diverge.

**Root cause:** The comment says "only action_codec.rs imports strata_asm" but the comment is violated. No enforcement (no clippy lint, no compile-time check).

---

### Narrative 6: Error pattern divergence makes adding structured error recovery impossible

**Scenario:**
1. Product requirement: "If auth fails with 'session expired,' show user a login button. If HW wallet disconnects, show 'reconnect device.'"
2. Engineer tries to add typed error handling:
   ```rust
   match application::create_proposal(...).await {
       Ok(proposal) => { /* ... */ },
       Err(DesktopError::HwWalletDisconnected) => show_reconnect_button(),
       Err(DesktopError::SessionExpired) => show_login_button(),
       Err(_) => show_generic_error(),
   }
   ```
3. But application modules return `Result<T, String>`. There's no `DesktopError` enum.
4. Engineer must now build one, but existing application code doesn't use it. Refactoring is needed across 8+ modules.
5. The backend has this pattern (`AppError` enum) but it was never ported to desktop.
6. No ADR explains why the two apps use different error strategies.

**Root cause:** Error pattern divergence is not documented. Both approaches are valid, but they need to be intentional and justified.

---

## Evidence index (paths)

### Configuration & Defaults
- `orchestrator-be/src/config.rs` lines 25–75 — Environment variable fallbacks (test key default, magic bytes default)

### Error Handling Divergence
- `orchestrator-be/src/error.rs` — `AppError` enum with `IntoResponse` trait
- `desktop-app/src-tauri/src/application/proposals.rs` line 261 — `Result<Proposal, String>`
- `desktop-app/src-tauri/src/application/authentication.rs` line 53 — `Result<AuthChallenge, String>`
- `desktop-app/src-tauri/src/application/orchestrator_auth.rs` line 19 — `Result<...>, String>`

### Authority Enum Duplication
- `orchestrator-be/src/domain/authority.rs` lines 1–13
- `desktop-app/src-tauri/src/domain/authority.rs` lines 1–12
- `desktop-app/src-tauri/src/infrastructure/action_codec.rs` lines 99–115 — `to_strata_role()` conversion

### Asm Role Membership Coupling
- `orchestrator-be/src/infrastructure/asm_role_membership.rs` — Full authority coverage (lines 12, 40, 59, 84)
- `desktop-app/src-tauri/src/infrastructure/asm_role_membership.rs` — StrataAdmin-only mock (lines 28–32)
- Both use hard-coded mock via URL string matching (lines 125–170 in backend, lines 14–17 in desktop)

### SSZ Codec Boundary
- `desktop-app/src-tauri/src/infrastructure/action_codec.rs` lines 1–6 (comment asserting single import site)
- `desktop-app/src-tauri/src/infrastructure/signing.rs` lines 9–10 (violates comment: imports strata_asm directly)
- `desktop-app/src-tauri/src/infrastructure/signing.rs` line 57 (`MultisigAction::from_ssz_bytes()` call)

### Handler Pattern Inconsistency
- `orchestrator-be/src/handlers/proposals.rs` — Inline hex validation before delegating
- `orchestrator-be/src/handlers/auth.rs` — Pure passthrough handlers

### Sighash Computation
- `desktop-app/src-tauri/src/infrastructure/signing.rs` line 59 — `action.compute_sighash(seqno)` call
- No golden-value test comparing to protocol spec
- Backend never re-verifies sighash (e2e-tests do not validate sighash against a fixed value)

### Feature Flag Unused
- `desktop-app/src-tauri/Cargo.toml` line 46 — `custom-protocol` feature defined but not used in code

### Implicit Network Assumptions
- `desktop-app/src-tauri/src/application/proposals.rs` — No offline fallback
- Backend handlers assume orchestrator is the source of truth
- No ADR documenting this assumption

### ADR Gaps
- No ADR-006 (Backend Uptime Assumption)
- No ADR-007 (Sighash Validation Boundary)
- No ADR-008 (Broadcast Confirmation Strategy)
- No ADR on Authority Type Consistency (when to extract shared library)
- No ADR on Error Handling Strategy (why backend uses enum, desktop uses String)

---

## Smallest fixes vs largest bets (be explicit)

### Smallest Fixes (< 1 day, < 50 lines changed)

1. **Update config comments + add validation**
   - Add `Config::validate()` that asserts operator_key is set (or throw error if not set)
   - Update comments to reflect whether each setting is mandatory or optional
   - Lines: ~20 new, mostly comments

2. **Enable custom-protocol feature in Cargo.toml (or delete it)**
   - If needed: add `default = ["custom-protocol"]` to `[features]` and enable in CI
   - If not needed: remove it entirely
   - Lines: 1–2 changed

3. **Fix SSZ codec comment or enforce boundary**
   - Quick fix: update comment in `action_codec.rs` to say "mostly strata-facing imports live here"
   - Lines: 1 comment updated
   - Or: move `from_ssz_bytes()` call from `signing.rs` into `action_codec.rs`, create a wrapper function `decode_action_from_hex()` that both can call
   - Lines: ~15 new

### Medium Fixes (1–2 days, 100–300 lines)

4. **Extract `DesktopError` enum from String pattern**
   - Create `desktop-app/src-tauri/src/error.rs` with typed error variants
   - Update all `Result<T, String>` signatures to `Result<T, DesktopError>`
   - Add `impl From<SomeInternalError> for DesktopError`
   - Lines: ~100 new, ~200 changed (refactoring existing code)

5. **Create abstract `AsmStateRpc` trait, replace hard-coded mocks**
   - Define trait with `get_status()` method
   - Implement `RealAsmStateRpc` (current code)
   - Implement `MockAsmStateRpc` (extracted from hard-coded logic, now all 5 authorities)
   - Update both backend and desktop to inject the trait
   - Lines: ~150 new, ~80 changed

### Largest Bets (1–2 weeks, architectural)

6. **Extract shared `alpen-types` crate**
   - Create `crates/alpen-types/src/` with `Authority`, `ProposalStatus`, `Signature`, `ActionId` types
   - Make both backend and desktop consume from this crate
   - Remove duplicate enum definitions
   - Update imports across both codebases
   - Lines: ~100 new (shared), ~50 changed per codebase

7. **Add ADR-006 through ADR-008 + tests**
   - Write ADRs documenting backend uptime assumption, sighash validation, broadcast confirmation
   - Add sighash golden-value test fixtures (with known-good values from Alpen spec)
   - Add e2e tests for all 5 authorities (not just StrataAdmin)
   - Lines: ADRs ~150 lines, tests ~200 lines

8. **Implement offline/manual fallback path** (if required by product)
   - Extract signing and broadcast logic into a shared CLI library
   - Add commands to manually aggregate signatures and broadcast (for network-partition recovery)
   - Lines: ~500 new

---

## What would change my mind (missing evidence / experiments)

1. **ADR-006 (Backend Uptime) exists and explicitly rejects offline resilience** — "We accept the trade-off of online-only coordination because [reason]." If the team has already documented this, the divergence is intentional, not implicit. Status: **Not found.**

2. **Authority type unification already planned in a roadmap** — If there's a ticket or RFC to extract `alpen-types` crate before PayoutAdmin, this is acknowledged debt. Status: **Not found in docs/specs or ADRs.**

3. **Error handling divergence is intentional** — "Desktop uses String because Tauri IPC can't serialize enums" or "String is easier for WebView debugging." A pragmatic trade-off with a documented reason. Status: **Not found; both approaches are technically viable.**

4. **Mock completeness is enforced by test** — A test that explicitly checks "all 5 authorities are mocked in desktop" and fails if one is missing. Status: **Not found; e2e tests don't cover all authorities.**

5. **Sighash golden-value test with protocol spec reference** — A fixture that says "per SPS-65 Section X, for MultisigAction with fields [A, B, C] and seqno Y, the sighash must be [fixed hex]." Status: **Not found; no golden-value test.**

6. **SSZ codec boundary enforced by clippy rule or test** — A `#[test] fn test_only_action_codec_imports_strata_asm()` that fails if other modules import strata_asm. Status: **Not found; comment is aspirational only.**

---

## Summary & Risk Profile

**Blocking issues (require ADR + changes before scaling):**
1. Desktop error type divergence (impacts error recovery, testing, maintainability)
2. Authority enum duplication (breaks silently on new authority addition)
3. ASM role membership split coverage (offline validation impossible in desktop)

**High priority (impacts reliability, testing):**
4. Configuration defaults without validation (test key could leak to production)
5. Handler validation pattern inconsistency (unclear boundaries)
6. Mock injection strategy (hard-coded, non-deterministic)

**Medium priority (affects future feature velocity):**
7. Missing ADRs for sighash, broadcast, uptime assumptions
8. SSZ codec boundary violated but not enforced
9. Sighash golden-value test missing

**Low priority (hygiene):**
10. Unused feature flag
11. Incomplete broadcast error handling documentation

---

**Recommendation:** Prioritize blocking issues (1–3) before shipping PayoutAdmin (Slice 5). These are architectural boundaries that are easier to fix now than after they've been replicated 5x across more authority-specific logic.
