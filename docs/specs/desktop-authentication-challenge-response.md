# Spec: Desktop Authentication (Challenge-Response + Role Membership)

## Objective

Implement a basic desktop-only authentication flow where a signer proves key ownership by signing a challenge,
and the app authorizes access only if that signer key belongs to the selected administration role in ASM state.

This spec targets the desktop app first and does not require orchestrator-be participation.

## Scope

**Included (V1):**
- Role membership resolution from ASM runner JSON-RPC status (`getStatus` response)
- Decoding `AnchorState` and administration subprotocol section
- Extracting role signer keys from `AdministrationSubprotoState`
- Challenge-response authentication in desktop app
- Local role-based gating in desktop UI/session
- Security controls: nonce, expiry, single-use challenge, domain separation
- Fallback strategy when RPC is unavailable

**NOT included (V1):**
- Orchestrator-be auth endpoints/middleware/session issuance
- Backend-enforced authorization
- On-chain transaction validity checks (SPS-65 quorum verification remains separate)
- Complete role coverage beyond what upstream currently exposes
- Persistent identity accounts/user profiles

## Technical Design

### 1) Membership source: ASM JSON-RPC status

Desktop resolves role membership through the same pattern used by E2E helpers:

1. Call JSON-RPC `getStatus`
2. Read `cur_state.state` (or `current_state.state`) byte array
3. Decode SSZ into `AnchorState`
4. Locate administration subprotocol section (`AdministrationSubprotocol::ID`)
5. Decode to `AdministrationSubprotoState`
6. Resolve authority by role and extract `config().keys()`

Expected output shape for the auth module:

```text
role_to_keys: HashMap<Role, Vec<CompressedPublicKeyHex>>
```

### 2) Challenge-response flow

1. User selects wallet/vendor and role intent.
2. Desktop creates challenge payload:
   - `domain`: fixed auth tag (example: `alpen-multisig/auth/v1`)
   - `nonce`: 32-byte random
   - `issued_at`
   - `expires_at` (short TTL, suggested 60-120 seconds)
   - `role`
   - `session_id` (desktop-local)
3. Desktop hashes payload into challenge digest.
4. Wallet signs challenge.
5. Desktop verifies signature and obtains signer public key (recovery or equivalent wallet-verified output).
6. Desktop checks signer key membership in role key set.
7. On success, desktop creates local authenticated session scoped to that role.

### 3) Authorization model (desktop local)

- Authorization predicate:
  - `is_valid_signature(challenge, signature, signer_pubkey) && signer_pubkey in role_keys[role]`
- Session fields:
  - `role`
  - `signer_pubkey_hex`
  - `authenticated_at`
  - `membership_version` (or fetched timestamp)
  - `expires_at`
- Re-auth triggers:
  - Session expiry
  - Role change by user
  - Membership refresh indicating key set change

### 4) Signature format compatibility

Wallet adapters currently produce different signature formats. Auth verifier must define one of:

- **Preferred:** normalize to one challenge-signing format for all vendors
- **Alternative:** verifier dispatch by `signatureFormat` and apply vendor-specific validation path

Auth module must reject unsupported format combinations explicitly with actionable errors.

### 5) Failure handling and fallback

If membership resolution from RPC fails:

- Enter degraded mode using last successful cached key set (if still within configured freshness window), or
- Deny authentication with high-signal error explaining the RPC/key-state dependency.

V1 recommendation:
- Cache role key sets in memory and optional local storage
- Refresh on app start + explicit user action + periodic interval

## Security Requirements

1. **Replay protection**
   - Nonce is single-use
   - Expired challenges are invalid
   - Challenge store prevents nonce reuse during TTL window

2. **Domain separation**
   - Auth challenge domain/tag must be distinct from governance transaction sighash domains
   - Prevents reusing auth signatures as protocol action signatures

3. **Key rotation safety**
   - Membership proof is point-in-time only
   - Membership must be refreshed periodically
   - Session validity should be shorter than membership refresh cadence

4. **Strict role binding**
   - Challenge includes intended role
   - A signature valid for one role must not authorize another role

## Known Limitations

1. RPC `getStatus` response shape may evolve (`cur_state` vs `current_state`), requiring decoder maintenance.
2. Upstream role coverage is currently partial; not all product roles may be resolvable yet.
3. Desktop-only auth is local authorization, not a complete governance execution guarantee.
4. If RPC is unavailable and cache is stale, authentication must fail closed.

## Test Cases

1. **Happy path**
   - Resolve role keys from status
   - Valid challenge signature from member key
   - Auth succeeds and role session is created

2. **Non-member key**
   - Valid signature but key absent from selected role set
   - Auth denied

3. **Replay attempt**
   - Reuse same nonce/signature
   - Auth denied

4. **Expired challenge**
   - Signature submitted after `expires_at`
   - Auth denied

5. **Role mismatch**
   - Key belongs to Role A, user attempts Role B
   - Auth denied

6. **RPC unavailable with fresh cache**
   - Auth uses cached key set and succeeds/fails deterministically

7. **RPC unavailable with stale/no cache**
   - Auth denied with explicit dependency error

8. **Unsupported signature format**
   - Adapter returns unhandled format
   - Auth denied with actionable message

## Module Structure (proposed)

Desktop (`desktop-app/src-tauri`):
- `application/authentication.rs` (flow orchestration)
- `infrastructure/asm_status_rpc.rs` (JSON-RPC + status decode)
- `infrastructure/challenge_verifier.rs` (challenge validation + signature verification)
- `domain/auth_session.rs` (session value types and guards)

Desktop frontend (`desktop-app/src`):
- `api/authentication.ts` (IPC bridge for auth commands)
- `contexts/auth-session-context.ts` (local auth state)
- UI gating in relevant screens/actions

This module split is guidance; implementation should follow existing project patterns when integrating.
