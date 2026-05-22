# Cross-cutting Drift (Rust↔TS) — Adversarial Assessment

> Fresh re-audit (2026-05-14). Independent re-read of the live tree — not a delta over the 2026-05-13 axis-04. Where this report restates a prior finding, it adds new evidence; where it contradicts it (e.g. `/api/v1` is now in place), it says so.

## Scope & threat model (what we're trying to break)

**Trust boundaries under attack:**

1. **Backend HTTP wire** (`orchestrator-be/src/handlers/*.rs` ↔ `desktop-app/src-tauri/src/application/orchestrator_client.rs` over reqwest) — JSON with `snake_case` field names, typed `Authority`/`ProposalStatus`/`BroadcastStatus` enums.
2. **Tauri IPC** (`desktop-app/src-tauri/src/commands/*.rs` ↔ `desktop-app/src/api/*.ts` over `invoke()`) — Tauri commands serialize via `serde_json`; mixed `camelCase` (proposals) and `snake_case` (orchestrator-auth) conventions in the same crate.
3. **TS runtime view** (`desktop-app/src/api/proposals.ts`, `orchestrator-auth.ts`, `tauri-bridge.ts`) — string-union TS types, no runtime validation. `ApiResult<T> = { ok; data } | { ok: false; error: string }` flattens every error.

**Attack surface we're trying to break (concrete signer/operator harms):**

- A) **State desync via broadcast bypass.** Tauri's `proposals_broadcast` calls `bitcoind` directly without ever telling the backend → backend stays in `approved`, FE displays "Enacted". Re-broadcast trivially possible.
- B) **Duplicate-signer bypass via hex case.** Backend approves session via `eq_ignore_ascii_case` but checks duplicates with `==`. Same signer can sign twice (upper- then lower-case pubkey).
- C) **Authority enum subset.** Tauri's `Authority` knows only `strata_admin`; any backend `Proposal` carrying `sequencer_manager` / `alpen_admin` / `security_council` / `payout_admin` fails Tauri deserialization → "invalid response" error in FE while data is fine on the wire.
- D) **u64 SeqNo precision loss** through Tauri IPC (JSON number → JS `number`).
- E) **Error context collapse.** 401 vs 409 vs 500 become an opaque `error: string`; UI cannot branch.
- F) **Hardcoded broadcast result.** Tauri `BroadcastResultDto` returns `"enacted"/"reveal_confirmed"` literals; FE-typed unions accept them and the FE invariant "displayed state = on-chain state" breaks.
- G) **Two parallel auth systems** (Tauri-local vs HTTP) using divergent wire strings (`strata_administrator` vs `strata_admin`) coexist in the same TS module.

**Evidence map (where I looked):**

| Surface | Path |
| --- | --- |
| Backend domain types | `orchestrator-be/src/domain/{proposal,authority,auth}.rs` |
| Backend handlers | `orchestrator-be/src/handlers/{proposals,auth,mod}.rs` |
| Backend error | `orchestrator-be/src/error.rs` |
| Backend application | `orchestrator-be/src/application/proposals.rs` |
| Tauri commands | `desktop-app/src-tauri/src/commands/{proposals,orchestrator_auth,authentication,signing}.rs` |
| Tauri HTTP client | `desktop-app/src-tauri/src/application/orchestrator_client.rs` + `infrastructure/orchestrator_client.rs` |
| Tauri local auth | `desktop-app/src-tauri/src/application/authentication.rs` + `domain/auth.rs` |
| Tauri local broadcast | `desktop-app/src-tauri/src/application/proposals.rs` |
| TS API facade | `desktop-app/src/api/{proposals,orchestrator-auth,authentication,tauri-bridge,signing,asm-state}.ts` |
| TS types | `desktop-app/src/types/{index,auth-role}.ts` |
| TS hooks/screens | `desktop-app/src/domain/**` + `desktop-app/src/screens/**` |

---

## Top findings (ranked) — Blocking/High | Medium | Low

### BLOCKER-1 — Tauri `proposals_broadcast` bypasses the backend's atomic broadcast claim; backend state never advances past `approved`

The backend implements race-safe broadcasting with an explicit atomic claim: `claim_broadcast(action_id)` transitions `Idle → CommitBroadcasted` exclusively and returns `Conflict` on contention (`orchestrator-be/src/application/proposals.rs:252-254`). The handler `execute_broadcast` (`/api/v1/proposals/:action_id/broadcast`, `orchestrator-be/src/handlers/proposals.rs:180-212`) is the only path that drives the state machine through `CommitBroadcasted → CommitConfirmed → RevealBroadcasted → RevealConfirmed → Enacted` (`application/proposals.rs:334-417`).

The desktop client does not call that handler. `commands::proposals::proposals_broadcast` (`desktop-app/src-tauri/src/commands/proposals.rs:280-316`) constructs its own `HttpBitcoinRpcClient`, parses operator key material from `BroadcastInput`, then calls the **desktop-local** `desktop_app::application::proposals::broadcast_commit_then_reveal` (`desktop-app/src-tauri/src/application/proposals.rs:109-229`). That function uses the orchestrator client **only** to `get_proposal` (line 120). It builds commit & reveal, calls `btc_rpc.send_to_address` (line 158) and `btc_rpc.send_raw_transaction` (line 207), and waits for confirmations — never touching the backend.

Effect on cross-cutting contract:
- Backend state remains `status = approved`, `broadcast_status = idle`, `commit_txid = NULL`, `reveal_txid = NULL` forever.
- FE redraws the dashboard after navigation back; `proposals_dashboard_screen.tsx:62` puts the proposal back into the `quorumReached` bucket, where the dashboard offers `onBroadcastProposal` again. Another signer (or the same one on a second machine) can re-trigger the entire commit/reveal sequence → two on-chain governance transactions for one proposal.
- The backend's `claim_broadcast` machinery is dead code from the desktop happy path.

This was **not** flagged in the 2026-05-13 axis-04 (which only flagged "Tauri has no claim mechanism"). The actual defect is stronger: the Tauri layer is not just missing local idempotency — it's actively side-stepping the backend's correct implementation.

Evidence:
- Rust:
  - `desktop-app/src-tauri/src/commands/proposals.rs:280-316` (`proposals_broadcast`)
  - `desktop-app/src-tauri/src/application/proposals.rs:109-229` (local `broadcast_commit_then_reveal`)
  - `desktop-app/src-tauri/src/main.rs:27` (registered command)
  - `orchestrator-be/src/handlers/proposals.rs:180-212` (unused `execute_broadcast`)
  - `orchestrator-be/src/application/proposals.rs:234-305` (atomic claim)
- TS:
  - `desktop-app/src/api/proposals.ts:114-116` (`broadcastProposal`)
  - `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts:79-86`
  - `desktop-app/src/screens/broadcast-proposal-screen.tsx:106-117` ("Proposal enacted onchain")
  - `desktop-app/src/screens/proposals-dashboard-screen.tsx:61-64` (re-buckets backend's stale `approved`)

---

### BLOCKER-2 — `BroadcastResultDto` is hardcoded `"enacted"/"reveal_confirmed"`; FE-typed unions accept the lie

`commands/proposals.rs:309-315`:

```292:316:desktop-app/src-tauri/src/commands/proposals.rs
    Ok(BroadcastResultDto {
        action_id: input.action_id,
        proposal_status: "enacted".to_string(),
        broadcast_status: "reveal_confirmed".to_string(),
        commit_txid,
        reveal_txid,
    })
```

These strings are NOT derived from the actual proposal record. `BroadcastResultDto` is `#[serde(rename_all = "camelCase")]` (`commands/proposals.rs:99-107`), so the FE receives `{ proposalStatus: "enacted", broadcastStatus: "reveal_confirmed", ... }`. The FE type is `BroadcastResult.proposalStatus: ProposalStatus` (`desktop-app/src/api/proposals.ts:38-44`) — both literals happen to be valid union members, so TypeScript happily prints "Proposal enacted onchain." (`broadcast-proposal-screen.tsx:108`) even though:

- The backend has never been told the broadcast happened (see BLOCKER-1).
- The actual `broadcast_commit_then_reveal` could have only reached `RevealBroadcasted` if the second confirmation timeout fired between the call and a partial failure (the return path early-exits to `BroadcastError::Timeout`, but if anything mutates outside the function's atomic awareness, the literal still ships).

The TS-typed string-unions provide a false sense of safety: the Tauri layer is the only place that constructs the value, and it constructs a literal — not a projection of state.

Evidence:
- Rust: `desktop-app/src-tauri/src/commands/proposals.rs:309-315`
- TS contract: `desktop-app/src/api/proposals.ts:4-12, 38-44`
- TS consumer: `desktop-app/src/screens/broadcast-proposal-screen.tsx:106-117`

---

### BLOCKER-3 — Backend duplicate-signer check is case-sensitive while authentication is case-insensitive (same module)

```37:43:orchestrator-be/src/application/proposals.rs
    if !sig
        .signer_pubkey
        .eq_ignore_ascii_case(session.signer_pubkey)
    {
        return Err(AppError::Unauthorized);
    }
```

```86:94:orchestrator-be/src/application/proposals.rs
    let already_signed = proposal
        .signatures
        .iter()
        .any(|s| s.signer_pubkey == sig.signer_pubkey);

    if already_signed {
        return Err(AppError::Conflict("signer already signed".to_string()));
    }
```

The same function — `approve_action` — validates that the requesting signer matches the session via `eq_ignore_ascii_case` (line 75-77) but checks duplicates via `==` (line 90). The FE preserves whatever casing the wallet adapter emits (`sign-poc-screen.tsx:38` lowercases for *display* comparison but `approveProposal` is called with the raw `signerPubkey` from the orchestrator session, `sign-poc-screen.tsx:168`). `use-create-proposal.ts:18-22` lowercases keys-to-add/remove but does NOT touch the signer's own key.

Attack: a signer creates a proposal with `signer_pubkey = "02AB...CD"` (uppercase from a hardware wallet variant), then re-authenticates with the same key reported lowercase by another adapter (Trezor T1 vs Safe-3 firmware variants, or the mnemonic path that uses `hex::encode` → always lowercase, `desktop-app/src-tauri/src/infrastructure/signing.rs:81-82`). Session check passes (case-insensitive). Duplicate check fails to detect (case-sensitive). Two signature rows with different `signer_pubkey` strings but the same underlying key. ASM threshold verification at reveal time will fail (canonical key ordering re-derives lower-case hex, so the matching-key search finds the same canonical key twice → only one true signer counted → threshold not met, broadcast eventually wasted).

Evidence:
- Rust: `orchestrator-be/src/application/proposals.rs:38-42, 87-90`
- Rust (broadcast verify path that re-canonicalizes): `desktop-app/src-tauri/src/infrastructure/broadcast_tx.rs` + ASM verification path
- TS (no normalization on self-key): `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:132` (sends `sig.publicKeyHex` raw)
- TS (cosmetic lower-case only): `desktop-app/src/screens/sign-poc-screen.tsx:38`

---

### HIGH-4 — Tauri `Authority` enum is a strict subset of backend's (`StrataAdmin` only); deserialization of non-Strata proposals fails

```12:42:desktop-app/src-tauri/src/domain/authority.rs
pub enum Authority {
    StrataAdmin,
}
...
    pub fn from_wire(s: &str) -> Result<Self, AuthorityParseError> {
        match s {
            "strata_admin" => Ok(Authority::StrataAdmin),
            other => Err(AuthorityParseError::Unknown(other.to_string())),
        }
    }
```

Backend ships five variants:

```5:12:orchestrator-be/src/domain/authority.rs
#[serde(rename_all = "snake_case")]
pub enum Authority {
    AlpenAdmin,
    StrataAdmin,
    SequencerManager,
    SecurityCouncil,
    PayoutAdmin,
}
```

And the FE already lists `StrataSequencerManager` (`desktop-app/src/types/auth-role.ts:3`) and `authorityFromRole` maps it to `"sequencer_manager"` (`desktop-app/src/api/orchestrator-auth.ts:48-50`). When the user logs in as Sequencer Manager and the dashboard calls `list_proposals`, the backend may return a `Proposal { authority: "sequencer_manager", ... }`. Tauri's `HttpOrchestratorClient::send_and_parse` (`desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:43-61`) routes it through `serde_json::from_str`, which invokes `Authority::deserialize` (`desktop-app/src-tauri/src/domain/authority.rs:52-57`) → `from_wire` → `Err(Unknown)` → `serde::de::Error::custom` → `OrchestratorError::Deserialization`. The FE sees `{ ok: false, error: "Failed to deserialize response: ..." }`. The data is on the wire; the contract is silently incompatible.

Compounding: `authorityFromRole`'s `default` branch returns `'strata_admin'` (`desktop-app/src/api/orchestrator-auth.ts:51-52`). Any future role added to the `AuthRole` enum without updating the switch falls through to Strata Administrator — silently signing into the wrong authority context.

Evidence:
- Rust: `desktop-app/src-tauri/src/domain/authority.rs:15-42, 52-57` vs `orchestrator-be/src/domain/authority.rs:5-12`
- TS: `desktop-app/src/api/orchestrator-auth.ts:45-54` (fall-through default), `desktop-app/src/types/auth-role.ts:1-4`
- Side-channel (Tauri serialization gate): `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:52-61`

---

### HIGH-5 — `u64` SeqNo round-trips through JSON number / JS `number`; precision unsafe above `2^53-1`

Backend `SeqNo = u64` (`orchestrator-be/src/domain/proposal.rs:9`); response DTOs use raw `u64` fields with serde default (e.g. `NextSeqNoResponse.next_seq_no`, `orchestrator-be/src/handlers/proposals.rs:10-13`). Tauri parses to `u64` (`desktop-app/src-tauri/src/application/orchestrator_client.rs:71-73`) and re-emits `u64` to the FE (`commands/proposals.rs:189-195`).

Tauri's IPC serializes integers via `serde_json::Value::Number`, so they reach JS as `number` — which is float64, with safe integer range `[-(2^53-1), 2^53-1]`. The FE TypeScript types these as `number` (`desktop-app/src/api/proposals.ts:16, 46-48, 50-56, 75-77`) and constructs `seqNo = Number(formData.seqNo.trim())` (`desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:114`) with no `Number.MAX_SAFE_INTEGER` guard — only `Number.isInteger(seqNo) && seqNo >= 0` (line 115).

Not exploitable today (real `seq_no` values are small), but it's a structural drift: any future protocol that pushes `seq_no` into the upper u64 half corrupts the round-trip silently. The TS schema (`desktop-app/src/domain/create-proposal/model/create-proposal.schema.ts:67-77`) accepts `\d+` of unbounded length — a user can type `99999999999999999999`, `Number()` returns `1e+20`, schema validation passes (`Number.isInteger(1e20)` is true), Tauri ships it to the backend, backend hashes a *different* `ActionId` than the wallet signed, signature mismatch.

Evidence:
- Rust: `orchestrator-be/src/domain/proposal.rs:9`, `desktop-app/src-tauri/src/domain/proposal.rs:11`, `desktop-app/src-tauri/src/commands/proposals.rs:14, 60, 119, 189`
- TS: `desktop-app/src/api/proposals.ts:16, 75-77`, `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:114`, `desktop-app/src/domain/create-proposal/model/create-proposal.schema.ts:67-78`

---

### HIGH-6 — `ApiResult.error` is a flat string; HTTP status / domain error variant lost across every layer transition

Three error models on the way down:

1. Backend `AppError` → `{Unauthorized, NotFound, BadRequest, Conflict, Internal}` mapped to HTTP status + `{"error": "<msg>"}` body (`orchestrator-be/src/error.rs:9-44`).
2. Tauri `OrchestratorError` → `{Request, Backend{status, message}, Deserialization}` (`desktop-app/src-tauri/src/application/orchestrator_client.rs:10-18`).
3. Tauri command bridge: `.map_err(|e| e.to_string())` (`commands/proposals.rs:194, 211, 220, 229, 242, 270, 307`) — the discriminated status is collapsed into a free-form sentence. Even worse: `map_proposal_error` / `map_broadcast_error` only special-case `status == 401`, producing an English sentence (`commands/proposals.rs:138-155`); everything else (409 Conflict, 503, deserialization) is forwarded as `other.to_string()`.

The TS layer pretends nothing was lost:

```1:4:desktop-app/src/types/index.ts
// ─── API Response shapes ──────────────────────────────────────────────────────

export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: string }
```

So FE branching on error semantics resorts to substring matching (`isSessionExpiredReauthError(err)` greps for `'Session expired'`, `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:14-16`). A backend translator that re-words "session expired" → "session has expired" breaks the FE silently.

Worse, "session unauthorized (401)" is only emitted for proposal/broadcast Tauri commands — `proposals_get_next_seq_no` and `proposals_prepare_broadcast` paths use `e.to_string()` directly (`commands/proposals.rs:193-194` vs `211`), so the same backend 401 produces a different human-readable string depending on which command surfaced it. FE can't normalize.

Evidence:
- Rust: `orchestrator-be/src/error.rs:9-44`, `desktop-app/src-tauri/src/application/orchestrator_client.rs:10-18`, `desktop-app/src-tauri/src/commands/proposals.rs:138-155, 193-194, 211, 270, 307`
- TS: `desktop-app/src/types/index.ts:3`, `desktop-app/src/api/tauri-bridge.ts:11-17`, `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:12-16`

---

### MEDIUM-7 — Naming convention split *inside* the Tauri crate: orchestrator-auth DTOs use snake_case, proposal DTOs use camelCase

Tauri's IPC layer is supposed to present a uniform shape to React. It doesn't.

- Proposals (`commands/proposals.rs:10-107`): every DTO carries `#[serde(rename_all = "camelCase")]`.
- Orchestrator-auth: `OrchestratorAuthChallenge` and `OrchestratorAuthSession` (`application/orchestrator_client.rs:46-68`) **do not** carry `rename_all`. They keep snake_case field names because they double as HTTP wire types. But the Tauri commands `orchestrator_auth_start/complete/get_session` (`commands/orchestrator_auth.rs:24-50`) return those same structs straight to the FE.

The FE compensates with parallel `Raw*` types and explicit field-by-field mapping:

```20:31:desktop-app/src/api/orchestrator-auth.ts
type RawOrchestratorAuthChallenge = {
	challenge_id: string
	challenge_hex: string
}

type RawOrchestratorAuthSession = {
	token: string
	authority: string
	signer_pubkey: string
	expires_at_unix_ms: number
}
```

```56:117:desktop-app/src/api/orchestrator-auth.ts
return tauriCall<RawOrchestratorAuthChallenge>(...).then((result) => {
    ...
    return {
        ok: true,
        data: {
            challengeId: result.data.challenge_id,
            challengeHex: result.data.challenge_hex,
        },
    }
})
```

Adding a field to `OrchestratorAuthSession` requires four coordinated edits (Rust DTO, Rust constructor, TS `Raw*` type, TS mapping). Missing the mapping silently drops the field at the IPC boundary with no compile-time signal — TS allows extra keys, and the mapping just ignores them.

Evidence:
- Rust: `desktop-app/src-tauri/src/application/orchestrator_client.rs:42-68` vs `commands/proposals.rs:10-107`
- TS: `desktop-app/src/api/orchestrator-auth.ts:20-31, 56-117`

---

### MEDIUM-8 — Two parallel auth systems with divergent wire formats live in the same TS module

The desktop app has two unrelated authentication code paths that both serialize an "authority/role" string:

1. **Tauri-local auth** (Tauri-only, no backend round trip): `commands::authentication::*` (`commands/authentication.rs`), `application::authentication` (`desktop-app/src-tauri/src/application/authentication.rs`), `domain::auth::AuthRole` (`desktop-app/src-tauri/src/domain/auth.rs:6-11`) with `#[serde(rename_all = "snake_case")]` over `StrataAdministrator/StrataSequencerManager` → wire `"strata_administrator"`, `"strata_sequencer_manager"`. The local `role_wire(...)` returns the same strings (`application/authentication.rs:255-260`).
2. **Orchestrator HTTP auth** (Tauri → backend): `commands::orchestrator_auth::*` (`commands/orchestrator_auth.rs:24-50`) → `application::orchestrator_auth` → backend `Authority::StrataAdmin/SequencerManager` etc. with wire `"strata_admin"`, `"sequencer_manager"` (`orchestrator-be/src/handlers/auth.rs:171-179`).

The FE has matching dual mappings in `desktop-app/src/api`:

- `authentication.ts:8-22` types use the local `AuthRole` (`'strata_administrator'`).
- `orchestrator-auth.ts:45-54` rewrites the same role to the HTTP wire string (`'strata_admin'`).

Two adversarial concerns:

a) **Drift between the two wire dialects.** Today `authorityFromRole` is explicit and correct, but the `default` branch returns `'strata_admin'` — any newly added `AuthRole` quietly authenticates as Strata Administrator on the backend while running as the new role in the Tauri-local session. The mismatch is silent: Tauri local thinks it is `StrataSequencerManager`, backend thinks it is `StrataAdmin`, dashboard label uses the local one (`proposals-dashboard-screen.tsx:28-29`), so the user sees "Sequencer Manager" while signing as Strata Administrator on chain.

b) **Two sources of truth for "am I authenticated?"**: `authGetSession` (Tauri-local, `desktop-app/src/api/authentication.ts:52-54`) and `orchestratorAuthGetSession` (backend session cached in Tauri memory, `desktop-app/src/api/orchestrator-auth.ts:96-117`). If one expires/clears and the other doesn't, the FE proceeds to call proposal APIs with an invalid backend session (returns 401) while local state thinks everything is fine.

Evidence:
- Rust: `desktop-app/src-tauri/src/domain/auth.rs:6-20`, `application/authentication.rs:255-260`, `orchestrator-be/src/domain/authority.rs:5-12`, `orchestrator-be/src/handlers/auth.rs:171-179`
- TS: `desktop-app/src/api/authentication.ts:8-58`, `desktop-app/src/api/orchestrator-auth.ts:45-54`

---

### MEDIUM-9 — Tauri's local `proposals_broadcast` has no idempotency; concurrent invocations re-broadcast on Bitcoin

Combined with BLOCKER-1 (no backend claim), this is an exploitable race. The FE disables the button via `<BroadcastDetailsCard isBroadcasting>` (`broadcast-proposal-screen.tsx:90-95`) inside one React process — but there is no app-wide lock. If the user opens a second desktop instance, both `proposals_broadcast` invocations:

1. Read the same `proposal` (`get_proposal` returns `approved`).
2. Each derives the same commit address (deterministic from action payload + operator key).
3. Each calls `btc_rpc.send_to_address(commit_address, ...)` (`desktop-app/src-tauri/src/application/proposals.rs:158-161`) producing **two different commit UTXOs** funding the same P2TR address.
4. Both build a reveal spending their own commit. One reveal wins (orphaning the other commit) — the orphaned commit's sats stay locked in the operator's UTXO set; the second reveal is invalid and may surface as a bitcoind error → `BroadcastError::BitcoinRpc`, which Tauri mapper passes back as a plain English string.

Backend never knew about either. The proposal record stays `approved`. If a third user opens the dashboard, the "Broadcast" button is offered again. Cycle continues.

Evidence:
- Rust: `desktop-app/src-tauri/src/application/proposals.rs:109-229` (no claim, no mutex)
- Rust: `desktop-app/src-tauri/src/commands/proposals.rs:280-316` (free async)
- FE: `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts:70-87` (single-instance disable only)

---

### MEDIUM-10 — TS string-union `ProposalStatus`/`BroadcastStatus` has no runtime validator; backend can grow new states silently

```4:13:desktop-app/src/api/proposals.ts
export type ProposalStatus = 'pending' | 'approved' | 'enacted' | 'canceled' | 'expired'

export type BroadcastStatus =
	| 'idle'
	| 'commit_broadcasted'
	| 'commit_confirmed'
	| 'reveal_broadcasted'
	| 'reveal_confirmed'
	| 'failed'
```

Tauri's `Proposal.status` is `String` (`desktop-app/src-tauri/src/domain/proposal.rs:13, 17` and the DTO `commands/proposals.rs:62, 66`). The HTTP client carries arbitrary strings through to the FE. The TS narrowing is compile-time only; `tauriCall<Proposal>` does no runtime check. If the backend ships `"approved_awaiting_quorum_reset"` or `"failed_with_recovery"`, the value flows through to FE comparisons:

- `proposal.status === 'pending'` returns false (`sign-poc-screen.tsx:39`) → "this proposal is no longer pending and cannot be signed" banner shows even though the value is non-terminal.
- `broadcastStatusToPhase(status)` (`desktop-app/src/domain/broadcast-proposal/model/broadcast-proposal.ts:5-20`) uses an exhaustive `switch` over the union and falls through `undefined` for unknown statuses — the function return type is `BroadcastPhase` but the runtime value is `undefined`, leading to silent renders of empty progress UI.

Evidence:
- Rust: `desktop-app/src-tauri/src/domain/proposal.rs:8-21`, `desktop-app/src-tauri/src/commands/proposals.rs:50-70, 116-130`
- TS: `desktop-app/src/api/proposals.ts:4-12`, `desktop-app/src/domain/broadcast-proposal/model/broadcast-proposal.ts:5-45`

---

### LOW-11 — `ProposalStatus` has `Display` but no `FromStr`; `BroadcastStatus` has both — asymmetric Rust contract

`orchestrator-be/src/domain/proposal.rs:42-57` defines `FromStr for BroadcastStatus` (returns `AppError::Internal` on unknown). `ProposalStatus` (line 60-86) only has `Display`. The handlers serialize statuses to strings explicitly (`handlers/proposals.rs:204-210` calls `.to_string()`) but no path reverses it. Any future code that needs to parse a `proposal_status` string from a broadcast response or external system has no library function — invites ad-hoc match expressions and divergence between callers.

Evidence:
- Rust: `orchestrator-be/src/domain/proposal.rs:42-86`

---

### LOW-12 — Backend response DTOs rely on `serde` defaults instead of explicit `#[serde(rename_all = "snake_case")]`

`PrepareBroadcastResponse`, `BroadcastResponse`, `NextSeqNoResponse`, `ProposalListResponse`, `CreateProposalRequest`, `ApproveActionRequest`, `ListProposalsQuery` (`orchestrator-be/src/handlers/proposals.rs:10-64`) do not carry an explicit `#[serde(rename_all = ...)]`. They happen to be snake_case because Rust style names the fields snake_case. The wire schema is therefore not stable against a future rename refactor — if someone renames `next_seq_no` to `nextSeqNo` for readability, the JSON wire silently changes from `next_seq_no` to `nextSeqNo` and breaks the Tauri client. Conversely, the Tauri ingress structs (`application/orchestrator_client.rs:21-73`) also lack explicit `rename_all`, so they too rely on snake_case field naming as the contract. The convention is implicit on both sides; explicit attributes would lock the wire format independently of identifier choices.

Evidence:
- Rust: `orchestrator-be/src/handlers/proposals.rs:10-64`, `desktop-app/src-tauri/src/application/orchestrator_client.rs:21-73`

---

### LOW-13 — `next_seq_no` integer carried as bare `u64` (not envelope) — same precision risk as HIGH-5, separate endpoint

`proposals_get_next_seq_no` returns `Result<u64, String>` to the FE (`desktop-app/src-tauri/src/commands/proposals.rs:189-195`); FE types it `Promise<ApiResult<number>>` (`desktop-app/src/api/proposals.ts:75-77`). No envelope. Documenting separately because the value is currency-of-the-protocol (every new proposal hashes `seq_no_be_bytes` into `ActionId`; a precision-corrupted `seq_no` produces an ActionId the wallet never signed → "Unauthorized" or "signature mismatch" at later steps).

Evidence:
- Rust: `desktop-app/src-tauri/src/commands/proposals.rs:189-195`, `orchestrator-be/src/handlers/proposals.rs:97-106`
- TS: `desktop-app/src/api/proposals.ts:75-77`

---

### LOW-14 — `signSighashMock` exists in the prod TS facade (`desktop-app/src/api/signing.ts:67-80`) — cross-cutting hygiene smell, not strictly a contract drift, but ships a "mock" Tauri command into release builds

Not central to this axis but worth flagging for the surface: `sign_sighash_mock` is invoked via `tauriCall` from the production TS bundle. A search shows the Tauri side actually registers `sign_action_sighash` (real ECDSA signing, `desktop-app/src-tauri/src/commands/signing.rs:21-27`), so `signSighashMock` calls the *real* signer — the "mock" label in TS is misleading. Naming drift between layers, fuel for incident-time confusion.

Evidence:
- TS: `desktop-app/src/api/signing.ts:67-80`
- Rust: `desktop-app/src-tauri/src/commands/signing.rs:21-27` (no `sign_sighash_mock` registered)
- IPC handler list: `desktop-app/src-tauri/src/main.rs:33-37`

> Note on retraction vs. 2026-05-13: the prior axis-04 listed "No API versioning" as MEDIUM. The repository now ships `/api/v1` (`orchestrator-be/src/main.rs:112`, `desktop-app/src/api/orchestrator-auth.ts:6`, `README.md:13`, `docs/deliverable/research.md:342`). That finding is **retracted**.

---

## Attack narratives (3–6)

### Narrative 1 — "I broadcast it twice; the second time it actually went on chain a second time"

Signer Alice has just collected the third (final) signature on a Strata Admin proposal. Backend marks it `approved`. Alice clicks "Broadcast" in the desktop app. `useBroadcastProposal.broadcast` invokes Tauri `proposals_broadcast` (`desktop-app/src/api/proposals.ts:114-116`), which calls the desktop-local commit/reveal pipeline (BLOCKER-1). Two confirmations later (regtest auto-mined), Tauri returns `{ proposalStatus: "enacted", broadcastStatus: "reveal_confirmed", commitTxid, revealTxid }` — but those last two strings are *literals*, not derived from any record (BLOCKER-2). Alice's UI shows "Proposal enacted onchain."

Meanwhile Bob, on a separate machine sharing the same operator key (operator key is passed *into* `BroadcastInput` from the renderer per `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts:38`), loads the dashboard. Backend still says `status: approved, broadcast_status: idle, commit_txid: null`. The dashboard buckets the proposal under `quorumReached` and offers "Broadcast" again. Bob clicks. Bob's Tauri instance fetches the same proposal, builds the same commit address (deterministic on `(action_id, payload, operator_key, network)`), and broadcasts a fresh commit UTXO + reveal. Two governance enactments hit chain. ASM does its own dedup at the protocol layer, but the operator pays for two on-chain broadcasts; if both commits get spent by their respective reveals before ASM dedup, the governance action fires twice (within whatever idempotency the ASM enforces).

**Why static review missed it:** the desktop's `broadcast_commit_then_reveal` reads almost identical to the backend's version (same return shape, same canonical key fetching) but the entry point silently uses the *local* one.

---

### Narrative 2 — "I am signed in as Sequencer Manager but every list call returns ‘Failed to deserialize response'"

Sequencer Manager Mei picks `Strata Sequencer Manager` on the connect screen. `authorityFromRole(AuthRole.StrataSequencerManager)` returns `"sequencer_manager"`. `orchestrator_auth_start` posts that to the backend, which parses `Authority::SequencerManager` — challenge issued. Mei completes auth; backend stores a session bound to `Authority::SequencerManager`. So far so good.

Mei opens the dashboard. `listProposals` → `proposals_list` Tauri command → `HttpOrchestratorClient::list_proposals` (`desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:136-147`) → `send_and_parse::<ProposalListResponse>`. Inside the response, each `Proposal.authority` is `"sequencer_manager"`. `Authority::deserialize` (`desktop-app/src-tauri/src/domain/authority.rs:52-57`) calls `from_wire("sequencer_manager")` → `Err(Unknown(...))` → `serde::de::Error::custom`. `send_and_parse` returns `OrchestratorError::Deserialization`. Tauri command maps to `e.to_string()` → FE receives `{ ok: false, error: "Failed to deserialize response: ..." }`. Mei sees an opaque error and a blank list.

**Why static review missed it:** the backend serializes correctly, the Tauri auth flow accepts the role through to a real session, and `domain::authority::Authority` is *only* exercised after deserialization passes — which it never does for non-Strata. No integration test covers the SequencerManager `Proposal` deserialization in Tauri.

---

### Narrative 3 — "We applied the threshold update but the second co-signer is the same key — but with uppercase hex"

Trezor Suite returns compressed pubkeys lower-case; a legacy Sparrow export the operator imports presents the same key upper-case. The signer authenticates with the lower-case variant (session in backend bound to `signer_pubkey = "02ab...cd"`). Later, after backend restart, they re-auth with the upper-case variant from Sparrow. Session re-binds to `"02AB...CD"`. Now they `approve_action`. `eq_ignore_ascii_case` (`orchestrator-be/src/application/proposals.rs:38-42`) lets the session check pass — the session pubkey casing matches the sig pubkey casing now. But the proposal's `signatures` already contains the lower-case row from earlier. The dedup check uses `==` (line 90); the upper-case sig is treated as a *new* signer. Two rows stored.

At broadcast time, `build_signed_payload_bytes` (`desktop-app/src-tauri/src/infrastructure/broadcast_tx.rs`) maps each signature back to a canonical key from the ASM-ordered list. Both rows resolve to the same canonical key. Threshold counting collapses both into one. If the proposal needed 3 sigs and only "had" 3 (with two of them duplicates), the reveal is short-quorum. The reveal still gets broadcast — but ASM rejects it onchain. Operator pays the commit fee for nothing, governance action stalls.

---

### Narrative 4 — "Why did my proposal create with seq_no = 1 but my Trezor signed seq_no = 0?"

Operator types `seq_no: 9007199254740993` (one above `Number.MAX_SAFE_INTEGER`) into the form. Schema (`create-proposal.schema.ts:67-78`) checks `^\d+$` then `Number(...)` — returns `9007199254740992`. `Number.isInteger(9007199254740992)` is true. Validation passes.

Path 1: TS calls `computeSighash(seqNo, actionHex)` → Tauri `compute_sighash(9007199254740992u64, ...)` (JSON `Number` → `u64` parse). Sighash computed against `9007199254740992`.

Path 2: TS calls `createProposal({ seqNo, actionHex, signerPubkey, signatureHex })` — `seqNo` is the JS-rounded value. Tauri parses `9007199254740992u64`. Backend `compute_action_id(9007199254740992, action_hex)` matches the sighash → the proposal is created.

Now a different operator on a different desktop receives the proposal, calls `getProposalByActionId`. Backend returns the proposal serialized with `seq_no: 9007199254740992` (correct). HTTP parsing into Tauri's `Proposal { seq_no: u64 }` succeeds. Tauri returns to FE as JS `number = 9007199254740992`. FE recomputes sighash for the *same* value → matches. So it works **as long as everyone uses the same rounded value**. The issue surfaces if anyone (CLI, future API consumer, alternative backend) ever stores or transmits the un-rounded value `9007199254740993` — sighash diverges silently.

The protocol level: ASM doesn't see JS at all. It sees the BE-bytes encoded `seq_no` from the on-chain reveal. If the operator typed `9007199254740993` thinking that's their seq, but on-chain it's `9007199254740992`, post-mortem of "why did the governance action skip my seq?" is impossible without auditing JS rounding.

---

### Narrative 5 — "Backend says 409 Conflict but the UI says 'something went wrong, try again'"

Signer Carlos re-tries an `approve_action` after a flaky network. Backend correctly returns 409 `{"error":"conflict: signer already signed"}` (`orchestrator-be/src/application/proposals.rs:93`). Tauri HTTP client wraps as `OrchestratorError::Backend { status: 409, message: "conflict: signer already signed" }` (`desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:52-56`). Tauri command `proposals_approve` maps via `map_proposal_error` — which only special-cases 401 (`commands/proposals.rs:138-145`). Everything else: `other.to_string()` → `"Orchestrator returned error 409: conflict: signer already signed"`. FE shows the raw string in `setSignError(String(e))` (`sign-poc-screen.tsx:177`). Carlos sees the error and concludes the system is broken, retries again, again, again — each attempt re-spending bandwidth and triggering log alerts. The UX should have said "You already signed — proceed to the next step." It can't, because the FE has no `errorCode` to branch on.

---

### Narrative 6 — "Our Tauri-local session says I'm Sequencer Manager but the backend signed me in as Strata Administrator"

A future developer adds `AuthRole.SecurityCouncil = 'security_council'` to `desktop-app/src/types/auth-role.ts`. The TS compiler is happy; nobody updates `authorityFromRole` (`desktop-app/src/api/orchestrator-auth.ts:45-54`). A signer for the Security Council picks the role at sign-in. Tauri local auth flow correctly recognizes `SecurityCouncil` (its `AuthRole` is updated separately — but in fact it isn't because Tauri's `domain::auth::AuthRole` is *not* in sync with the TS enum). Two paths:

- If the Tauri-local enum got updated, the dashboard label shows "Security Council".
- `authorityFromRole(AuthRole.SecurityCouncil)` hits the `default` → returns `'strata_admin'`. `orchestrator_auth_start { authority: "strata_admin" }` issues a Strata Administrator challenge.
- The Security Council signer signs the Strata Administrator challenge. Backend verifies the signature is valid AND that the signer is a member of the Strata Administrator role (`orchestrator-be/src/handlers/auth.rs:125-133`). If the human signer happens to be on both sets (some signers may be), the backend issues a session bound to Strata Administrator authority. The signer believes they're acting as Security Council.

The proposals they create from that session land under the *wrong* authority in the backend. Other Strata Admin signers see them. ASM verifies against Strata Admin keys. Governance enacts a Security Council intent under Strata Admin authority. Catastrophic only if the signer happens to be a member of both groups (which the threat model contemplates for shared signers).

---

## Evidence index (paths)

### State synchronization & broadcast bypass (BLOCKER-1, BLOCKER-2, MEDIUM-9)
- `orchestrator-be/src/application/proposals.rs:234-305, 252-254, 308-417`
- `orchestrator-be/src/handlers/proposals.rs:156-178, 180-212`
- `desktop-app/src-tauri/src/commands/proposals.rs:246-278, 280-316, 309-315`
- `desktop-app/src-tauri/src/application/proposals.rs:52-102, 104-229, 158-161, 207-210`
- `desktop-app/src-tauri/src/main.rs:21-27`
- `desktop-app/src/api/proposals.ts:38-44, 110-116`
- `desktop-app/src/domain/broadcast-proposal/hooks/use-broadcast-proposal.ts:16-87`
- `desktop-app/src/screens/broadcast-proposal-screen.tsx:97-117`
- `desktop-app/src/screens/proposals-dashboard-screen.tsx:35-73, 139-141`

### Pubkey case handling (BLOCKER-3)
- `orchestrator-be/src/application/proposals.rs:37-43, 75-94`
- `orchestrator-be/src/handlers/proposals.rs:128-154`
- `desktop-app/src-tauri/src/infrastructure/signing.rs:67-84, 145-162` (always lowercase output via `hex::encode`)
- `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:18-22, 60-77, 126-134`
- `desktop-app/src/screens/sign-poc-screen.tsx:35-39, 161-173`

### Authority enum drift (HIGH-4, MEDIUM-8)
- `orchestrator-be/src/domain/authority.rs:5-12`
- `orchestrator-be/src/handlers/auth.rs:16-19, 171-179`
- `desktop-app/src-tauri/src/domain/authority.rs:15-100`
- `desktop-app/src-tauri/src/domain/auth.rs:6-20`
- `desktop-app/src-tauri/src/application/authentication.rs:30-44, 255-260`
- `desktop-app/src/api/orchestrator-auth.ts:45-54`
- `desktop-app/src/api/authentication.ts:8-22, 44-58`
- `desktop-app/src/types/auth-role.ts:1-4`

### Integer precision (HIGH-5, LOW-13)
- `orchestrator-be/src/domain/proposal.rs:9, 92`
- `orchestrator-be/src/handlers/proposals.rs:10-13, 97-106`
- `desktop-app/src-tauri/src/application/orchestrator_client.rs:21-27, 70-73`
- `desktop-app/src-tauri/src/domain/proposal.rs:11`
- `desktop-app/src-tauri/src/commands/proposals.rs:14, 60, 119, 189-195`
- `desktop-app/src/api/proposals.ts:16, 46-48, 50-56, 75-77`
- `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:113-126, 150-156`
- `desktop-app/src/domain/create-proposal/model/create-proposal.schema.ts:19, 67-78`

### Error model flattening (HIGH-6)
- `orchestrator-be/src/error.rs:9-44`
- `desktop-app/src-tauri/src/application/orchestrator_client.rs:10-18`
- `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:43-61`
- `desktop-app/src-tauri/src/commands/proposals.rs:138-155, 193-194, 209-211, 218-220, 226-229, 240-243, 268-270, 306-307`
- `desktop-app/src/types/index.ts:1-4`
- `desktop-app/src/api/tauri-bridge.ts:1-18`
- `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:12-16, 137-141`

### Naming convention split (MEDIUM-7, LOW-12)
- `orchestrator-be/src/handlers/proposals.rs:10-64`
- `orchestrator-be/src/handlers/auth.rs:16-43`
- `desktop-app/src-tauri/src/application/orchestrator_client.rs:21-73`
- `desktop-app/src-tauri/src/commands/proposals.rs:10-107`
- `desktop-app/src-tauri/src/commands/orchestrator_auth.rs:7-22`
- `desktop-app/src/api/orchestrator-auth.ts:8-117`

### TS string-union runtime gap (MEDIUM-10)
- `desktop-app/src/api/proposals.ts:4-12`
- `desktop-app/src/domain/broadcast-proposal/model/broadcast-proposal.ts:1-46`
- `desktop-app/src/screens/sign-poc-screen.tsx:39, 260-266`
- `desktop-app/src/screens/proposals-dashboard-screen.tsx:61-73`
- `desktop-app/src-tauri/src/domain/proposal.rs:8-21`
- `desktop-app/src-tauri/src/commands/proposals.rs:50-130`

### Rust contract asymmetry (LOW-11)
- `orchestrator-be/src/domain/proposal.rs:28-86`

### Misleading mock naming (LOW-14)
- `desktop-app/src/api/signing.ts:67-80`
- `desktop-app/src-tauri/src/commands/signing.rs:21-27`
- `desktop-app/src-tauri/src/main.rs:33-37`

### API versioning (retraction vs. 2026-05-13)
- `orchestrator-be/src/main.rs:106-114`
- `desktop-app/src/api/orchestrator-auth.ts:6`
- `README.md:13`
- `docs/deliverable/research.md:342`

---

## Smallest fixes vs largest bets (be explicit)

### Smallest fixes (≤ 1 day each, surgical)

1. **Drop `BroadcastResultDto`'s hardcoded statuses** (BLOCKER-2). In `desktop-app/src-tauri/src/commands/proposals.rs:280-316`, after `broadcast_commit_then_reveal` returns, re-`get_proposal(action_id)` from the backend and project its real `status` / `broadcast_status` into the DTO. Cost: ~6 lines; immediate consistency for FE consumers.

2. **Normalize pubkey hex to lowercase on every entry to backend** (BLOCKER-3). One-line change at `orchestrator-be/src/application/proposals.rs:74-99` to call `.to_ascii_lowercase()` before insert and before comparison; plus the same in `create_update_action` line 47 when stashing the first signature. Equivalent change in the desktop `Signature` constructor (`desktop-app/src-tauri/src/application/proposals.rs:267-275`). Pair with a unit test that signs once with mixed case and asserts duplicate-rejection.

3. **Expand Tauri `Authority` to mirror backend** (HIGH-4). Five-variant enum in `desktop-app/src-tauri/src/domain/authority.rs`; same wire strings as the backend. Removes the silent deserialization failure for non-Strata proposals.

4. **Add `errorCode` to `ApiResult`** (HIGH-6). Extend `desktop-app/src/types/index.ts` with `errorCode?: 'CONFLICT' | 'UNAUTHORIZED' | 'NOT_FOUND' | 'BAD_REQUEST' | 'INTERNAL' | 'NETWORK' | 'DESERIALIZATION'`. Have Tauri commands return a structured error (a `serde_json::Value` or a typed enum). FE switches on the code for UX branching.

5. **Replace `authorityFromRole` `default` with exhaustive `switch (true)` + `never` assertion** (MEDIUM-8). Force a TS compile error on any new `AuthRole` that isn't explicitly mapped. Six lines.

6. **Apply `#[serde(deny_unknown_fields)]` to backend response DTOs** (LOW-12) and add explicit `#[serde(rename_all = "snake_case")]` to every wire struct. Locks the wire schema against future identifier renames.

7. **Add a `serde_json::from_str::<ProposalStatus>` runtime guard in the Tauri Proposal mapper** (MEDIUM-10). On unknown status, log and surface a typed Tauri error rather than passing the unknown string through. Or simpler: change `desktop-app/src-tauri/src/domain/proposal.rs` to deserialize `status` as the backend's typed `ProposalStatus`/`BroadcastStatus` enums (requires sharing the backend types via a small `multisig-protocol` crate — see "Largest bets").

8. **Rename `signSighashMock` → `signSighash` and `sign_sighash_mock` Tauri target → `sign_action_sighash`** (LOW-14). The TS facade is wrong by name and would mislead an incident responder.

### Medium bets (3–7 days)

9. **Promote the orchestrator broadcast endpoint to the only broadcast path** (BLOCKER-1 + MEDIUM-9). Delete `proposals_broadcast` Tauri command's local commit/reveal; replace with a Tauri call to `POST /api/v1/proposals/:action_id/broadcast`. Keep `proposals_prepare_broadcast` as today since prepare-only is also implemented backend-side. This single change collapses the entire double-broadcast risk and reuses backend `claim_broadcast()`. The Tauri-local broadcast path remains useful as the *manual fallback* (operator runs CLI), but it should never be invoked from the GUI's happy path.

10. **Generate TS proposal/auth types from Rust DTOs** (HIGH-4, MEDIUM-7, MEDIUM-10). Use `ts-rs`, `typeshare`, or `specta` (already de facto Tauri ecosystem) on the Rust DTOs. Single source of truth for field names, enums, and snake/camel decisions.

11. **Add HTTP integration tests for cross-authority `list_proposals`** (HIGH-4). Spin up the backend with an in-memory repo seeded with `Proposal { authority: SequencerManager }`. Call `list_proposals` from the Tauri HTTP client (compile a test binary using the actual crate). Assert success. Will fail today, locking in the fix.

12. **Carry `seq_no` as a string on the JSON wire** (HIGH-5). One coordinated change: backend DTOs `pub seq_no: String` (with custom serde that round-trips `u64` ↔ decimal string), Tauri client mirrors, FE uses `BigInt` end-to-end. The form input already collects a string; the schema can be widened. Cost: ~one day, mostly tests.

### Largest bets (multi-week, structural)

13. **Extract a `multisig-protocol` crate** (HIGH-4, BLOCKER-3, MEDIUM-10, LOW-11). One Cargo crate carrying `Authority`, `ProposalStatus`, `BroadcastStatus`, `ActionId`, `SeqNo`, normalization helpers (`canonical_pubkey_hex(s: &str) -> Cow<str>`), and the action-id hash function. Both `orchestrator-be` and `desktop-app/src-tauri` depend on it. No more silent enum drift; pubkey normalization is enforced by the type system (`Pubkey(String)` newtype that only accepts lowercase). TS codegen from this crate is the natural follow-up.

14. **Define a JSON Schema or OpenAPI spec for `/api/v1` and run it as a contract test in CI** (every drift item in this report). The schema acts as the third party that both backend and Tauri client must satisfy. Reject any backend response or Tauri-mapped DTO that doesn't match. Costs: schema authoring (~3 days), CI plumbing (~2 days), maintenance ongoing. Long-term ROI is the highest of any item here.

15. **Single auth subsystem in Tauri** (MEDIUM-8). Today there are two parallel auth code paths with overlapping state machines (`authentication` and `orchestrator_auth`). Pick one and delete the other. The HTTP-bound `orchestrator_auth` is the source of authority truth (it talks to the ASM-derived membership); the local Tauri auth is a leftover from an earlier POC. Removing it removes a class of "which session is real" bugs.

---

## What would change my mind (missing evidence / experiments)

1. **If `desktop-app/src/api/proposals.ts`'s `broadcastProposal` is in fact dead code** (the production flow uses an *operator console* binary not in this repo to call the backend `/broadcast` endpoint), then BLOCKER-1/2/MEDIUM-9 lose their teeth. Test: search for *any* caller of `POST /api/v1/proposals/:action_id/broadcast` outside the Rust binary's own test suite. Today I see only `e2e-tests/tests/e2e_propose_sign.rs` references — meaning the endpoint isn't yet driven from the desktop GUI by design. Confirmation would shift BLOCKER-1 to MEDIUM (still surprising, still a fallback gap, but not a routine prod path).

2. **If a single signer is never on both Strata Admin and another authority's key set** (PRD policy), Narrative 6 stops being catastrophic — at worst the FE auth fails. Test: read the PRD and grep ASM admin-state seeds for cross-membership.

3. **If `desktop-app/src-tauri/src/domain/authority.rs`'s single-variant enum is intentional ("POC-4 ships only Strata Admin")** and the FE is gated so SequencerManager flows never reach `proposals_list`, HIGH-4 reduces to LOW. The code comments at line 14 say "Single variant for POC-4; new roles will be added as the feature set grows" — but the FE already exposes `StrataSequencerManager` in `AuthRole`. Either delete the FE option for now or expand Tauri. Whichever happens first deflates the finding.

4. **If `Number(formData.seqNo)` is gated upstream by a UX max value** (e.g., the form actually fetches `nextSeqNo` and disables manual entry above some sane bound), HIGH-5 reduces to MEDIUM. Current code (`create-proposal-form.tsx:266-275`) shows the form pre-fills from `nextSeqNo` but allows editing. A check like `seqNo <= Number.MAX_SAFE_INTEGER` is one line.

5. **If `desktop-app/src-tauri/src/application/proposals.rs` has any kind of in-memory single-flight wrapper** (a `OnceCell<Mutex<HashMap<ActionId, ()>>>`), MEDIUM-9 disappears for the single-instance case. I didn't find one; greping for `Mutex` in that file returns only the test helper at line 333. Worth re-verifying with a one-line search before triage.

6. **If the production Tauri build sets `tauri.conf.json#security.csp` to a non-disabled value AND restricts which JS contexts can call `proposals_broadcast`**, the renderer-sourced operator key concern (referenced in `docs/assessment/2026-05-14-adversarial/02-rust-tauri-adversarial.md:17`) softens. That is axis-02 territory, but its severity feeds back into BLOCKER-1's risk profile (operator key leak makes double-broadcast attacker-controllable).

7. **If E2E tests verify status round-trip from backend `ProposalStatus::Approved` → JSON `"approved"` → Tauri `Proposal.status: String` "approved" → FE `proposal.status === 'approved'`**, MEDIUM-10 risk drops. `e2e-tests/tests/e2e_propose_sign.rs:49` is the right file to look at; if the asserts cover non-Pending statuses, the runtime safety net is partial.

8. **If a future PR has already introduced shared types via `ts-rs`/`specta`/`typeshare`** but it isn't merged yet, items 10/13/14 are partially in-flight; the suggested fixes can be coordinated with that PR rather than introduced in isolation.

If items 1–4 above are all true, the axis verdict drops from "BLOCKER" to "HIGH". If 1, 5, and 6 are all true, the broadcast-bypass risk is a UX gap, not a security gap. Otherwise the present ranking stands.
