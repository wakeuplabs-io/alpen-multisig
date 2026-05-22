# Rust Tauri (desktop-app/src-tauri) — Adversarial Assessment

## Scope & threat model (what we're trying to break)

- **IPC trust model**: Local webview invokes ~30 Rust commands (`desktop-app/src-tauri/src/main.rs`). Anything that reaches `invoke` can exercise Bitcoin RPC, orchestrator proxies, mnemonic paths, and operator keys if parameters are attacker-controlled from the renderer.
- **CSP / web isolation**: Effective XSS in the frontend becomes native-I/O capability against user machine and RPC credentials (`tauri.conf.json`).
- **Secrets path**: Mnemonic-derived signing, Trezor workflows, **`operator_secret_key_hex` and Bitcoin RPC passwords** passed through Tauri command args (`commands/proposals.rs` `BroadcastInput`).
- **Session durability**: Orchestrator bearer token sits in-process (`application/orchestrator_auth.rs`).
- **Truthfulness vs backend**: Commands must not misrepresent orchestrator/onchain outcomes to the signer.

## Top findings (ranked) — Blocking/High | Medium | Low

### Blocking / High

1. **`proposals_broadcast` fabricates success metadata.** After `broadcast_commit_then_reveal`, the command returns `BroadcastResultDto` with **hard-coded** `proposal_status: "enacted"` and `broadcast_status: "reveal_confirmed"` (`desktop-app/src-tauri/src/commands/proposals.rs`). It ignores the refreshed proposal snapshot from HTTP (unlike orchestrator Axum handler which reloads repo state). Any partial protocol mismatch or deserialization gap **lies to the UX** — direct signer-safety violation.

2. **`operator_secret_key_hex` crosses the IPC boundary from the renderer.** `BroadcastInput` includes operator key material typed from TypeScript (`desktop-app/src/api/proposals.ts`, `commands/proposals.rs`). A compromised renderer / malicious injected script invokes `proposals_broadcast` with exfil payloads. Combined with CSP disabled (below), XSS severity escalates to **key theft**, not “just” UI spoofing.

3. **CSP explicitly null (disabled).** `tauri.conf.json` sets `"csp": null` under `app.security`. Tauri relies on CSP to constrain renderer execution; **`null` is maximum XSS blast radius** for a wallet-adjacent app.

### Medium

4. **Large default `invoke_handler` surface.** Signing, mnemonic listing, orchestrator proxies, HW wallet helpers all share one privilege plane. No capability allowlist JSON found under `src-tauri` — any command callable from webview equally if reachable from JS bundle.

5. **Orchestrator session in global `Mutex` without encryption at rest.** `orchestrator_auth.rs` keeps bearer token cleartext in memory; acceptable for POC, weak for unattended machine / memory scrapers versus OS keychain.

6. **Bitcoin RPC defaults on backend mirror pattern risk in Tauri** — callers supply URLs/creds via IPC inputs; phishing UI could redirect `baseUrl`/RPC to attacker infra (social engineering + missing URL pinning).

### Low

7. **Error strings only** — Commands return `Result<_, String>`; no structured codes for programmatic UI branching beyond substring checks (`map_proposal_error` treats 401 specially).

## Attack narratives (3–6)

1. **XSS → Bitcoin drain / key exfil.** With CSP null, attacker triggers script injection in renderer (dependency supply chain, pasted HTML, malicious deep link handling). Script calls `broadcastProposal` with attacker `btcRpcUrl` and logs `operatorSecretKeyHex` from form state.

2. **Fake “enacted” confirmation.** Backend returns success but reconciliation bug or race leaves proposal `approved`; UI still reads hard-coded enacted from IPC result — signer believes settlement complete and stands down monitoring.

3. **Malicious packaged mod.** User installs patched binary replacing `invoke_handler` subset — binary integrity / code signing discussion rather than Rust bug; still affects threat model assumptions.

4. **Shared-machine session bleed.** Mutex-held orchestrator token; second OS user launches app within same compromised session — POC threat only if profiles shared.

## Evidence index (paths)

| Topic | Paths |
|-------|-------|
| Command surface | `desktop-app/src-tauri/src/main.rs` |
| Broadcast / operator key IPC | `desktop-app/src-tauri/src/commands/proposals.rs` (`BroadcastInput`, `proposals_broadcast`) |
| Orchestrator bearer storage | `desktop-app/src-tauri/src/application/orchestrator_auth.rs` |
| HTTP client to orchestrator | `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs` |
| CSP | `desktop-app/src-tauri/tauri.conf.json` |

## Smallest fixes vs largest bets (be explicit)

**Smallest**

- Replace hard-coded `proposal_status` / `broadcast_status` in `proposals_broadcast` with values from `proposals::get_update_action(&client, &action_id)` after broadcast (mirror orchestrator Axum handler).
- Set a strict default CSP compatible with dev server + Vite; document unsafe exceptions.
- Strip operator key handling from IPC: read only from OS keychain or sidecar env in release builds; reject frontend-supplied secrets in hardened profile.

**Largest bets**

- Fine-grained Tauri capabilities / permission groups separating “read ASM” vs “spend/sign” vs “broadcast”.
- Process split: privileged worker with no webview IPC for mnemonic and operator ops.
- Attested release pipeline + reproducible builds for signer distributions.

## What would change my mind (missing evidence / experiments)

- Confirm production build injects CSP via CI not committed in-repo (dual source of truth check).
- Threat-model doc explicitly accepting “IPC trusted webview”; if accepted, downgrade XSS → key theft but document residual laptop malware risk.
- E2E test asserting `broadcastProposal` payload matches persisted orchestrator proposal fields after broadcast.
