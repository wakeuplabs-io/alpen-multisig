# Spec: Proposal Creation — Signer Update

## Objective

Define the product and technical requirements for creating `signer_update` proposals with safe validation,
clear preview, and session-expiry reauthentication behavior.

This spec focuses on desktop UX + frontend validation + proposal creation flow contracts.

## Scope

### Included

- `signer_update` form requirements and validations.
- Preview requirements before signing/submission.
- Current signer set visibility requirements.
- Session validity requirements during preview and submission.
- Reauthentication popup behavior when session expired.
- Acceptance criteria and test coverage plan.

### NOT included

- New proposal types beyond `signer_update`.
- Protocol-level consensus/validity changes (SPS-50/SPS-51/SPS-65 remain source of truth).
- Backend signer-set synchronization architecture changes.
- Proposal approval/execution flow changes.

## Requirements Alignment

- **Signer safety**: prevent invalid signer set updates and accidental threshold misconfiguration.
- **Session-bounded writes**: proposal creation must happen under a valid authority-scoped session.
- **High-signal UX**: explicit, actionable validation and reauth prompts.

## Technical Requirements

### 1) Signer update form data model

Inputs:

- `seqNo`
- `title`
- `keysToAdd[]`
- `keysToRemove[]`
- `newThreshold`

All key inputs use compressed pubkey hex format (`02/03 + 32 bytes`, optional `0x` prefix).

### 2) Validation rules (blocking)

For `actionType = signer_update`, the following are required:

1. At least one effective change exists (add/remove not both empty after trim).
2. Keys must be valid compressed pubkey hex.
3. Duplicate keys are forbidden:
   - no duplicates inside `keysToAdd`
   - no duplicates inside `keysToRemove`
   - no cross-list duplicates (same key in add and remove)
4. `newThreshold` must be integer in `[1, 255]`.
5. `newThreshold` cannot be greater than resulting signer count:

```
resultingSigners = unique((currentSigners - keysToRemove) + keysToAdd)
newThreshold <= count(resultingSigners)
```

6. `seqNo` must be a non-negative integer.

### 3) Current signer visibility

The UI must show current multisig signer keys (read-only) and current threshold while composing
`signer_update`, so the signer can reason about removals/additions safely.

Required display:

- Current signer list (full key, monospace, copy-friendly).
- Current threshold.
- Optional computed helper: resulting signer count.

### 4) Preview requirements (before submit)

A preview step is mandatory before final submit/sign.

Preview must include:

- Title
- Action type (`signer_update`)
- Sequence number
- Keys being added
- Keys being removed
- New threshold
- Resulting signer count
- Computed sighash (the exact hash being signed)

Submit remains disabled if form is invalid.

### 5) Session validity and reauthentication UX

Proposal creation and preview are auth-scoped operations and must enforce valid orchestrator session.

Rules:

1. If session is valid, continue normally.
2. If session is expired/missing when user clicks `Preview` or `Create & sign`:
   - stop the action
   - show a blocking popup/modal: `Session expired. Re-authenticate to continue.`
   - offer CTA: `Re-authenticate`
3. After successful reauth:
   - restore user to proposal flow without losing form data
   - retry the interrupted action (preview or submit) once
4. If reauth fails:
   - keep user in form state
   - show high-signal error and allow retry.

## Error Message Requirements

Errors should be specific and actionable. Minimum messages:

- `Duplicate signer key`
- `Signer key must be compressed pubkey hex (33 bytes, 02/03..., optional 0x)`
- `Provide at least one signer key to add or remove`
- `Threshold must be an integer between 1 and 255`
- `Threshold cannot be greater than the number of signers after this update`
- `Session expired. Re-authenticate to continue.`

## Acceptance Criteria

1. **No duplicates accepted**
   - Given duplicated key in add/remove inputs
   - When validating or submitting
   - Then operation is blocked with duplicate error.

2. **Current keys are visible**
   - Given loaded multisig config
   - When opening signer update form
   - Then current signer keys and current threshold are visible.

3. **Threshold bound is enforced**
   - Given threshold > resulting signer count
   - When validating form
   - Then operation is blocked with threshold-bound error.

4. **Preview is complete**
   - Given valid inputs
   - When entering preview
   - Then preview contains full signer delta + resulting threshold/count + sighash.

5. **Session-expiry popup is enforced**
   - Given expired/missing session
   - When attempting preview or submit
   - Then a reauth popup is shown and action does not continue until reauth succeeds.

6. **Form state is preserved across reauth**
   - Given user has filled signer update fields
   - When session expires and user reauthenticates
   - Then entered form values remain intact.

## Test Plan

### Frontend unit/integration

1. Duplicate detection in `keysToAdd`.
2. Duplicate detection in `keysToRemove`.
3. Cross-list duplicate detection.
4. Threshold > resulting signers rejected.
5. Current signer list renders when config loaded.
6. Preview contains all required signer update fields.
7. Expired session triggers reauth popup for preview.
8. Expired session triggers reauth popup for submit.
9. Successful reauth retries intended action once.
10. Failed reauth keeps form state and shows error.

### Backend contract checks (existing behavior compatibility)

11. `create proposal` remains authority-scoped by authenticated session.
12. Unauthorized/expired session response remains uniform and non-leaking.

## Rollout Notes

1. Land frontend validation and preview completeness first.
2. Add reauth popup + retry behavior next.
3. Verify no regressions in existing create proposal and session flows.
