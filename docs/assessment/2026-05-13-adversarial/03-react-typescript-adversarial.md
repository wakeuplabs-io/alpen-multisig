# React/TypeScript Frontend — Adversarial Assessment

**Date:** 2026-05-13  
**Scope:** `desktop-app/src/` — All TypeScript/TSX React 18 frontend code  
**Stance:** Adversarial. Treat every UI/state inconsistency as a potential signer safety failure.  
**Rubrics:** react-code-audit SKILL, react-frontend-patterns.md, typescript-standards.md, AGENTS.md conventions

---

## Scope & Threat Model: What We're Trying to Break

### Attack Surface
- **Signer authority context leakage** — Can a signer from one multisig see, infer, or interact with another's proposals?
- **Optimistic state poisoning** — Can stale, cached, or out-of-sync UI state trick a signer into signing the wrong transaction?
- **Missing confirmation context** — Does the UI clearly show what authority/proposal/change the signer is about to confirm before hardware wallet prompt?
- **Type safety at IPC boundary** — Is Tauri-bridge result data validated, or could a compromised/hijacked backend inject malicious payloads?
- **Session/auth races** — Can an attacker force re-authentication, role switching, or wallet disconnection mid-signing?
- **Copy/paste foot-guns** — Are sighashes, signatures, raw tx data properly labeled and validated on paste-in?
- **Route guard bypasses** — Can deep-linking, history manipulation, or state refresh bypass auth checks?
- **Effect dependency and stale closures** — Can useEffect deps or closure-captured values trigger unintended re-auth or data refetches with old authority context?

### Key Threat Scenarios
1. **Authority swap during signing** — Signer switches roles mid-proposal; old context remains in UI.
2. **Backend provides wrong sequencer pubkey** — Signer approves a proposal with stale/hijacked sequencer verification key.
3. **IPC result not validated** — Backend returns `proposalStatus: unknown` (not a valid ProposalStatus enum); UI treats it as valid.
4. **Session token replayed across authorities** — Auth token is not scoped to exact multisig; signer can use it for different authority.
5. **Sighash swapped in preview** — Signer reviews proposal A, but when they click "Sign", sighash is computed for proposal B.

---

## Top Findings (Ranked) — Blocking/High | Medium | Low

### **BLOCKING: D1 — No runtime validation of IPC results (Type assertion gap at Tauri boundary)**

**Severity:** BLOCKER  
**Location:** `desktop-app/src/api/tauri-bridge.ts:11–17`  
**Evidence:**
```
11 | export async function tauriCall<T>(command: string, args?: Record<string, unknown>): Promise<ApiResult<T>> {
12 |   try {
13 |     const data = await invoke<T>(command, args)
14 |     return { ok: true, data }
15 |   } catch (err) {
15 |     return { ok: false, error: err instanceof Error ? err.message : String(err) }
```

**Problem:**
- Result type `T` is **never validated** against actual backend response. Tauri's `invoke<T>` only casts the type without runtime checks.
- Backend (Rust Tauri) can return any JSON; TypeScript just trusts the shape.
- Example: `Proposal` type expects `status: 'pending' | 'approved' | ...`, but backend sends `status: 'unknown'` → TypeScript allows it, React renders as valid.
- No Zod/runtime guard at the boundary.

**Signer Risk:**
- Signer sees a proposal with fabricated status (e.g., `status: 'executed'`), believes it's safe to broadcast again.
- Proposal signatures array is untrusted; could be empty when signer thought there were N signatures.
- Sequence number, authority name could be spoofed by a compromised backend.

**Why This Breaks Signer Safety:**
SPS-50/SPS-65 require the **frontend to never trust backend data for protocol validity**. Backend can only coordinate; it cannot redefine what `pending`, `approved`, or `enacted` mean. Yet we accept unvalidated shapes from it.

**Fix:**
- Wrap all IPC results in runtime validators (Zod schemas).
- Example: `tauriCall` should validate result against a schema before returning.
- For Proposal: validate `status` is one of the allowed values, signatures array is non-empty if `status !== 'pending'`.

**Smallest Fix:**
```typescript
// desktop-app/src/api/tauri-bridge.ts
import { z } from 'zod'
const ProposalStatusSchema = z.enum(['pending', 'approved', 'enacted', 'canceled', 'expired'])

export async function tauriCall<T>(
  command: string, 
  args?: Record<string, unknown>, 
  schema?: z.ZodSchema<T>
): Promise<ApiResult<T>> {
  try {
    const data = await invoke<unknown>(command, args)
    if (schema) {
      const validated = schema.parse(data)
      return { ok: true, data: validated }
    }
    return { ok: true, data: data as T }
  } catch (err) {
    ...
  }
}
```

---

### **BLOCKING: D2 — No explicit authority scoping in session tokens; cross-authority token reuse risk**

**Severity:** BLOCKER  
**Location:** `desktop-app/src/api/orchestrator-auth.ts:13–18` and `desktop-app/src/contexts/session-provider.tsx:34–62`  
**Evidence:**

```typescript
// orchestrator-auth.ts:13–18
export type OrchestratorAuthSession = {
  token: string  // ← No explicit multisig/authority field; token is opaque
  authority: string
  signerPubkey: string
  expiresAtUnixMs: number
}
```

and

```typescript
// session-provider.tsx:34–62
const ensureOrchestratorSession = useCallback(async () => {
  const currentSession = await orchestratorAuthGetSession()
  if (!currentSession.ok) throw new Error(currentSession.error)
  if (currentSession.data !== null) {
    return  // ← Session is assumed valid for any authority; no re-auth if role changes
  }
  
  const challengeResult = await orchestratorAuthStart({
    baseUrl: ORCHESTRATOR_BASE_URL,
    authority: authorityFromRole(selectedRole),  // ← Challenge is scoped, but...
  })
  // Token is issued but never validated against selectedRole later
}, [adapter, selectedRole])
```

**Problem:**
- `OrchestratorAuthSession.token` is opaque; frontend has no way to verify it's scoped to the selected authority.
- If a signer switches roles mid-session (rare but possible), `ensureOrchestratorSession` sees a non-null session and **skips re-auth**.
- Backend could issue a token for `strata_admin` authority, but signer switches to `sequencer_manager` without re-auth.
- `listProposals()` uses the stale token for the wrong authority.

**Attack Scenario:**
1. Signer authenticates for `StrataAdministrator`.
2. Signer clicks through to dashboard; backend issues token T1 scoped to `strata_admin`.
3. Signer clicks "Switch to Sequencer Manager" (role selector UI).
4. Frontend re-calls `connectSession()`, but before that completes, code calls `ensureOrchestratorSession()`.
5. Old token T1 is still in memory; `ensureOrchestratorSession` sees non-null session and returns early.
6. `listProposals()` uses T1 (scoped to `strata_admin`) for sequencer queries → backend grants access to proposals from wrong authority.

**Why This Breaks Signer Safety:**
PRD §3 (Authority Isolation): *"A signer of one multisig authority MUST be treated as a non-signer with respect to all other multisig authorities."* If tokens aren't scoped or validated, this isolation breaks.

**Fix:**
- Add explicit `authority` or `authorityId` to the token JWT claims (backend concern).
- Frontend: Before using token, verify `session.authority === authorityFromRole(selectedRole)`.

**Smallest Fix:**
```typescript
// session-provider.tsx:34–48 (modified ensureOrchestratorSession)
const ensureOrchestratorSession = useCallback(async () => {
  const currentSession = await orchestratorAuthGetSession()
  if (!currentSession.ok) throw new Error(currentSession.error)
  
  const requiredAuthority = authorityFromRole(selectedRole)
  if (currentSession.data !== null && currentSession.data.authority === requiredAuthority) {
    return  // Token is valid for this authority
  }
  
  // If authority mismatch, force re-auth
  await orchestratorAuthLogout(ORCHESTRATOR_BASE_URL)
  // ... rest of auth flow ...
}, [adapter, selectedRole])
```

---

### **HIGH: D3 — Session invalidation race: role switch doesn't guarantee old tokens are purged before new auth**

**Severity:** HIGH  
**Location:** `desktop-app/src/contexts/auth-session-provider.tsx:35–41`  
**Evidence:**
```typescript
// auth-session-provider.tsx:35–41
useEffect(() => {
  if (session === null || session.role === selectedRole) {
    return  // ← If session.role !== selectedRole, logout happens async
  }
  setSession(null)
  void authLogout()  // ← Fire-and-forget; no await
}, [selectedRole, session])
```

**Problem:**
- When signer switches roles, `authLogout()` is NOT awaited.
- Old session token may still exist in backend memory (cookie, session store) during the race window.
- Frontend immediately calls `connectSession()` without waiting for `authLogout()` to complete.
- If two identity tokens (old role, new role) are both valid, backend cannot distinguish which signer is using which token.

**Race Condition:**
1. Signer authenticated as `StrataAdministrator` (token T1).
2. Signer switches role to `SequencerManager`.
3. `authLogout()` starts async (no await).
4. Frontend immediately calls `ensureOrchestratorSession()` → starts auth for `sequencer_manager`.
5. Meanwhile, backend still has T1 in the session store.
6. Signer might accidentally use T1 to list proposals for `sequencer_manager` if timing aligns.

**Why This Breaks Signer Safety:**
If a signer's old token is not invalidated server-side before new auth, token could leak across authority boundaries (e.g., via browser DevTools cache inspection, or if an attacker hijacks a session).

**Fix:**
- Make `authLogout()` async and await it before re-auth.

**Smallest Fix:**
```typescript
// auth-session-provider.tsx:35–41
useEffect(() => {
  if (session === null || session.role === selectedRole) return
  setSession(null)
  
  // Await logout before allowing new auth
  void (async () => {
    await authLogout()
  })()
}, [selectedRole, session])
```

---

### **HIGH: D4 — Sighash preview is computed asynchronously, but sighash used for signing is NOT re-validated; swap risk**

**Severity:** HIGH  
**Location:** `desktop-app/src/domain/create-proposal/components/create-proposal-form.tsx:124–138` and `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:110–145`  
**Evidence:**

In form component (lines 124–138):
```typescript
async function handlePreviewClick() {
  const isValid = await trigger(undefined, { shouldFocus: true })
  if (!isValid) return
  try {
    const sighashHex = await onPreviewValid(getValues())  // ← Compute sighash from current form values
    if (sighashHex === null) return
    setPreviewSighashHex(sighashHex)
    setIsPreviewMode(true)
  } catch (error) {
    // ...
  }
}
```

Then in the hook (lines 110–145):
```typescript
async function submitCreateProposal(formData: CreateProposalFormValues) {
  // ...
  const seqNo = Number(formData.seqNo.trim())  // ← Form values could have changed!
  
  // ...
  const actionHex = await buildActionHex(formData)  // ← Recomputed from potentially different form values
  const sighashResult = await computeSighash(seqNo, actionHex)  // ← NEW sighash computed here
  if (!sighashResult.ok) throw new Error(sighashResult.error)
  
  const sig = await adapter.signSighash(sighashResult.data.sighashHex)  // ← Signs NEW sighash, not preview one
```

**Problem:**
1. Signer clicks "Preview" → form values captured, sighash S1 computed and displayed.
2. Signer reviews sighash S1 on screen; matches expectations.
3. Signer clicks "Back to new proposal" (line 182 in form).
4. **Signer secretly edits the "threshold" field** (form is now in edit mode, keysToAdd/keysToRemove could be edited).
5. Signer clicks "Sign and Create Proposal" (line 329).
6. `submitCreateProposal` captures **NEW form values**, recomputes sighash S2 ≠ S1.
7. Trezor prompts signer with S2, but signer's mental model was S1 (from preview).
8. Signer approves S2 without noticing the change → signs a different proposal than intended.

**Diagram:**
```
Preview Screen:
  Sighash: 0x1234... (proposal: change threshold to 2)

User navigates back, edits form:
  Now: change threshold to 1
  
Sign Screen:
  Sighash: 0x5678... (proposal: change threshold to 1, but signer expects 0x1234...)
  
Trezor shows: 0x5678...
Signer sees different hex, might not notice.
```

**Why This Breaks Signer Safety:**
SPS-65 requires signers to verify the proposal content on the hardware wallet screen. If the form can be modified between preview and signing, and the sighash is recomputed without showing the signer the change, we violate the explicit-confirmation principle.

**Fix:**
- Capture form values at preview time, freeze them for signing, OR
- Re-display the sighash before signing with a "Has this changed?" check.

**Smallest Fix:**
```typescript
// In form component (create-proposal-form.tsx):
const [frozenPreviewData, setFrozenPreviewData] = useState<CreateProposalFormValues | null>(null)

async function handlePreviewClick() {
  // ...
  const sighashHex = await onPreviewValid(getValues())
  setFrozenPreviewData(getValues())  // ← Freeze form values
  setIsPreviewMode(true)
}

// In submit:
async function handleSubmitAttempt(data: CreateProposalFormValues) {
  if (isPreviewMode && frozenPreviewData) {
    // Verify form hasn't changed since preview
    if (JSON.stringify(data) !== JSON.stringify(frozenPreviewData)) {
      setError('Form was modified after preview. Re-preview to confirm changes.')
      return
    }
  }
  await onSubmitValid(data)
}
```

---

### **HIGH: D5 — Authority label never displayed in signing confirmation on Trezor; signer can't verify which multisig they're signing for**

**Severity:** HIGH  
**Location:** `desktop-app/src/domain/sign-proposal/components/sign-proposal-view.tsx:43–133`  
**Evidence:**

The component shows:
```typescript
// Line 44–66: Proposal card displays authority and proposalIdLabel
<div className="rounded-xl border border-[#f1f5f9] bg-[#fcfcff] p-4">
  <p className="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">Proposal</p>
  <h2 className="m-0 mt-2 font-['BIZ_UDPMincho'] text-[31px] leading-[1.12] text-[#0a0a0a]">{proposalTitle}</h2>
  <p className="m-0 mt-2 text-xs text-[#6b7280]">
    {proposalIdLabel} <span className="mx-1.5 text-[#d1d5db]">•</span> {authorityLabel}{' '}
    <span className="mx-1.5 text-[#d1d5db]">•</span> {proposalTypeLabel}
  </p>
</div>
```

But then at line 72–86:
```typescript
<div className="mt-5">
  <p className="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#9ca3af]">
    SPS-65 Sighash (32 bytes)
  </p>
  <div className="mt-2 flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f8fafc] px-3 py-2.5">
    <code className="block min-w-0 flex-1 break-all font-mono text-[12px] leading-5 text-[#334155]">
      {sighashHex}  // ← Only sighash shown; no authority info embedded
    </code>
```

And the Trezor prompt instruction (line 94–98):
```typescript
<div className="mt-4 rounded-lg border border-[#e5e7eb] bg-[#fafaff] p-3.5">
  <div className="flex items-start gap-2.5">
    <div className="mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-[#ddd8ff] bg-[#f5f3ff] text-[#7c6fcd]">
      <UsbSessionDefaultIcon width={13} height={13} className="text-[#7c6fcd]" />
    </div>
    <div>
      <p className="m-0 text-sm font-medium text-[#111827]">Connect your Trezor and confirm on device</p>
      <p className="m-0 mt-1 text-[12px] text-[#6b7280]">
        The sighash above appears on the device screen. Verify it matches before approving.
      </p>  // ← No mention that signer must also verify they're signing for the right authority
    </div>
  </div>
</div>
```

**Problem:**
- Signer sees `authorityLabel` on the React screen but hardware wallet (Trezor) does NOT display it.
- Trezor only shows sighash (computed as `SHA256(tag || seqno || payload)`).
- The `tag` includes the action type (e.g., "strata/admin/signer_update"), but Trezor firmware displays it in hex, not human-readable.
- Signer can copy-paste a sighash from one authority to another; if they mix up authorities (e.g., sign for `strata_admin` when they meant `sequencer_manager`), they won't see the difference on Trezor.

**Attack Scenario:**
1. Signer intends to sign a `StrataAdministrator` proposal.
2. React UI shows: "Authority: Strata Administrator".
3. Signer's Trezor is slow; they get impatient and switch to another app.
4. Another proposal for `SequencerManager` appears; UI shows "Authority: Sequencer Manager".
5. Signer clicks "Sign"; Trezor prompts with sighash S2.
6. Signer mistakenly thinks they're still signing the original `StrataAdministrator` proposal.
7. Trezor does NOT show "Sequencer Manager" in plain text; signer approves S2 for wrong authority.

**Why This Breaks Signer Safety:**
PRD §8: *"The user MUST be able to clearly read and understand each message they are signing on their hardware wallet screen."* If authority is not embedded in the sighash or displayed on Trezor, signer cannot verify they're signing for the intended multisig.

**Fix:**
- (Backend/Protocol level) Include authority identifier in the sighash payload or tag.
- (Frontend level) Display a warning: "Verify that your Trezor shows the same authority/sighash before approving."

**Smallest Fix (Frontend):**
```typescript
// sign-proposal-view.tsx:94–101
<div className="mt-4 rounded-lg border border-[#e5e7eb] bg-[#fafaff] p-3.5">
  <div className="flex items-start gap-2.5">
    <div>
      <p className="m-0 text-sm font-medium text-[#111827]">Connect your Trezor and confirm on device</p>
      <p className="m-0 mt-1 text-[12px] text-[#6b7280]">
        ⚠️ <strong>VERIFY on your Trezor:</strong> The sighash matches above AND your device shows 
        this is for <strong>{authorityLabel}</strong> (not a different multisig).
      </p>
    </div>
  </div>
</div>
```

---

### **HIGH: D6 — No deep-link protection; attacker can navigate signer directly to `/proposals/:actionId/sign` without prior route context**

**Severity:** HIGH  
**Location:** `desktop-app/src/screens/sign-poc-screen.tsx` and `desktop-app/src/App.tsx:57–63`  
**Evidence:**

App.tsx routing (lines 57–63):
```typescript
<Route
  path="/proposals/:actionId/sign"
  element={
    <RequireAuth>
      <SignPocScreen />
    </RequireAuth>
  }
/>
```

RequireAuth only checks `isAuthenticated` (line 14–22), not whether the signer has accessed the dashboard or is in a valid proposal-context.

**Problem:**
- `RequireAuth` only checks if user is logged in; doesn't verify they're in a valid proposal workflow.
- Attacker (or attacker-controlled URL) can send signer a link: `app://proposals/deadbeef/sign`.
- Signer clicks link; `SignPocScreen` loads.
- `SignPocScreen` tries to fetch proposal `deadbeef` from backend.
- If backend is compromised, it could return a **fabricated proposal** for a different authority than signer is signed into.
- Signer sees a legitimate-looking proposal and signs it without realizing it's for the wrong authority.

**Why This Breaks Signer Safety:**
Signer should never be one click away from signing without a series of confirmations and context checks. Deep-linking bypasses the dashboard's workflow and allows an attacker to inject a proposal context.

**Fix:**
- Validate that the proposal being signed matches the currently authenticated authority.
- Require navigation flow (signer must go through dashboard → select proposal → sign, not direct link).

**Smallest Fix:**
```typescript
// sign-poc-screen.tsx (modified to validate authority context)
export function SignPocScreen() {
  const { selectedRole } = useSession()
  const { actionId } = useParams<{ actionId: string }>()
  
  useEffect(() => {
    if (!actionId) return
    
    async function validateProposal() {
      const proposal = await getProposalByActionId({ baseUrl: ORCHESTRATOR_BASE_URL, actionId })
      if (!proposal.ok) {
        setError('Proposal not found or access denied.')
        return
      }
      
      // Verify proposal authority matches selected role
      const requiredAuthority = authorityFromRole(selectedRole)
      if (proposal.data.authority !== requiredAuthority) {
        setError('This proposal is for a different authority. Please switch roles or select the correct multisig.')
        return
      }
      
      // Proposal is valid for this authority; proceed
      setProposal(proposal.data)
    }
    
    void validateProposal()
  }, [actionId, selectedRole])
  
  // ...
}
```

---

### **MEDIUM: D7 — Session warning threshold is 5 minutes; no persistent re-auth prompt**

**Severity:** MEDIUM  
**Location:** `desktop-app/src/contexts/session-provider.tsx:28–32`  
**Evidence:**
```typescript
const remainingMs = Math.max(0, (session?.expiresAtUnixMs ?? 0) - nowMs)
const min = Math.floor(remainingMs / 60_000)
const sec = Math.floor((remainingMs % 60_000) / 1_000)
const sessionTimeLabel = session ? `${String(min).padStart(2, '0')}:${String(sec).padStart(2, '0')}` : '--:--'
const sessionWarning = session !== null && min < 5
```

**Problem:**
- Session warning is True when < 5 min remaining.
- But the UI doesn't block signing; signer can still click "Sign" with 4:59 left.
- If signing takes 2 min and session expires mid-signing, backend rejects the proposal.
- No reactive re-auth prompt; signer must manually re-authenticate.

**Impact:**
- Annoying UX; signer may lose work.
- NOT a signer-safety blocker (backend correctly rejects expired session).

**Fix:**
- Disable signing buttons when `sessionWarning` is true, with a re-authenticate prompt.

---

### **MEDIUM: D8 — Form validation allows empty key rows; confusing UX for signer-update proposals**

**Severity:** MEDIUM  
**Location:** `desktop-app/src/domain/create-proposal/model/create-proposal.schema.ts:46–52`  
**Evidence:**
```typescript
if (data.keysToAdd.length < 1) {
  ctx.addIssue({ code: 'custom', path: ['keysToAdd'], message: 'At least one row for keys to add' })
}
if (data.keysToRemove.length < 1) {
  ctx.addIssue({ code: 'custom', path: ['keysToRemove'], message: 'At least one row for keys to remove' })
}
```

Then lines 70–80:
```typescript
if (data.actionType === 'signer_update') {
  const normalizedAdds = data.keysToAdd.map((row) => row.value.trim()).filter((value) => value.length > 0)
  const normalizedRemoves = data.keysToRemove.map((row) => row.value.trim()).filter((value) => value.length > 0)
  
  if (normalizedAdds.length === 0 && normalizedRemoves.length === 0) {
    ctx.addIssue({
      code: 'custom',
      path: ['keysToAdd'],
      message: 'Provide at least one signer key to add or remove',
    })
  }
}
```

**Problem:**
- Validation requires ≥1 row in `keysToAdd` and ≥1 row in `keysToRemove`, but rows can be **empty strings**.
- Signer sees two input fields, thinks they must fill both, but actually only needs one non-empty field in either.
- This is not a signer-safety risk per se, but poor UX could lead to accidental proposal creation (e.g., blank proposal).

**Fix:**
- Simplify validation: require ≥1 non-empty row across keysToAdd OR keysToRemove, not both arrays.

---

### **MEDIUM: D9 — No explicit error recovery for signature verification failures; generic error toast**

**Severity:** MEDIUM  
**Location:** `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:110–145`  
**Evidence:**
```typescript
async function submitCreateProposal(formData: CreateProposalFormValues) {
  setError(null)
  setIsSubmitting(true)
  try {
    // ...
    const sig = await adapter.signSighash(sighashResult.data.sighashHex)
    // ← No validation that sig.publicKeyHex matches a signer in the multisig!
    
    const createResult = await createProposal({
      baseUrl: ORCHESTRATOR_BASE_URL,
      seqNo,
      actionHex,
      signerPubkey: sig.publicKeyHex,
      signatureHex: sig.signatureHex,
    })
    if (!createResult.ok) throw new Error(createResult.error)
  } catch (err) {
    if (isSessionExpiredReauthError(err)) {
      throw err
    }
    setError(String(err))  // ← Generic error; no hint if it's signature validation failure
  }
}
```

**Problem:**
- Backend returns error like: "Invalid signature or signer not in multisig" (from backend validation).
- Frontend just displays: "Error: Invalid signature or signer not in multisig".
- Signer doesn't know if the error is from Trezor (signature refused), network, or signer not in multisig.
- No retry guidance.

**Impact:**
- Poor UX; signer is confused.
- NOT a signer-safety blocker (backend correctly validates).

**Fix:**
- Parse specific error codes from backend; display contextual guidance.

---

### **MEDIUM: D10 — Orchestrator auth challenge is not bound to wallet address; signer could mix wallet accounts**

**Severity:** MEDIUM  
**Location:** `desktop-app/src/contexts/session-provider.tsx:51–58`  
**Evidence:**
```typescript
const signature = await adapter.signSighash(challengeResult.data.challengeHex)
const completeResult = await orchestratorAuthComplete({
  baseUrl: ORCHESTRATOR_BASE_URL,
  challengeId: challengeResult.data.challengeId,
  signerPubkey: signature.publicKeyHex,  // ← Public key from Trezor signature, not validated against connected wallet
  signatureHex: signature.signatureHex,
  signatureFormat: signature.signatureFormat,
})
```

**Problem:**
- Signer connects wallet (Ledger/Trezor) at address A (say, index 0).
- Frontend has `wallet.address = A`.
- Signer starts orchestrator auth; Trezor prompts to sign.
- Signer manually switches Trezor to address B (index 1) and signs.
- Frontend receives `publicKeyHex` from address B, sends it to backend.
- Backend has no way to know that B was not the intended signer.
- If B is also a signer in the multisig (but different authority or group), backend might grant access.

**Fix:**
- Validate that `signature.publicKeyHex` matches the connected wallet address before sending to backend.

**Smallest Fix:**
```typescript
// session-provider.tsx:51–83
const signature = await adapter.signSighash(challengeResult.data.challengeHex)

// Verify signature is from the connected wallet
if (wallet && signature.publicKeyHex !== wallet.publicKeyHex) {
  throw new Error('Signature does not match connected wallet address. Please confirm on your hardware wallet.')
}

const completeResult = await orchestratorAuthComplete({
  // ...
})
```

---

### **LOW: D11 — Proposal type inference relies on heuristics; could misidentify proposal type**

**Severity:** LOW  
**Location:** `desktop-app/src/domain/proposals-dashboard/components/proposals-dashboard.tsx:338–346`  
**Evidence:**
```typescript
function inferProposalType(proposal: Proposal): string {
  if (proposal.authority.toLowerCase().includes('sequencer')) {
    return 'Sequencer update'
  }
  if (proposal.actionHex.toLowerCase().startsWith('0x01')) {
    return 'Verification key update'
  }
  return 'Signer update'
}
```

**Problem:**
- Proposal type is inferred from `authority` string and `actionHex` prefix, not from backend metadata.
- If authority string is `'sequencer_manager'` but `actionHex` is `0x01...`, it returns `'Sequencer update'`.
- Backend should explicitly send proposal type; frontend shouldn't guess.

**Impact:**
- Low: Signer sees wrong label, but the actual proposal content is still accurate. Not a signer-safety blocker.

**Fix:**
- Add `proposalType` field to `Proposal` type; backend provides it explicitly.

---

### **LOW: D12 — No explicit "you are switching roles" confirmation dialog**

**Severity:** LOW  
**Location:** `desktop-app/src/contexts/auth-session-provider.tsx:35–41` (and ProposalsDashboardScreen role selector UI)  
**Evidence:**
```typescript
useEffect(() => {
  if (session === null || session.role === selectedRole) {
    return
  }
  setSession(null)
  void authLogout()  // ← Silent logout, no confirmation
}, [selectedRole, session])
```

**Problem:**
- Signer clicks role selector button; role changes instantly.
- No "Are you sure you want to switch roles? This will log you out." prompt.
- Signer could accidentally click the wrong role button.

**Impact:**
- Low: Signer can just click back and re-auth. Not a signer-safety blocker.

**Fix:**
- Add a confirmation modal before role switch.

---

## Attack Narratives (3–6): "How This Fails in Production / for a Signer / for Maintainers"

### **Scenario 1: Cross-Authority Token Leakage (D2 + D3)**

**Setup:** Two signers: Alice (Strata Admin) and Bob (Sequencer Manager). Same hardware wallet seed (different derivation index).

**Attack Flow:**
1. Alice authenticates as `StrataAdministrator`; backend issues token T_A scoped to `strata_admin`.
2. Alice opens dashboard, sees her proposals.
3. Alice has a system hiccup; browser tab crashes and restarts.
4. Session token T_A is still in localStorage / browser memory.
5. Alice tries to switch to `SequencerManager` (or is tricked into clicking the button by an attacker's phishing message).
6. `authLogout()` is called async (no await); frontend immediately calls `ensureOrchestratorSession()` for sequencer manager.
7. Old token T_A is still valid in backend's session store for ~30s (grace period before truly expiring server-side).
8. Frontend uses T_A to list proposals; backend interprets T_A as scoped to `strata_admin` (not `sequencer_manager`).
9. Alice (thinking she's in Sequencer Manager view) sees Strata Admin proposals.
10. Alice signs a Strata Admin proposal while believing she's in Sequencer Manager context.
11. Blockchain validation fails (wrong authority), but Alice has already exposed her signer address to the backend as a Sequencer Manager (privacy loss).

**Why This Happens:**
- No explicit authority scoping in token.
- Logout is async, not awaited; old session persists during race window.
- Frontend doesn't validate `session.authority === selectedRole` before using the token.

**Signer Impact:** Identity confusion, potential wrong-authority signature, privacy leak.

---

### **Scenario 2: Sighash Swap Between Preview and Sign (D4)**

**Setup:** Signer intends to create a proposal that removes a signer and increases threshold to 3. Preview shows sighash 0x1234...

**Attack Flow:**
1. Signer fills form: "Remove key XXX, add key YYY, set threshold to 3."
2. Signer clicks "Preview and Create."
3. Frontend computes sighash 0x1234... (for the above proposal), displays it.
4. Signer reviews; hex matches their expectation.
5. Signer clicks "Back to new proposal" (React preview mode exits, edit mode re-enters).
6. Signer notices "threshold" field still shows "3"; accidentally edits it to "2" (typo/glitch).
7. Signer clicks "Sign and Create Proposal" again.
8. Frontend NOW recomputes sighash from the MODIFIED form → sighash 0x5678... (threshold = 2, not 3).
9. Trezor prompts with 0x5678...
10. Signer's mental model: "I'm signing for threshold 3."
11. Signer approves 0x5678... without re-reviewing (assumes it's the same as preview).
12. Backend receives signature for "threshold = 2" proposal.
13. Proposal is created with wrong threshold; governance broken (quorum unreachable with threshold 2 if there are only 2 signers left).

**Why This Happens:**
- Preview form values are not frozen; signer can edit form after preview.
- Sighash is recomputed without re-showing signer the change.
- No "form changed since preview" warning.

**Signer Impact:** Unintended proposal, governance failure, need for emergency re-vote.

---

### **Scenario 3: Authority Mismatch via Deep Link (D6)**

**Setup:** Attacker knows signer's app is at address `http://localhost:5173`. Attacker crafts a malicious deeplink.

**Attack Flow:**
1. Attacker sends signer a message: "Click here to quickly sign the pending Strata proposal: `app://proposals/aaaa1234/sign`"
2. Signer, expecting a legitimate internal link, clicks it.
3. Deep link bypasses the dashboard workflow; `SignPocScreen` loads for proposal `aaaa1234`.
4. `SignPocScreen` calls backend to fetch proposal `aaaa1234`.
5. Backend (if compromised or if attacker hijacked DNS) returns a fabricated proposal:
   - `actionId: 'aaaa1234'`
   - `authority: 'strata_admin'` (different from signer's selected role `sequencer_manager`)
   - `actionHex: '0xabcd...'` (sequencer update, not strata admin proposal)
6. Frontend displays proposal; no authority validation.
7. Signer sees "Sequencer update" label (inferred from actionHex) but authority says "strata_admin."
8. Signer approves on Trezor.
9. Signature is for the wrong authority; blockchain rejects it or it gets mixed with other pending proposals.

**Why This Happens:**
- Deep link bypasses dashboard workflow.
- `RequireAuth` doesn't validate proposal authority matches selected role.
- No authority mismatch check before signing.

**Signer Impact:** Signature leak, governance confusion, potential loss of voting power if signature ends up in wrong proposal.

---

### **Scenario 4: Unvalidated IPC Result Spoofing (D1)**

**Setup:** Backend (Tauri Rust process) has a bug or is compromised.

**Attack Flow:**
1. Signer loads dashboard.
2. Frontend calls `listProposals()` → IPC to Tauri backend.
3. Tauri returns JSON (attacker-controlled or buggy):
   ```json
   [
     {
       "actionId": "legit123",
       "status": "executed",
       "signatures": []
     }
   ]
   ```
4. Frontend casts this to `Proposal[]` with NO runtime validation.
5. UI renders proposal as "Executed" (status check passes, no error).
6. Signer thinks proposal is already done, doesn't sign it.
7. Later, backend tells signer (via out-of-band message) that the proposal actually needs their signature.
8. Signer is confused; they thought it was executed.

**OR:**

1. Backend returns `signatures` array with spoofed public keys (not real signers).
2. Frontend displays "2 / 3 signatures collected" when actually 0 valid signatures exist.
3. Signer thinks quorum is reached, broadcasts the proposal.
4. Blockchain rejects it (invalid signatures).

**Why This Happens:**
- No Zod/runtime schema validation on IPC results.
- TypeScript type casting doesn't validate shape.

**Signer Impact:** Confusion, wasted signatures, failed proposals, loss of trust in app.

---

### **Scenario 5: Session Expiry Allows Stale Sighash Signing (D9 + contextual UX failure)**

**Setup:** Signer starts signing at 14:58 (session expires at 15:00, i.e., 2 min left).

**Attack Flow:**
1. Signer navigates to `/proposals/:actionId/sign` at 14:58.
2. Frontend fetches proposal, displays sighash.
3. Signer connects Trezor (takes 30s, now 14:58:30).
4. Trezor prompts to sign; signer takes time reviewing (1 min, now 14:59:30).
5. Signer approves on Trezor (now 15:00:15 — **session has expired**).
6. Frontend sends signature to backend.
7. Backend has cached the session and still recognizes the token, but logs: "Session expired during signing."
8. Backend rejects the signature submission.
9. Frontend catches the error; displays generic message: "Error: Session expired."
10. Signer must re-auth and re-sign; loses the sighash, must re-compute (and verify again on Trezor).

**Why This Happens:**
- No session expiry warning during active signing.
- Session countdown is visible in UI but not blocking/prompting.
- No error recovery guidance.

**Signer Impact:** Poor UX, wasted time, annoyance.

---

### **Scenario 6: Role Switching During Proposal Creation (D3 + D5)**

**Setup:** Signer starts creating a Strata Admin proposal but switches roles mid-process.

**Attack Flow:**
1. Signer is in `StrataAdministrator` role, fills out a signer-update proposal form.
2. Signer clicks "Preview and Create."
3. Form is valid; frontend calls `ensureOrchestratorSession()` for `strata_admin`.
4. Authorization successful; backend issues token T_A for `strata_admin`.
5. Frontend resets form to default (line 34–42 of create-proposal-form.tsx).
6. Signer accidentally clicks the role selector button (or is tricked into clicking).
7. Role changes to `SequencerManager`.
8. `authLogout()` is called async (D3).
9. Before logout completes, signer quickly navigates back to create-proposal form.
10. Form is still displaying "Strata Administrator" at the top (cached from context).
11. Signer fills out a NEW proposal thinking they're in Strata Admin role.
12. Signer clicks "Preview and Create."
13. Frontend calls `ensureOrchestratorSession()` for `sequencer_manager`; old token T_A is still partially valid.
14. Frontend might reuse T_A (race condition); backend interprets proposal as being for `strata_admin` (wrong authority).
15. Signer gets an error or wrong proposal is created.

**Why This Happens:**
- Async logout doesn't block role-switch workflow.
- Form state doesn't re-render authority labels reactively when role changes.
- No explicit "you've switched roles, form has been cleared" prompt.

**Signer Impact:** Confusion, wrong proposal, need for manual cleanup.

---

## Evidence Index (Paths)

All findings are directly citable in the codebase:

1. **D1 (IPC validation gap):** `desktop-app/src/api/tauri-bridge.ts:11–17`
2. **D2 (No authority scoping in tokens):** `desktop-app/src/api/orchestrator-auth.ts:13–18` and `desktop-app/src/contexts/session-provider.tsx:34–62`
3. **D3 (Session invalidation race):** `desktop-app/src/contexts/auth-session-provider.tsx:35–41`
4. **D4 (Sighash swap):** `desktop-app/src/domain/create-proposal/components/create-proposal-form.tsx:124–138` and `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:110–145`
5. **D5 (Authority not on Trezor):** `desktop-app/src/domain/sign-proposal/components/sign-proposal-view.tsx:43–133`
6. **D6 (Deep-link bypass):** `desktop-app/src/screens/sign-poc-screen.tsx` and `desktop-app/src/App.tsx:57–63`
7. **D7 (Session warning <5 min, no blocking):** `desktop-app/src/contexts/session-provider.tsx:28–32`
8. **D8 (Empty key rows allowed):** `desktop-app/src/domain/create-proposal/model/create-proposal.schema.ts:46–80`
9. **D9 (Generic error on signature failure):** `desktop-app/src/domain/create-proposal/hooks/use-create-proposal.ts:110–145`
10. **D10 (Wallet address not validated):** `desktop-app/src/contexts/session-provider.tsx:51–58`
11. **D11 (Proposal type heuristic):** `desktop-app/src/domain/proposals-dashboard/components/proposals-dashboard.tsx:338–346`
12. **D12 (No role-switch confirmation):** `desktop-app/src/contexts/auth-session-provider.tsx:35–41` (implicit; UI component not shown)

---

## Smallest Fixes vs Largest Bets (Be Explicit)

### **Smallest Fixes (Quick Wins)**

1. **D12 (Role-switch confirmation):** Add modal before role change. ~50 lines. No backend changes.
2. **D8 (Empty key row validation):** Simplify Zod schema to require ≥1 non-empty row (not both arrays). ~10 lines change.
3. **D11 (Explicit proposal type):** Add `proposalType` field to backend `Proposal` response. ~5 lines frontend, ~10 lines backend.
4. **D7 (Session warning disable):** Disable sign buttons when `sessionWarning` is true. ~20 lines.

### **Medium-Size Fixes**

1. **D4 (Freeze preview data):** Capture form values at preview; validate no change before signing. ~50 lines.
2. **D5 (Authority warning on Trezor):** Enhance sign-proposal-view.tsx warning message. ~10 lines.
3. **D10 (Validate wallet address):** Add pubkey check before sending signature to backend. ~15 lines.

### **Largest Bets (Requires Coordination)**

1. **D1 (IPC validation):** Add Zod schemas for all IPC result types. ~200 lines (tauriCall wrapper + schema definitions). Blocks many other fixes.
2. **D2 (Authority scoping in tokens):** Backend must embed authority in token JWT claims; frontend must validate. ~100 lines frontend, ~50 lines backend.
3. **D3 (Await logout before re-auth):** Refactor auth flow to await logout. ~30 lines, but touches critical path.
4. **D6 (Deep-link protection):** Add proposal-authority validation before signing. ~50 lines frontend + backend endpoint check.
5. **D9 (Error recovery for signatures):** Parse backend error codes; show contextual guidance. ~80 lines frontend.

### **Riskiest Fix (Highest Payload)**

- **D1 (IPC validation)**: Requires adding Zod to every API call. Risk: Over-validation could break legitimate backend responses if backend is in transition. Payoff: Prevents entire class of spoofing attacks.

### **Simplest High-Impact Fix**

- **D2 (Authority validation in ensureOrchestratorSession)**: Add one line:
  ```typescript
  if (currentSession.data !== null && currentSession.data.authority === requiredAuthority) {
    return
  }
  ```
  Payoff: Blocks cross-authority token reuse. No backend changes needed.

---

## What Would Change My Mind (Missing Evidence / Experiments)

1. **Evidence that authority is already embedded in JWT:** If backend JWT includes authority and we missed it, D2/D3 severity drops to LOW. **Action:** Inspect `orchestratorAuthGetSession()` response; decode JWT payload.

2. **Evidence that Trezor firmware auto-validates seqno/action-type:** If Trezor shows the sighash tag or authority, D5 severity drops to MEDIUM. **Action:** Test with actual Trezor; review SPS-65 sighash tag encoding.

3. **Evidence of existing runtime validation wrapper:** If a Zod schema layer already wraps `tauriCall`, D1 severity drops to MEDIUM (and findings become "incomplete validation"). **Action:** Grep for Zod usage in tauri-bridge or proposal.ts.

4. **Evidence that logout is awaited elsewhere:** If role-switch effect awaits `authLogout()` in a parent component, D3 severity drops to LOW. **Action:** Search for `await authLogout()`.

5. **Evidence of integration test for authority isolation:** If e2e tests verify that a signer of one multisig cannot access another's proposals, D2/D6 are already covered. **Action:** Check `alpen-multisig-e2e-tests/` for authority-scoping tests.

---

## Summary & Recommendations

### **Critical Path (Do First)**

1. **D1 (IPC validation)** — Add Zod schemas to tauri-bridge. Blocks D1 directly; reduces risk of all spoofing scenarios.
2. **D2 (Authority validation)** — Verify session authority before using. One-line fix; highest impact-per-LOC.
3. **D4 (Freeze preview)** — Validate form hasn't changed since preview before signing.
4. **D6 (Deep-link protection)** — Validate proposal authority before rendering sign screen.

### **Longer-Term Improvements**

- Embed authority explicitly in sighash tag (protocol-level, SPS-65 update).
- Add integration tests for authority isolation and cross-signer scenarios.
- Implement per-multisig session tokens (not just a single token for all authorities).
- Require explicit "confirm role switch" flow with session expiry messaging.

### **Testing Recommendations**

1. **Unit:** Authority validation in `useCreateProposal`, session-scoping in `SessionProvider`.
2. **Integration:** Cross-authority token reuse (two signers, try to swap tokens).
3. **E2E:** Role-switch mid-signing, deep-link to wrong-authority proposal, session expiry during signing.

---

**Report Status:** READ-ONLY AUDIT COMPLETE  
**Next Step:** Triage findings with team; prioritize fixes by severity and impact.
