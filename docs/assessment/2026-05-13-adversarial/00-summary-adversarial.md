# Alpen Multisig — Adversarial Assessment Rollup (2026-05-13)

## Scope, method, and constraints

Synthesis of 17 axis reports produced 2026-05-13 against the `alpen-multisig` monorepo (Rust orchestrator backend + Tauri 2 + React/TS desktop app, coordination-only multisig signing). Inputs covered: backend code [axis 01], Tauri shell [axis 02], React frontend [axis 03], Rust↔TS drift [axis 04], platform/CI/CD/observability [axis 05], application architecture [axis 06], distributed systems realism [axis 07], DDD/domain [axis 08], data engineering [axis 09], testing strategy [axis 10], failure modes/troubleshooting [axis 11], product discovery [axis 12], product owner/UX journeys [axis 13], diverge/options coherence [axis 14], Diataxis/docs [axis 15], research/source-citation [axis 16], and agent/skill/rule definitions [axis 17].

Method: read-only audit of code, ADRs, PRDs, story map, specs. No runtime probes executed. Severity inherits from the axis with the strongest evidence chain; conflicts are surfaced explicitly. Every claim is cited by axis number. Read this rollup top-down; the executive verdict captures the bar that production must clear.

## Executive verdict (one paragraph, brutal but specific)

Alpen Multisig is a thoughtfully layered POC that is **not safe to operate as a governance authority today**. The backend silently falls back to an in-memory store and a publicly-known test operator secret key when env vars are missing, leaks all proposals across authorities because `list_proposals`/`get_proposal` discard the `AuthenticatedSession` extractor, and re-implements quorum/threshold logic the PRD explicitly forbids; the Tauri shell accepts plaintext mnemonics and operator keys across IPC with `csp: null` and a `regtest` default; the React layer has no Zod validation on IPC, no runtime authority-scope guard on session reuse, can recompute a different sighash between preview and Trezor confirmation, and has zero frontend tests; releases are unsigned, dependencies (git-pinned `alpen-*` revs, npm `^`-ranges, no lockfile) have no SCA/audit; durability, idempotency, structured logging, rate limiting, request correlation, retention, encryption-at-rest, and a backend ops runbook are all missing; user discovery, payload-divergence visualization, manual-fallback workflow, DoR enforcement, and an SPS-65→code citation chain are unwritten or unvalidated; and the rule/skill stack itself drifts between `.claude/rules/` and `.cursor/rules/`. Smallest credible production-readiness gate is 4–6 weeks of focused work just to close the Tier 0 items; full production hardening as the axes envision it is roughly a quarter [axes 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12, 13, 14, 15, 16, 17].

## Ranked org-level backlog

### Tier 0 — Security & signer safety (BLOCKING)

1. **Operator secret key defaults to publicly-known test key `0x00...01` if env unset.** Risk: anyone observing the commit tx can forge a competing reveal with the test key, stealing the multisig commit UTXO. Smallest fix: remove `unwrap_or_else` for `OPERATOR_SECRET_KEY_HEX`, fail startup, refuse the literal test value. Larger bet: secrets manager / KMS / Vault, key rotation procedure, audit log of every operator-sign event [axis 01 F2, 02 D4, 05 BLOCKER-003, 06, 11 #8, 14 H4].

2. **Authority scope leakage across all read endpoints.** `list_proposals` and `get_proposal` in `orchestrator-be/src/handlers/proposals.rs` accept `AuthenticatedSession` as `_auth` and never filter by authority; `prepare_broadcast` and `execute_broadcast` likewise omit the authority check. Risk: any signer of any authority enumerates and reads every other authority's proposals (PRD §3.2 explicitly forbids existence inference). Smallest fix: add `authority` parameter to `list_by_status`/`find_by_action_id`; in `get_proposal`, return 401 not 404 on authority mismatch. Larger bet: full repo-level authority scoping with integration test that confirms cross-authority denial [axis 01 F1, F3, F9, 03 D2, 06 #7, 08 (ubiquitous language)].

3. **Plaintext secrets across the Tauri IPC boundary.** `sign_with_mnemonic_path`, `sign_action_sighash`, and `BroadcastInput.operator_secret_key_hex` all accept mnemonics / private keys as plain `String` from the webview; bearer tokens are cloned through `OnceLock<Mutex<…>>` with no `zeroize`. Risk: a single compromised React dep (XSS, supply-chain) exfiltrates every signer's key. Smallest fix: never accept raw mnemonics from the webview — accept derivation indices only and load from OS keychain; wrap sensitive fields in `ZeroizeOnDrop`. Larger bet: split-signing daemon / hardware-wallet-only operator key, see Tier 1 [axis 02 D1, D2, D4, 05 BLOCKER-003, MEDIUM-002].

4. **CSP disabled in Tauri (`"csp": null`).** Any XSS via a compromised React/npm dependency runs with full `window.__TAURI__.invoke()` access and can call every command including `proposals_broadcast` and signing commands. Smallest fix: set `"default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' https://…"`. Larger bet: Tauri 2 capabilities per window, SRI for bundled JS, pinned and audited deps [axis 02 D3, 05 BLOCKER-005].

5. **No client-side validation that the backend returned the proposal the user requested (MITM / malicious-backend window).** `orchestrator_client::create_proposal` returns whatever the backend serializes; `broadcast_commit_then_reveal` uses `proposal.action_hex` from the backend response, not what the user entered. Smallest fix: hash the user's submitted action and compare against the returned proposal before signing/broadcast; re-display action and require explicit confirmation. Larger bet: Merkle-root commitment / signed-proposal envelopes returned by backend [axis 02 D5, 03 D1, D6].

6. **Payload divergence: signer cannot verify on the hardware wallet what they sign in the UI.** Trezor only shows a 32-byte SPS-65 sighash; authority and action semantics are not displayed on-device; UI text says "verify it matches" without telling the signer how. No `docs/specs/sps65-signing-visualization.md` exists. Smallest fix: prominent on-screen "verify this hex matches your device" warning that names the authority, plus a frozen-preview gate (see #7). Larger bet: protocol-level addition of authority/tag bytes inside the signed payload + Trezor firmware integration that prints them; multi-employee signed binaries (PRD NF-3) [axis 02 D5, 03 D5, 12 (digest test), 13 #2, 15 #3, 16].

7. **Sighash swap between preview and sign in `create-proposal-form.tsx`.** Preview computes sighash S1 from `getValues()`; clicking back to edit and then submitting recomputes a different sighash S2 from the new form values without re-displaying the change. Smallest fix: freeze form values at preview, deep-compare on submit, force re-preview if anything changed. Larger bet: capture-and-sign UX where the canonical sighash is rendered once and re-confirmed [axis 03 D4, 13 #2].

8. **No runtime validation of IPC results at the Tauri bridge.** `tauriCall<T>` casts unknown JSON to `T`; backend (or compromised IPC) can return `status: "executed"`, fabricated `signatures: []`, or unknown enum variants and the UI renders them as valid. Smallest fix: Zod schemas for `Proposal`, `ProposalStatus`, `BroadcastStatus`, `AuthSession` at the bridge. Larger bet: codegen TS types from Rust serde (or shared `multisig-types` crate / OpenAPI schema) and CI contract tests [axis 03 D1, 04 BLOCKER-1, BLOCKER-2, 10 B3].

9. **Session token has no authority binding; cross-authority token reuse.** `OrchestratorAuthSession.token` is opaque; `ensureOrchestratorSession` returns early if `currentSession !== null` regardless of `selectedRole`. `authLogout()` is fired but not awaited before re-auth. Smallest fix: validate `session.authority === authorityFromRole(selectedRole)` before reuse; `await authLogout()` before re-auth. Larger bet: signed JWT carrying authority claim, short access + refresh tokens [axis 03 D2, D3, 04 BLOCKER-1, 13 #4 (cross-authority bug in `broadcast-proposal-screen.tsx` line 17 mapping `StrataAdministrator → "Alpen Administrator"`)].

10. **Deep-link `/proposals/:actionId/sign` bypasses authority context.** `RequireAuth` only checks `isAuthenticated`; an attacker-supplied link makes the app fetch and prompt to sign a proposal belonging to a different authority. Smallest fix: in `sign-poc-screen.tsx`, refuse to render if `proposal.authority !== authorityFromRole(selectedRole)`. Larger bet: require navigation provenance (must come from dashboard with explicit selection) [axis 03 D6, 13 #4].

11. **No release signing, no SCA, no lockfile, git-rev pinned `alpen-*` crates with no signature verification, npm `^`-ranges and `package-lock.json` not committed.** A compromised transitive dep (recall axis 02 D3) is trivially shipped to signers as an unsigned binary. Smallest fix: commit `package-lock.json`, `npm ci`, add `cargo audit`/`cargo deny` to CI (do NOT skip "noisy" pre-release warnings as ADR-004 currently allows), add `detect-secrets`/`TruffleHog` pre-commit. Larger bet: signed `release.yml` with Apple/Authenticode signing, PGP-signed checksum manifest, Tauri updater verification, multi-employee signing per PRD NF-3 [axis 05 BLOCKER-001, BLOCKER-004, HIGH-001, 02 D3].

12. **Backend re-implements protocol validity rules in violation of "coordination only".** `approve_action` auto-transitions to `Approved` based on `signatures.len() >= required_signatures` (axis 16); `required_signatures` is snapshotted at create time and never re-synced with on-chain ASM threshold. PRD §1 explicitly forbids "signature threshold checks". Risk: backend marks Approved → user broadcasts → ASM rejects because the real threshold changed. Smallest fix: either remove the auto-transition and let on-chain determine quorum, or write an ADR explicitly carving out "advisory" quorum detection and add a threshold-resync test. Larger bet: dynamic threshold sync per proposal; remove all SPS-65 logic from backend; add a forbidden-import lint on `strata-crypto`/`strata-asm-params` in `orchestrator-be/Cargo.toml` [axis 16 #1, #2, 06 #1, 13 #5; synthesized risk (cite axes 16 + 13): threshold change mid-proposal can desync — flagged: Synthesized risk (cite axes)].

13. **Default network is `regtest` in `parse_network`.** A frontend bug or missing param silently broadcasts on the wrong network with the operator key. Smallest fix: remove default; require explicit `bitcoin`/`testnet`/`signet`/`regtest` and fail otherwise [axis 02 D6].

14. **Bearer token transported over user-supplied `base_url`; no HTTPS enforcement.** Anyone passing `http://…` ships the bearer in clear. Smallest fix: reject non-`https://` in `build_client` (allow `http://localhost` only in dev). Larger bet: cert pinning [axis 02 D7].

15. **Frontend `VITE_OPERATOR_SECRET_KEY_HEX` env path exists and can leak via sourcemaps.** Operator key must never traverse the frontend bundle. Smallest fix: delete the env var path; load operator key only in Rust at startup. Larger bet: hardware-wallet-only operator signing [axis 05 BLOCKER-003, MEDIUM-002, 02 D4].

### Tier 1 — Correctness, durability, idempotency (HIGH)

16. **In-memory storage is the silent default when `DATABASE_URL` is unset.** A crash or pod restart deletes all in-flight proposals and sessions; PRD §2.3 offline-fallback guarantee is violated because signers no longer know what they were signing. Smallest fix: fail startup if `DATABASE_URL` missing in production mode. Larger bet: durable append-only event log [axis 01 F5, 05 BLOCKER-002, 07 #1, 09 #1, 11 #4].

17. **Auth challenges and sessions live in `Arc<RwLock<HashMap>>` with no persistence, no TTL cleanup, no distributed store.** Memory grows unbounded under challenge spam (no rate limit), sessions vanish on restart, multi-instance load balancing fails. Smallest fix: TTL cleanup task + `tower-governor` rate limit + `parking_lot` RwLock to avoid poison fail-closed. Larger bet: Postgres- or Redis-backed sessions [axis 01 F4, F7, 02 D2, 07 #3, #6, 09 #4, 11 #7].

18. **Broadcast is non-atomic; partial state corrupts proposals.** `claim_broadcast` flips status to `CommitBroadcasted`; subsequent commit tx, fee estimate, reveal tx, and DB writes are not wrapped. A panic or Bitcoin-RPC timeout (no `tokio::time::timeout` on any RPC call) strands the proposal with a commit UTXO on-chain and `broadcast_status` desynced. No `/reset-broadcast` admin endpoint exists. Smallest fix: timeout-wrap every BTC/ASM RPC call; add reset endpoint guarded by admin auth. Larger bet: durable, resumable broadcast state machine + Postgres advisory lock [axis 01 F8, 06 #8, 07 #2, #10, 11 #4].

19. **Duplicate-signer race + non-linearized quorum transition.** `approve_action` reads, checks "already_signed" in the application layer, then `add_signature` in the repo without holding the lock across the check; under retry storms or hardware-wallet stalls two requests can both pass the check, append duplicate sigs, and both attempt `update_broadcast_status(Approved)`. The on-chain ASM then rejects "insufficient unique signers". Smallest fix: move duplicate detection inside `add_signature` under the write lock; have `add_signature` return `(proposal, quorum_reached)` and let the repo own the state transition. Larger bet: optimistic locking with `version: u64` CAS [axis 07 #4, #5, 08 (race), 10 B1].

20. **Tauri broadcast path has no idempotency / no in-flight dedupe / no UI button disable.** Double-clicking, retrying after timeout, or a malicious frontend can broadcast the same commit UTXO twice; Bitcoin-RPC error becomes "input already spent" downstream. Smallest fix: disable Send button while broadcasting; cache action_ids of in-flight broadcasts in Tauri. Larger bet: `Idempotency-Key` header passed end-to-end; backend stores `(action_id, idempotency_key) → (commit_txid, reveal_txid)` [axis 02 D10, 04 BLOCKER-5, 07 #8, 10 H1].

21. **`u64` seq_no silently rounded by JavaScript.** `seq_no` deserializes as JSON `number`; above `2^53−1` precision is lost; backend will reject the corrupted value as a different proposal. Smallest fix: serialize `seq_no` as string in JSON; parse via `BigInt` in TS. Larger bet: codegen-checked numeric types [axis 04 HIGH-4].

22. **Status enums travel as opaque strings in TypeScript; no enum guard.** Backend can ship a new `pending_expired` variant and the UI silently renders it as `pending`, hiding urgency. Combined with axis 04 BLOCKER-1 (camelCase vs snake_case vs `AuthRole.StrataAdministrator='strata_administrator'`), serialization mismatches will surface as 401/400 with no obvious cause. Smallest fix: branded TS unions for `ProposalStatus`, `BroadcastStatus`, `Authority`; Zod parse at the bridge (see Tier 0 #8). Larger bet: shared `multisig-types` codegen [axis 04 BLOCKER-1, BLOCKER-2, 06 #10, 08 (5 vs 2 authorities), 14 MEDIUM (enum duplication)].

23. **Error model collapses to `error: string` across the bridge.** Backend `AppError { Unauthorized, NotFound, BadRequest, Conflict, Internal }` → desktop `OrchestratorError::Backend{status, message}` → React `ApiResult<T>.error: string`. The UI cannot tell retry-safe from non-idempotent, nor "device disconnected" from "session expired". Smallest fix: add `errorCode` discriminant to `ApiResult`; thread HTTP status and Tauri error category through. Larger bet: typed `DesktopError` enum mirroring `AppError`; per-error UI recovery copy [axis 04 BLOCKER-3, 14 BLOCKING (desktop error type), 11 #2, #3].

24. **Signer pubkey case sensitivity mismatch between backend and Tauri.** Backend compares with `eq_ignore_ascii_case`; Tauri uses `==`. A hardware wallet that returns mixed-case hex can both pass and fail duplicate detection, inflating quorum. Smallest fix: normalize hex pubkeys to lowercase at every ingress; DB `CHECK (signer_pubkey ~ '^[a-f0-9]{66}$')` [axis 04 MEDIUM-6, 09 #5].

25. **Mock RPC via URL pattern matching is wired into production code paths.** `asm_role_membership::is_signer_member_for_authority` checks `mock_membership` before real RPC; a typo'd env var that contains "mock" or "localhost" silently authorizes an attacker. Same anti-pattern in desktop, with the desktop mock hardcoded to StrataAdmin only — so e2e tests for the other 4 authorities silently hit unmocked code. Smallest fix: dependency-injected `AsmStateRpc` trait, mocks only in `#[cfg(test)]`. Larger bet: remove all mocks from production code [axis 06 #5, 14 (mock injection), 08 (authority mapping incomplete)].

26. **No SSZ validation at the create-proposal boundary.** Garbage `action_hex` is stored and only fails at broadcast time, days later, after expiry pressure has built. Smallest fix: SSZ-decode in the create handler; reject early with a clear error. Larger bet: action codec is the only `strata_asm_*` boundary — see #28 [axis 06 #3, 14 (SSZ codec leak)].

27. **No timeouts, retries, backoff on Bitcoin RPC or ASM RPC.** A stuck Bitcoin RPC pegs Tokio workers and cascades to a service-wide outage; signers fall back to manual workflows that don't exist. Smallest fix: `tokio::time::timeout` around every external call; structured retries with jitter. Larger bet: circuit breaker per dependency [axis 07 #10, 11 narrative 5].

28. **Strata crates leak from `infrastructure/action_codec.rs` into the desktop application layer.** ADR-005 promises a single import site; `application/proposals.rs` imports `strata_asm_txs_admin::actions::MultisigAction` directly, and `infrastructure/signing.rs` calls `MultisigAction::from_ssz_bytes` outside the codec. Smallest fix: route all SSZ decode through `action_codec`. Larger bet: enforce via clippy/`deny` lint that only `action_codec.rs` may import `strata_asm_*`/`strata_crypto` [axis 06 #4, 14 (SSZ codec)].

29. **No structured logging, no request/correlation IDs, no `/ready` probe.** Backend errors log `tracing::error!("internal error: {e}")` with no `action_id`, `authority`, or `seq_no`; frontend toasts show "Something went wrong" with no ID. A Sev-2 incident takes 15–30 min just to find the right log line. Smallest fix: `#[tracing::instrument]` on handlers with `action_id`/`authority`/`seq_no`; generate a request UUID in `tauri-bridge.ts` and surface it in error toasts; add `/health` + `/ready` that actually check Postgres/BTC/ASM. Larger bet: structured JSON logs + Prometheus metrics + SLO ("0 stuck proposals for >1h") + runbook [axis 04 MEDIUM-7, 05 MEDIUM-001, 11 #1, #3, #10].

30. **No persistent auth challenge / session store; pubkey + sig auth has no replay defense at the HTTP layer.** Replay-rejection test exists at the Tauri layer (axis 10 M1) but not at the HTTP backend. Smallest fix: persist consumed-challenge IDs with TTL in Postgres; backend test for `POST /auth/verify` with replayed `challenge_id`. Larger bet: stateless JWTs + nonce ledger [axis 09 #4, 10 M1].

31. **No append-only event/audit log.** Status and broadcast transitions are overwritten in place; auditors cannot reconstruct who signed when, an indispensable property for governance. Smallest fix: `proposal_events` table append-only with `event_type` + `data jsonb` + `created_at`. Larger bet: event sourcing as the source of truth with materialized read views [axis 08 (no events), 09 #6, 11 #4].

32. **No frontend tests; no Tauri IPC contract tests; no concurrent-approval test; no broadcast error-path test; signature verification only happy-path.** Combined with axes 04, 03, 06, this means most Tier 0 risks would not be caught by CI. Smallest fix: add the 3+6+5+6 tests enumerated in axis 10. Larger bet: e2e harness covering all 5 authorities, real ASM state, hardware-wallet smoke test [axis 10 B1, B2, B3, H1, H2, H3].

33. **BIP-137 recovery-id normalization missing in `broadcast_tx::build_signed_payload_bytes`.** Trezor's `signMessage` 65-byte format (header 27–42) is mis-parsed as `recid||r||s`; ECDSA recover fails and surfaces as a generic "signature invalid". Smallest fix: import or implement `normalize_recovery_id`. Larger bet: defined PSBT path that doesn't depend on BSM at all [axis 11 narrative 3].

34. **No proposal expiry enforcement.** `ProposalStatus::Expired` exists but nothing transitions to it; a 9-day-old proposal can still be signed and broadcast, only to be rejected on-chain. Smallest fix: check `now > expires_at` in `approve_action` and broadcast paths. Larger bet: background expiry job [axis 07 #11, 13 #10].

35. **Threshold snapshot vs on-chain change.** `required_signatures` is snapshotted at proposal creation. If the on-chain threshold changes, the UI counter and the auto-Approved transition can both be wrong in opposite directions. Pairs with #12. Smallest fix: refuse to broadcast if backend `required_signatures` doesn't match a freshly-fetched ASM threshold (or document explicitly that the snapshot is binding) [axis 13 #5].

36. **Hardcoded constants: `REVEAL_TX_VBYTES = 350`, `COMMIT_DUST_SATS = 1500`, magic bytes `0x414c504e`, derivation path `m/86'/0'/73'/0/*`.** Duplicated between backend and desktop; not configurable per network; role 73 is StrataAdministrator-only — signers on other roles will derive the wrong key. Smallest fix: centralize in config; fetch role at auth and pass it into `list_mnemonic_addresses`. Larger bet: shared `multisig-types` crate owns these and is consumed by both binaries [axis 02 D8, 06 #9, 14 H4].

37. **Authority mapping incomplete in `asm_role_membership::authority_to_role`.** Only `StrataAdmin` and `SequencerManager` map; `AlpenAdmin`, `SecurityCouncil`, `PayoutAdmin` fall through to "not mapped" — a user who selects an unsupported authority gets a generic 400 with no upstream context. Smallest fix: add a test that fails until all 5 are mapped; mark blocked ones with `#[doc]` linking to `docs/2-discovery/08-alpen-crate-prd-coverage.md`. Larger bet: feature-gate or grey-out unsupported authorities in the UI [axis 06, 08 LOW, 16 H4].

38. **No signature golden-test against SPS-65.** Only StrataAdmin 2-of-3 is exercised; the assumption that `compute_sighash` returns the canonical SPS-65 digest for the other 4 authorities and 13 update types is unverified. Smallest fix: parameterized e2e test over the 5×N grid with golden hex from the SPS-65 spec [axis 10 H3, 14 (sighash), 16 H1].

39. **Wallet address not validated against the connected wallet in `session-provider.tsx`.** A signer can sign with a different Trezor account than the UI advertises; backend has no way to detect intent. Smallest fix: compare `signature.publicKeyHex` to `wallet.publicKeyHex` before submission [axis 03 D10].

40. **Tauri commands lack a capability/OCAP model.** Every registered command is callable from any window regardless of auth/role state; a malicious frontend can call `proposals_broadcast` before auth completes. Smallest fix: gate high-risk commands on `get_session()` + role check. Larger bet: Tauri 2 capabilities per window [axis 02 D11, 05 BLOCKER-005].

### Tier 2 — Maintainability, architecture, docs, agent specs (MEDIUM)

41. **`AppState` is a god-object.** 12 unrelated concerns (repo, ASM URL, challenges, sessions, BTC client, operator keypair, magic bytes, network, timeouts) mixed into one struct; adding a second network or scheme requires touching every handler [axis 06 #2].

42. **`Proposal` is an anemic data bag.** All fields `pub`; `signatures.push()` is the only mutator; invariants (no-add-after-Approved, status↔broadcast_status coupling) live in `application/proposals.rs` not in the aggregate. Pairs with #19 and #31; the race conditions become inevitable once Postgres replaces the global `RwLock`. Smallest fix: private fields + `add_signature_if_pending` method. Larger bet: split into `ProposalAggregate` + `BroadcastAggregate` [axis 08 BLOCKING, 09 #12].

43. **Authority duplication & ubiquitous-language drift.** Backend `Authority`: 5 variants; desktop `Authority`: 5 variants (separately defined); React `AuthRole`: 2 variants (`strata_administrator`, `strata_sequencer_manager`). Story map names 5 actors. Bug in `broadcast-proposal-screen.tsx` line 17 maps `StrataAdministrator → 'Alpen Administrator'`. Smallest fix: fix the label bug; align React enum strings with backend wire format; add round-trip serde test. Larger bet: shared `multisig-types` crate consumed by backend + desktop + e2e [axis 04 BLOCKER-1, 06 #10, 08 (drift), 13 #4, 14 (auth enum dup)].

44. **Bitcoin/Strata types in the application layer.** `prepare_broadcast_bundle(operator_keypair: &UntweakedKeypair, network: Network, …)` makes the application depend on `bitcoin::Network` and `strata_l1_txfmt::MagicBytes` directly. Pairs with #28 [axis 06 #4, 08 MEDIUM].

45. **`SessionContext` lives in `application/` not `domain/`.** Cron jobs and batch tasks can't authorize on behalf of an authority without HTTP context [axis 08 MEDIUM].

46. **No API versioning.** No `/api/v1` path, no version header, no content-type negotiation; backend v2 + desktop v1 will break silently [axis 04 MEDIUM-9].

47. **No data retention, no soft delete, no FSM enforcement on `BroadcastStatus`.** `update_broadcast_status` can jump from `Idle` to `RevealConfirmed`; expired/canceled proposals accumulate forever; deletes cascade through `proposal_signatures` with no recovery. Smallest fix: `can_transition_to` predicate; `deleted_at` columns [axis 09 #9, #11, #12].

48. **No encryption at rest; broadcast errors echo RPC URLs and credentials into client responses.** Smallest fix: `pgcrypto` for `signer_pubkey`/`signature_hex`; sanitize broadcast errors before persistence and exposure [axis 09 #7, #10].

49. **No desktop local persistence.** Drafts vanish on app crash; signers may "shortcut" through manual hex construction to avoid losing work [axis 09 #8].

50. **DIVIO / Diataxis collapse in `README.md` and `AGENTS.md`.** README mixes tutorial + how-to + reference + explanation; AGENTS.md commands are a flat list with no "when to use" guidance. New engineer onboarding takes a day instead of 30 min [axis 15 #6].

51. **Critical missing docs.** No backend operations runbook; no `ADR-006` formalizing the coordination-only boundary (pairs with #12); no signer-safety model spec (pairs with Tier 0 #6); no threat model / incident playbook; no living capability matrix for the 8 still-upstream-blocked update types; no build-and-release reproducibility guide; no testing-strategy doc; superseded discovery docs not flagged at the head of the file [axis 15 #1, #2, #3, #4, #5, #7, #8, #9, #10].

52. **`docs/3-stories/` has no Definition of Ready / Definition of Done.** Stories like US-F1 list "human-readable representation" without specifying what that means on Trezor; US-H5 promises offline fallback that doesn't exist; no cancellation flow for the 7-day window; no signer-set-rotation story (compromised-key emergency removal). Smallest fix: 8-item DoR checklist; story-by-story audit. Larger bet: rewrite Slice 0 to include offline fallback as a walking-skeleton invariant [axis 13 #7, #8, narratives 5–6].

53. **Zero user discovery.** No interviews with real signers; "manual fallback works" and "signers can verify 32-byte digests" are assumed not validated; payout flows are speced before the script template is known [axis 12 (all)].

54. **Rule/skill stack drifts.** `.claude/rules/react-frontend-patterns.md` (68 lines, includes Architecture-by-Domain) vs `.cursor/rules/react-frontend-patterns.mdc` (39 lines, omits it); `.cursor/rules/general.mdc` duplicates AGENTS.md Key Conventions verbatim; `sdd/SKILL.md` and `sprint-board/SKILL.md` lack `description:` for auto-trigger; `rust-specialist` skill and `rust-backend-standards` rule disagree on `.unwrap()`. Smallest fix: delete `.cursor/rules/` and let AGENTS.md / `.claude/rules/` be the single source; add `description` fields. Larger bet: lint that fails CI on duplicate guidance [axis 17].

55. **"SPS-65 is the source of truth" but the source is not in the repo.** No `docs/specs/sps-reference/` excerpts; no code comment cites a specific SPS-65 section. Pairs with #12: the contradiction between docs and code is unverifiable. Smallest fix: archive key SPS-50/51/65 excerpts; add section-id comments in `signing.rs`, `proposals.rs`, `action_codec.rs` [axis 16 #2, #5].

56. **Crate pinning rationale incomplete (ADR-001).** Rev-pin for `alpenlabs/asm` and tag-pin for `strata-common` are mixed without an explicit trade-off and no defined "convergence signal" to migrate. Smallest fix: expand ADR with pro/con + migration trigger [axis 16 #4].

57. **Vestigial Tauri `custom-protocol` feature flag, deprecated GraphQL in `sprint-board` skill, unused configuration fields.** Hygiene [axis 14 LOW, 17 LOW].

## Cross-cutting themes (merged duplicates across axes)

- **"Backend coordination only" is asserted but unenforced.** Threshold checks, signature checks (or lack thereof), and authority enforcement live in the application layer; no forbidden-import lint; no ADR-006; no SPS-65 citation. The drift between docs and code (axis 16) is the single most consequential governance risk and explains why axes 01/06/16/13 disagree on what the backend should do [axes 01 F11, 06 #1, #5, 13 #5, 14 H4, 15 #2, 16 #1, #2].
- **Single source of truth is broken everywhere.** Authority defined 3× (backend, desktop Rust, React); rules duplicated (`.claude/` vs `.cursor/`); AGENTS.md key conventions copy-pasted into `.cursor/rules/general.mdc`; mockable values (operator key, magic bytes, role 73) hardcoded in multiple places. The fix-direction is consistent across axes: a shared `multisig-types` crate + a shared rule store [axes 04, 06, 08, 14, 17].
- **In-memory by default leaks across every concern.** Proposals, sessions, challenges, broadcast claims, broadcast cache are all `Arc<RwLock<HashMap<…>>>` with no TTL, no persistence, no distributed lock, no rate limit. One outage = total state loss [axes 01, 05, 07, 08, 09, 11].
- **No correlation chain frontend → Tauri → backend → on-chain.** No request IDs, no structured logs, no error codes, no `/ready` check, no metrics. Tier 0 and Tier 1 incidents are diagnosable only by guessing [axes 04, 05, 11, 15].
- **Signer safety UX is uncodified.** No spec linking the UI payload to the device display; no authority context on Trezor; sighash swap window between preview and sign; deep-link bypass; copy/paste signature workflow has no security review; mnemonic and operator key crossing the webview/IPC boundary [axes 02, 03, 12, 13, 15].
- **Testing pyramid is hollow above the unit layer.** Backend unit tests are decent; integration tests cover only one authority and the happy path; concurrent approval, broadcast error paths, signature negative paths, frontend, IPC contracts, durability-on-restart, all-5-authorities sighash, and rate-limit DOS all untested [axis 10].
- **Synthesized risk (cite axes 04 + 07 + 08 + 09): once Postgres replaces `RwLock`, every existing race condition (duplicate signer, quorum transition, idempotency, broadcast claim) becomes worse because the in-process write lock that accidentally serialized everything is gone.**
- **Synthesized risk (cite axes 02 + 05 + 17): the combination of CSP-disabled webview + npm `^`-ranges without a lockfile + no SCA in CI + unsigned releases + IPC-plaintext secrets + drifting rule guidance is a textbook supply-chain attack surface and is the single most likely path to catastrophic compromise.**

## Disagreements between axes (where two reports conflict, what's at stake)

- **Should the backend validate signatures at approve time?** Axis 01 F11 says no (LOW, by design — coordination only). Axis 06 #1 says yes (BLOCKING — backend must verify sighash matches the canonical SPS-65 computation). Axis 16 #1 says emphatically no (BLOCKING — backend already over-reaches by auto-Approving on threshold, and PRD §1 forbids "threshold checks"). Reconciliation: hygiene checks (compact-ECDSA format, 64-byte length, lowercase hex) belong in the backend; canonical SPS-65 / threshold validity belongs on-chain. Stakes: getting this wrong puts the backend into split-brain with the ASM (axis 16 attack narrative 2).
- **`list_proposals` authority scope severity.** Axis 01 F1 calls it BLOCKER (cites exact handler, missing filter, PRD §3.2). Axis 06 #7 calls it MEDIUM. Stakes: cross-authority confidentiality. Axis 01's evidence is stronger; treat as BLOCKER.
- **In-memory-by-default severity.** Axis 01 F5 says HIGH; axis 05 BLOCKER-002, axis 07 #1, axis 09 #1 say BLOCKING/CRITICAL. Stakes: PRD §2.3 offline-fallback guarantee. Consensus is BLOCKING; axis 01 understates.
- **RwLock poisoning.** Axis 01 F4 marks it HIGH and lists `parking_lot` as a real fix. Axis 02 D2 treats the same `Mutex`/`OnceLock` pattern as a memory-disclosure issue with `zeroize` as the fix. Both are valid but address different threats; both fixes are required.
- **Manual fallback feasibility.** Axis 12 (CRITICAL — untested with users) and axis 13 (#1, narrative 6 — deferred to Slice 5 with no spec) agree the workflow doesn't work; axis 15 #1 frames it as a missing ops doc. Stakes: the entire premise that the backend is "not a single point of failure" is unsubstantiated. Treat as Tier 0 once user research lands.
- **Authority enum extraction urgency.** ADR-005 says "not needed yet"; axes 04, 06, 08, 14 say it's needed before PayoutAdmin/Slice 4. ADR-005's "yet" is doing a lot of work; treat axes 04/14 as binding.

## Confidence table

| Claim category | Confidence | Justification (axis evidence) |
|---|---|---|
| Backend authorization gaps (`_auth` discarded in list/get/broadcast) | High | Axis 01 cites exact handler files and line numbers (108–126, 156–212) for F1/F3/F9; corroborated by axis 06 #7. |
| Operator key handling (test-key default + IPC + frontend env path) | High | Axes 01 F2, 02 D4, 05 BLOCKER-003, 06, 11 #8, 14 H4 all cite `config.rs:56–61` and the IPC struct in `commands/proposals.rs:74–88`. |
| In-memory state durability | High | Axes 01 F5, 05 BLOCKER-002, 07 #1, 09 #1 all cite `main.rs:90–104` and the `RwLock<HashMap>` pattern. |
| IPC type drift (camelCase / snake_case / `AuthRole` vs `Authority`) | High | Axis 04 details every mismatch with file:line; axis 06 #10 and axis 08 confirm independently. |
| Sighash preview-vs-sign swap | High | Axis 03 D4 traces both code paths (`create-proposal-form.tsx:124–138` + `use-create-proposal.ts:110–145`); axis 13 #2 corroborates. |
| Broadcast atomicity / stuck state | High | Axes 01 F8, 06 #8, 07 #2, 11 #4 converge on `application/proposals.rs:234–289` with the same failure modes. |
| Race conditions (duplicate signer, quorum) | Med-High | Axes 07 #4/#5, 08, 10 B1 cite specific paths and ranges; only axis 10 notes the test gap. No actual load test was run, so risk is theoretical-but-grounded. |
| CSP disabled / supply chain | High | Both axes 02 D3 and 05 BLOCKER-004/005 cite `tauri.conf.json:21–23` plus the package-lock absence. |
| Backend re-implementing threshold checks | High | Axis 16 #1 cites `application/proposals.rs:103–104` and the test at 557–580; pairs with axis 06 #1. |
| u64 precision loss | Med | Axis 04 HIGH-4 is the sole source; risk is real but only triggers above `2^53`; production seq_nos are small today. |
| Frontend test absence | High | Axis 10 B2 ran a `find` (empty); no other axis disputes. |
| Diataxis collapse in README/AGENTS | Med | Axis 15 #6 reasons from doc structure; no usability test data. |
| Story map gaps (DoR, signer-rotation, offline fallback) | High | Axis 13 enumerates story-by-story; axis 12 and 15 corroborate from different angles. |
| Product-discovery gap (no signer interviews) | High | Axis 12 is the only source but its evidence (lack of any interview artifact in `docs/2-discovery/`) is concrete. |
| Agent/skill conflicts (`.claude/` vs `.cursor/`) | Med | Axis 17 compares files line-by-line; one source. Possible the divergence is intentional caching but no doc says so. |
| SPS-65 source-of-truth claim unverified | High | Axis 16 #2/#5 searched for citations and found none; PRD does not use "coordination only" phrasing. |
| BIP-137 normalization missing | Med | Axis 11 narrative 3 traces `broadcast_tx.rs:108–115` against Trezor BSM header behavior. One source, technically detailed. |
| HW-wallet payload divergence | High | Axes 02 D5, 03 D5, 13 #2, 15 #3, 16 all agree from independent angles. |
| Default `regtest` network | High | Axis 02 D6 cites `commands/proposals.rs:158` directly. |
| Postgres-time race amplification (synthesized) | Med | Axes 04 + 07 + 08 + 09 each imply it; no axis demonstrates it because Postgres path is shallowly tested. |

## What we might still be wrong about (adversarial self-critique)

- **No axis ran the disconfirming probes it proposed.** Axis 01 lists 5 concrete experiments (auth-leak test, F2 startup behavior, F3 status-code parity, F5 restart durability, F7 memory growth). None were executed in this engagement; findings are code-read severity, not measured. If production deploy scripts enforce env-var presence, several Tier 0 items collapse to LOW.
- **The "coordination only" contradiction (axis 16) depends on an SPS-65 interpretation we cannot verify locally.** SPS-65 is a Notion document; we have no archived copy. Axis 16's strongest evidence is that the PRD §1 forbids "Signature threshold checks", but the PRD also describes the backend as a "coordination service" handling "Proposal state tracking prior to quorum", which could be read as licensing the auto-Approve transition. If Alpen confirms threshold detection is in-scope, finding #12 downgrades.
- **Authority enum count: 5 vs 2.** Axis 08 reads `desktop-app/src/types/auth-role.ts` literally; it is possible the 2-variant React enum is a Slice-0 placeholder and the full mapping is pending. The bug in `broadcast-proposal-screen.tsx` line 17 ("Strata"→"Alpen") could be a typo from copy-paste, not a security issue per se. We did not check git blame for intent.
- **`.cursor/rules/` vs `.claude/rules/` divergence (axis 17).** We assumed `.cursor/rules/` should mirror `.claude/rules/`. It is possible `.cursor/rules/` is a Cursor IDE artifact written by the editor itself, not a source-controlled rule. We did not verify how Cursor loads rules at runtime.
- **u64 precision (axis 04 HIGH-4) and BIP-137 normalization (axis 11 narrative 3) are real CS but may not be reached in practice.** seq_nos in the current test fixtures are single-digit; Trezor users may prefer PSBT over `signMessage`. Severity should drop if hardware-wallet UX is constrained.
- **Manual-fallback unfeasibility (axes 12 + 13) is asserted from a doc gap, not from a usability test.** It is possible internal Alpen Labs operators can do this reliably even without documentation. We have no evidence either way.
- **All Tier 0 findings rely on code-read; no axis verified the binary actually deploys with these defaults.** Production env files (`.env.production`, k8s secrets, systemd units) may set the variables we flag as "fallback to test value". Confirmation requires access to the deploy pipeline.
- **Confirmation-bias risks.** Every axis applied an adversarial stance and looked for failure. Some findings (axis 17 LOW, axis 14 LOW, axis 08 LOW) may be hygiene only. The Diataxis critique (axis 15 #6) is largely structural; a redesigned README may not actually improve onboarding velocity without empirical test.
- **Concurrent-approval and broadcast-race claims (axes 07, 10) are theoretical until a load test confirms them.** The current Tokio + in-process `RwLock` may serialize enough by accident; Postgres path may already use `SELECT FOR UPDATE` that we did not read.
- **We did not audit hardware-wallet code in `hw_wallet/trezor.rs` or `ledger.rs`.** Axis 02 narrative D treats role-73 derivation as a footgun but did not look at firmware-side error handling.
- **The 17 axes overlap heavily.** Some "findings" are the same defect re-counted (operator key, in-memory storage, authority leakage, IPC drift). The Tier 0 list above merges these but a less careful reader would inflate the count.

## Smallest fixes vs largest bets (org-level)

**Smallest fixes that close the most Tier 0 surface (≤ 1 week of focused work, parallelizable):**

- Remove fallback for `OPERATOR_SECRET_KEY_HEX`; reject the literal test value at startup (#1).
- Add `authority` filter to `ProposalRepository::list_by_status` + `find_by_action_id`; return 401 not 404 on authority mismatch in `get_proposal` (#2).
- Set strict CSP in `tauri.conf.json` (#4).
- Reject non-`https://` in `build_client`; require explicit `network` parameter, no `regtest` default (#13, #14).
- Add Zod schema to `tauriCall` for `Proposal`, `ProposalStatus`, `BroadcastStatus`, `AuthSession` (#8).
- Validate `session.authority === authorityFromRole(selectedRole)` before reuse; `await authLogout()` before re-auth; fix the `'Strata' → 'Alpen Administrator'` label bug (#9, #43).
- Freeze form values at preview; require re-preview if changed (#7).
- Refuse to render sign screen when `proposal.authority` ≠ selected role (#10).
- Commit `package-lock.json`; `npm ci`; `cargo audit`/`cargo deny` in CI without skipping noisy warnings (#11).
- Make `DATABASE_URL` mandatory in production mode (#16).
- Add `tokio::time::timeout` to every BTC/ASM RPC call (#27).
- Add `#[tracing::instrument(action_id, authority, seq_no)]` on every handler; generate request UUID in `tauri-bridge.ts`; surface it in error toasts; add `/ready` checking Postgres + RPCs (#29).
- Normalize signer pubkey to lowercase at every ingress; DB `CHECK` constraint (#24).

**Largest bets (org-level, ≥ 2 weeks each):**

- Shared `multisig-types` crate consumed by backend + desktop + e2e; codegen TS types from it; round-trip serde + JSON contract tests (#22, #43).
- Durable append-only event log + replay-on-startup; foundation for crash recovery, audit trail, and a resumable broadcast state machine (#16, #18, #31).
- Move all SPS-65 validity decisions out of the backend; add a forbidden-import lint on `strata_asm_*`/`strata_crypto` in `orchestrator-be`; write `ADR-006: Backend Coordination Boundary` with SPS-65 section citations (#12, #55).
- Split-signing daemon (or hardware-wallet-only operator key) so secret keys never cross the webview/IPC boundary; OS-keychain integration for bearer tokens with `zeroize` (#3, #15, #40).
- Signed release pipeline (Apple Developer, Authenticode, PGP-signed checksum manifest, Tauri updater verification) satisfying PRD NF-3 (#11).
- Real product discovery: 5–8 signer interviews + digest-verification usability test + manual-fallback tabletop sim, then re-prioritize the offline fallback into Slice 0/1 with an explicit spec (#52, #53, #51 manual-fallback).
- Concurrency hardening: optimistic locking with `version: u64`, Postgres advisory lock for broadcast claim, atomic add-signature-if-pending with quorum transition in repo, distributed session store before any horizontal scaling (#19, #20, #42).
- Diataxis rewrite: README as 5-line tutorial, AGENTS.md as reference, dedicated backend-ops runbook, threat model, incident playbook, signer-safety-model spec, capability matrix, SPS-65 archive (#50, #51, #55).

## Suggested 2-week, 6-week, 12-week sequencing

**Weeks 1–2 (stop the obvious bleeding; close 8 of 15 Tier 0).** Tier 0 quick fixes #1, #2, #4, #7, #8, #9, #10, #13, #14, #15; Tier 1 #16 (mandatory `DATABASE_URL`), #24 (pubkey lowercase), #29 (request IDs + `/ready` + structured logging); Tier 2 #54 (delete `.cursor/rules/`); fix the `broadcast-proposal-screen.tsx` authority label bug; write `ADR-006` skeleton even before implementing it.

**Weeks 3–6 (correctness + supply chain + ops).** Tier 0 #3 (move secrets off the IPC boundary; OS keychain), #11 (signed release pipeline MVP — at least Linux Authenticode + checksum manifest), #12 (decide threshold-check policy and either remove it or write the ADR carving it out); Tier 1 #17–#21, #23, #27, #29 full, #32 (frontend test scaffolding + the 3+6+5+6 axis-10 tests), #33 (BIP-137 normalization), #36 (centralize constants); Tier 2 #50–#52 (backend ops runbook, threat model, signer-safety-model spec); user discovery starts in parallel.

**Weeks 7–12 (architectural hardening + governance integrity).** Shared `multisig-types` crate; durable event log; resumable broadcast FSM; Postgres advisory locks + optimistic locking; redis/Postgres session store; encryption at rest; retention policy; capability matrix as a live doc; SPS-65 archive + section citations in code; signed-release pipeline for all three OSes with Tauri updater; full e2e across all 5 authorities + ASM real-state integration; manual-fallback wizard speced and implemented; signer-rotation/compromise story (`US-E_ROTATE`); 8-item DoR enforced and all stories audited; agent/skill stack consolidated.

## Axis index (one line per axis: file path + top finding)

- `docs/assessment/2026-05-13-adversarial/01-rust-backend-adversarial.md` — Authority scope leakage in `list_proposals`/`get_proposal`/broadcast handlers (`_auth` discarded) and test-key default for `OPERATOR_SECRET_KEY_HEX` are BLOCKING [axis 01].
- `docs/assessment/2026-05-13-adversarial/02-rust-tauri-adversarial.md` — Plaintext secrets across IPC + `csp: null` + operator-key on IPC + no MITM defense against backend response substitution [axis 02].
- `docs/assessment/2026-05-13-adversarial/03-react-typescript-adversarial.md` — No runtime IPC validation, no authority-binding on session reuse, sighash swap between preview and sign, deep-link bypasses authority check [axis 03].
- `docs/assessment/2026-05-13-adversarial/04-cross-cutting-drift-adversarial.md` — Authority/status enum and error model drift across Rust↔TS; u64 precision loss; broadcast idempotency hole [axis 04].
- `docs/assessment/2026-05-13-adversarial/05-platform-cicd-observability-adversarial.md` — Unsigned releases, no SCA, no graceful shutdown, CSP off, operator key with weak defaults — five BLOCKING platform gaps [axis 05].
- `docs/assessment/2026-05-13-adversarial/06-application-architecture-adversarial.md` — Backend skips sighash verification, Strata crates leak into application layer, mock RPC wired into production paths, AppState god-object [axis 06].
- `docs/assessment/2026-05-13-adversarial/07-distributed-systems-adversarial.md` — In-memory state loss on restart, non-atomic broadcast, duplicate-signer race, non-linearized quorum, no rate limit, no RPC timeout — single-instance only [axis 07].
- `docs/assessment/2026-05-13-adversarial/08-domain-ddd-adversarial.md` — `Proposal` is an anemic data bag; ubiquitous-language drift (5 backend / 2 React authorities); no domain events; transactionless repo enables race [axis 08].
- `docs/assessment/2026-05-13-adversarial/09-data-engineering-adversarial.md` — In-memory default, no schema governance, no append-only audit log, no encryption at rest, no retention, pubkey case collisions [axis 09].
- `docs/assessment/2026-05-13-adversarial/10-testing-strategy-adversarial.md` — Zero frontend tests, zero Tauri-IPC contract tests, no concurrent-approval test, broadcast/signature negative-paths uncovered [axis 10].
- `docs/assessment/2026-05-13-adversarial/11-troubleshooting-failure-modes-adversarial.md` — No correlation IDs, no error codes, no `/health`, partial state on panic, BIP-137 header normalization missing, session expiry mid-broadcast [axis 11].
- `docs/assessment/2026-05-13-adversarial/12-product-discovery-assumptions-adversarial.md` — Zero user interviews, 32-byte digest verification untested with signers, manual fallback unvalidated — discovery REJECTED [axis 12].
- `docs/assessment/2026-05-13-adversarial/13-product-owner-requirements-adversarial.md` — No AC for payload-divergence, backend-unavailability, state-conflict mid-sign, signer-rotation; authority label bug in broadcast screen; DoR missing [axis 13].
- `docs/assessment/2026-05-13-adversarial/14-diverge-options-coherence-adversarial.md` — Backend `AppError` vs desktop `Result<T, String>`; SSZ codec boundary violated; mock injection by URL match; no ADR-006/007/008; authority enum duplication [axis 14].
- `docs/assessment/2026-05-13-adversarial/15-documentation-diataxis-adversarial.md` — No backend ops runbook, no coordination-boundary ADR, no signer-safety model, no incident playbook, README/AGENTS Diataxis collapse [axis 15].
- `docs/assessment/2026-05-13-adversarial/16-research-sources-adversarial.md` — Backend threshold checks contradict "coordination only" claim; SPS-65 cited as source-of-truth but no local archive; sighash validated only for StrataAdmin [axis 16].
- `docs/assessment/2026-05-13-adversarial/17-agent-spec-quality-adversarial.md` — `.claude/rules/` vs `.cursor/rules/` duplication and 27-line content gap; 6 skills missing `description:`; `sdd` skill has no override branch [axis 17].
