# Rust Tauri Shell (desktop-app/src-tauri) — Adversarial Assessment

## Scope & threat model

**What we're trying to break:**
- A multisig signer desktop app built on Tauri (Rust backend + React frontend) that orchestrates offchain proposal coordination and onchain Bitcoin broadcast.
- Attack surface: IPC commands from the React webview → Rust handlers → cryptographic operations (key derivation, signing, proposal construction, broadcast) → backend HTTP calls and Bitcoin RPC.
- Threat actor: malicious frontend, compromised backend, network observer, supply-chain manipulation.
- High-value targets: mnemonics, secret keys, operator keypairs, authentication tokens, proposal data integrity, replay attacks, state machine invariants.

## Top findings (ranked) — Blocking/High | Medium | Low

### 🔴 BLOCKER — D1: Secret key material passed unencrypted over untrusted IPC boundary

**Severity:** CRITICAL  
**Location:**  
- `desktop-app/src-tauri/src/commands/signing.rs:22–27` (`sign_action_sighash` command)  
- `desktop-app/src-tauri/src/commands/signing.rs:43–55` (`sign_with_mnemonic_path` command)  
- `desktop-app/src-tauri/src/commands/proposals.rs:74–88` (`BroadcastInput` struct, line 81)  

**Evidence:**
```rust
#[tauri::command]
pub(crate) fn sign_action_sighash(
    secret_key_hex: String,    // ← SECRETS COME FROM WEBVIEW AS PLAINTEXT
    sighash_hex: String,
) -> Result<signing::SignatureResult, String>
```
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastInput {
    // ...
    pub operator_secret_key_hex: String,  // ← OPERATOR KEY IN JSON ACROSS IPC
```

**Attack narrative:**
A compromised React frontend (malware injection, supply-chain attack, XSS) or man-in-the-middle on the local IPC channel can capture:
1. Mnemonics (via `sign_with_mnemonic_path`, line 49)
2. Direct secret keys (via `sign_action_sighash`, line 23)
3. Operator keypairs (via `proposals_broadcast`, line 281)
All travel as **plaintext strings** in Tauri's invoke protocol (JSON serialization → local IPC socket).

**Risk:**
- **Key exfiltration:** Malicious frontend extracts signer mnemonics or secrets in clear.
- **No sandboxing:** Tauri IPC is process-local but offers no encryption at rest or in-flight within the desktop app sandbox.
- **Persistent compromise:** Once compromised, every future signing operation leaks the key.

**Smallest fix (partial mitigation):**
- Encrypt sensitive parameters at the IPC boundary using a per-session key derived from the OS keychain.
- Never accept raw mnemonics or secrets from the frontend; only accept indices/derivation paths pre-approved by the user.
- Implement keychain integration (OS-level credential storage) for operator keys.

**Largest bet:**
- Implement **split signing** architecture: move all cryptographic operations to a separate privileged process with restricted IPC and no webview access. The frontend communicates only via high-level requests ("sign this action," "derive address #5") with user prompts. Keys are never exposed to the web layer.

---

### 🔴 BLOCKER — D2: Backend token stored in-memory without expiry enforcement or protection

**Severity:** CRITICAL  
**Location:**  
- `desktop-app/src-tauri/src/application/orchestrator_auth.rs:1–20` (global `OnceLock<Mutex<OrchestratorAuthState>>`)  
- `desktop-app/src-tauri/src/application/orchestrator_auth.rs:48–64` (`get_session`)  

**Evidence:**
```rust
#[derive(Default)]
struct OrchestratorAuthState {
    session: Option<OrchestratorAuthSession>,  // ← BEARER TOKEN HERE, PLAINTEXT IN MEMORY
}

fn state() -> &'static Mutex<OrchestratorAuthState> {
    static STATE: OnceLock<Mutex<OrchestratorAuthState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(OrchestratorAuthState::default()))
}

pub fn get_session() -> Result<Option<OrchestratorAuthSession>, String> {
    let mut lock = state().lock().map_err(|_| "orchestrator auth state lock poisoned".to_string())?;
    let expired = lock.session.as_ref().map(|s| now_unix_ms() >= s.expires_at_unix_ms).unwrap_or(false);
    if expired {
        lock.session = None;  // ← ONLY CLEARED ON LAZY CHECK; NOT PROACTIVE
    }
    Ok(lock.session.clone())  // ← CLONED INTO EVERY RESPONSE
}
```

**Attack narrative:**
1. **Memory inspection:** A signer leaves the desktop app open. An attacker (USB malware, local privilege escalation, malicious task scheduler hook) reads the process memory and extracts the bearer token from the `OnceLock`.
2. **Token replay:** Attacker uses the token to impersonate the signer against the backend for hours (until expiry).
3. **Lazy expiry:** The token is only checked when `get_session()` is called. A token that expires while the frontend is idle is not cleared from memory until the next operation.
4. **Clone hazard:** Every call to `get_session()` clones the token into the response, creating ephemeral copies throughout the runtime heap.

**Risk:**
- **No memory zeroization:** Bearer tokens are copied and cloned without zeroing. Rust's standard `String` does not zero memory on drop.
- **No per-operation expiry:** Tokens are reused across multiple commands without refreshing or short-lived session binding.
- **Flat session model:** No distinction between "signer authenticated to sign" vs. "signer authenticated to manage proposals"; all operations use the same token.

**Smallest fix (partial mitigation):**
- Use `zeroize` crate to zero sensitive memory on drop:
  ```rust
  use zeroize::Zeroize;
  
  #[derive(Clone)]
  pub struct OrchestratorAuthSession {
      // ...
      token: ZeroizeOnDrop,  // ← Wrapper type that zeroes on drop
  }
  ```
- Implement proactive token refresh and short-lived challenge-response per operation.

**Largest bet:**
- Move backend auth into a separate **secure enclave or privileged daemon** (e.g., systemd user service) that holds the token. The Tauri app only receives opaque session IDs; all backend requests are proxied through the daemon with fresh token binding.

---

### 🔴 BLOCKER — D3: CSP is explicitly disabled; XSS will execute arbitrary code with full Tauri access

**Severity:** CRITICAL  
**Location:** `desktop-app/src-tauri/tauri.conf.json:21–23`

**Evidence:**
```json
"security": {
    "csp": null
}
```

**Attack narrative:**
1. **XSS via frontend dependency:** A malicious NPM package or compromised transitive dependency injects a `<script>` tag into the React build.
2. **Full Tauri access:** Because `csp: null`, the injected script runs with **no Content Security Policy** restrictions. It can:
   - Call `window.__TAURI__.invoke()` to trigger all registered commands (sign, broadcast, access tokens, etc.).
   - Access all localStorage, IndexedDB, cookies.
   - Exfiltrate data via network requests (no CORS policy enforced).
3. **Persistent attack:** The compromised code persists across app restarts because it's bundled in the built frontend.

**Risk:**
- **Total compromise:** An attacker with XSS has the same capabilities as the "compromised React frontend" in finding D1—they can extract secrets, create unauthorized proposals, broadcast transactions.
- **Supply chain risk:** A single dependency vulnerability (e.g., in a popular UI library) compromises all signers using this app.

**Smallest fix:**
- Enable a strict CSP immediately:
  ```json
  "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' http://localhost:* https://api.example.com"
  }
  ```

**Largest bet:**
- Implement **Subresource Integrity (SRI)** for all bundled JavaScript; audit and pin all transitive dependencies; use a dependency vulnerability scanner in CI/CD.

---

### 🔴 BLOCKER — D4: Operator secret key accepted directly in IPC command; no rate limiting or confirmation UX

**Severity:** CRITICAL  
**Location:**  
- `desktop-app/src-tauri/src/commands/proposals.rs:247–316` (`proposals_prepare_broadcast` and `proposals_broadcast`)  

**Evidence:**
```rust
#[tauri::command]
pub async fn proposals_broadcast(input: BroadcastInput) -> Result<BroadcastResultDto, String> {
    let client = build_client(input.base_url)?;
    let btc_rpc = HttpBitcoinRpcClient::new(
        &input.btc_rpc_url,
        input.btc_wallet_name.as_deref(),
        &input.btc_rpc_user,
        &input.btc_rpc_pass,
    );
    let keypair = parse_operator_keypair(&input.operator_secret_key_hex)?;  // ← KEY ACCEPTED FROM WEBVIEW
    // ... build commit/reveal Tx ...
    let (commit_txid, reveal_txid) = proposals::broadcast_commit_then_reveal(...).await?;
}
```

**Attack narrative:**
1. **Hot key exploit:** A signer's desktop is infected with malware that intercepts or hijacks the React app's invoke calls.
2. **Silent broadcast:** The malware supplies a crafted `operator_secret_key_hex` (either the real one exfiltrated from storage, or obtained from the signer session) and triggers a broadcast of an **unauthorized or malicious proposal**.
3. **No UX gate:** There is no explicit user confirmation modal asking "About to broadcast with operator key; confirm fingerprint?" The backend accepts the call immediately.
4. **Financial loss:** The operator's signing key is used to broadcast an unauthorized state change to the Alpen ledger, and the operator is liable.

**Risk:**
- **Authorization bypass:** Commands that cost real Bitcoin fees are gated only by possession of the secret key, not by user intent.
- **No multifactor confirmation:** Desktop malware that can invoke Tauri commands can immediately trigger a broadcast without secondary approval.

**Smallest fix:**
- Require explicit user confirmation (modal with key fingerprint display) before any broadcast operation.
- Log all broadcast attempts to an audit trail file.

**Largest bet:**
- Implement **hardware wallet only** for operator keys (Trezor/Ledger integration exists; see `hw_wallet.rs`). Require physical button press on the device to authorize each signing operation. (Migration path: deprecate mnemonic-based operator key signing.)

---

### 🔴 BLOCKER — D5: Backend trust assumptions not validated; attacker backend can return crafted proposals and signatures

**Severity:** HIGH  
**Location:**  
- `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:103–113` (create_proposal, no signature verification)  
- `desktop-app/src-tauri/src/application/proposals.rs:60–101` (broadcast setup, no canonical key verification)  
- `desktop-app/src-tauri/src/domain/proposal.rs:1–21` (Proposal deserialized without invariant checks)  

**Evidence:**
```rust
async fn create_proposal(
    &self,
    request: CreateProposalRequest,
) -> Result<Proposal, OrchestratorError> {
    let req = self.with_auth_headers(
        self.client
            .post(format!("{}/proposals", self.base_url))
            .json(&request),
    )?;
    self.send_and_parse(req).await  // ← NO SIGNATURE VERIFICATION
}
```

The backend responds with a `Proposal` struct that includes `action_hex`, `signatures`, etc. The Tauri client **does not validate** that:
1. The returned `action_hex` matches what it sent.
2. The returned signatures are valid against the sighash.
3. The authority in the proposal matches the authenticated user's authority.

**Attack narrative:**
1. **MITM or malicious backend:** An attacker controls or compromises the backend orchestrator (or intercepts traffic).
2. **Proposal substitution:** When the signer requests to broadcast proposal A, the backend returns proposal B (e.g., one that transfers admin keys or drains a multisig wallet).
3. **Silent broadcast:** The Tauri app does not verify the proposal; it trusts the backend's response and broadcasts B to Bitcoin with the operator key.
4. **Irreversible damage:** The attacker has escalated from backend compromise to onchain action without the signer noticing.

**Risk:**
- **No defense in depth:** Backend is the sole source of truth; no client-side validation layer.
- **Signer safety violation:** AGENTS.md says "Signer safety: Explicit confirmation steps, authority context, high-signal errors," but there is no explicit confirmation that the proposal being broadcast matches what the user initiated.

**Smallest fix:**
- Before broadcasting, re-display the proposal to the user: action hex, signatures, and computed hash. Require explicit confirmation.
- Verify on-client that returned signatures are valid against the action's sighash.

**Largest bet:**
- Implement **client-side proposal cache** with merkle root commitment to the orchestrator. Signer signs the merkle root; backend can only return proposals that were previously cached by the signer.

---

### 🟠 HIGH — D6: Default network is "regtest"; production deployment risk

**Severity:** HIGH  
**Location:** `desktop-app/src-tauri/src/commands/proposals.rs:158`

**Evidence:**
```rust
fn parse_network(network: Option<&str>) -> Result<bitcoin::Network, String> {
    match network.unwrap_or("regtest") {  // ← DEFAULTS TO REGTEST
        "bitcoin" => Ok(bitcoin::Network::Bitcoin),
        // ...
    }
}
```

**Attack narrative:**
1. **Misconfiguration:** A signer forgets to specify `network: "bitcoin"` when calling `proposals_broadcast`, or a frontend bug omits it.
2. **Test mode fund loss:** The operator key is derived and used on testnet/regtest instead of mainnet, but the Bitcoin RPC URL and wallet point to a different network than expected.
3. **Address reuse:** If the operator uses the same key across networks, address reuse across forks can lead to fund loss if a signature is replayed.

**Risk:**
- **Accidental loss:** Operator broadcasts to the wrong network and loses funds or is unable to perform the intended action.
- **No guard rail:** The default should require explicit network selection, not silently fall back.

**Smallest fix:**
```rust
fn parse_network(network: Option<&str>) -> Result<bitcoin::Network, String> {
    match network {
        Some("bitcoin") => Ok(bitcoin::Network::Bitcoin),
        Some("testnet") => Ok(bitcoin::Network::Testnet),
        Some("signet") => Ok(bitcoin::Network::Signet),
        Some("regtest") => Ok(bitcoin::Network::Regtest),
        _ => Err("network must be explicitly specified (bitcoin/testnet/signet/regtest)".to_string()),
    }
}
```

---

### 🟠 HIGH — D7: Bearer token passed in plaintext HTTP header; no HTTPS enforcement

**Severity:** HIGH  
**Location:** `desktop-app/src-tauri/src/infrastructure/orchestrator_client.rs:30–41`

**Evidence:**
```rust
fn with_auth_headers(
    &self,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::RequestBuilder, OrchestratorError> {
    let Some(token) = &self.token else {
        return Err(OrchestratorError::Request(
            "missing orchestrator bearer token".to_string(),
        ));
    };

    Ok(request.header("authorization", format!("Bearer {token}")))  // ← PLAINTEXT
}
```

And in `desktop-app/src-tauri/src/commands/proposals.rs:190`:
```rust
let client = build_client(input.base_url)?;  // ← base_url IS USER-SUPPLIED STRING
```

The `base_url` is accepted directly from the frontend with no validation. If it's `http://` (not `https://`), the token is transmitted in the clear.

**Attack narrative:**
1. **Network compromise:** Signer is on a public WiFi or network segment where an attacker can MITM HTTP traffic.
2. **Token sniff:** The attacker captures the bearer token from the unencrypted Authorization header.
3. **Impersonation:** Attacker uses the token to make requests to the backend on behalf of the signer.

**Risk:**
- **No transport security:** Unlike typical web apps, there is no automatic HTTPS-only policy. The Tauri app trusts the user to enter a valid URL.
- **Silent downgrade:** If a signer accidentally enters `http://...` instead of `https://...`, the app does not warn or enforce TLS.

**Smallest fix:**
- Enforce `https://` scheme:
  ```rust
  fn build_client(base_url: String) -> Result<HttpOrchestratorClient, String> {
      if !base_url.starts_with("https://") {
          return Err("orchestrator base_url must use https://".to_string());
      }
      // ...
  }
  ```
- Pin TLS certificates or use HPKB.

---

### 🟠 HIGH — D8: Mnemonic derivation hardcoded to role #73; no verification against on-chain role config

**Severity:** HIGH  
**Location:** `desktop-app/src-tauri/src/infrastructure/signing.rs:111–133`

**Evidence:**
```rust
pub fn list_mnemonic_addresses(
    mnemonic: &str,
    passphrase: &str,
    count: u32,
) -> Result<Vec<MnemonicAddressEntry>, String> {
    let mut out = Vec::with_capacity(count as usize);
    for n in 0..count {
        let derivation_path = format!("m/86'/0'/73'/0/{n}");  // ← HARDCODED ROLE 73
        // ...
    }
}
```

Role #73 is the Strata Administrator role in `strata-asm-params`. If the on-chain config changes or the user is assigned a different role, the mnemonic derivation path will produce wrong keys.

**Attack narrative:**
1. **Role transition:** A signer's authority is changed from StrataAdministrator to StrataSequencerManager on-chain.
2. **Stale path derivation:** The desktop app still derives keys from `m/86'/0'/73'/0/*` (admin role), but the backend now expects signatures from the sequencer role (#74).
3. **Signature rejection:** The signer's signatures are rejected by the backend because they come from the wrong role's keyspace.
4. **UX confusion:** The signer is told "signature invalid" with no indication that their role has changed.

**Risk:**
- **No role awareness:** The Tauri app has no way to verify which role the current signer belongs to on-chain.
- **Key derivation mismatch:** If the role codespace changes (e.g., role IDs are renumbered), all existing backups become useless.

**Smallest fix:**
- Fetch the current role membership during authentication (`start_challenge` already does this for `auth.rs`).
- Pass the role to `list_mnemonic_addresses` so it can compute the correct derivation path.

---

### 🟡 MEDIUM — D9: No rate limiting on signing commands; DoS vector

**Severity:** MEDIUM  
**Location:** All `#[tauri::command]` handlers in `src/commands/`

**Evidence:**
```rust
#[tauri::command]
pub(crate) fn compute_sighash(seqno: u64, action_hex: String) -> Result<signing::SighashResult, String> {
    signing::compute_sighash(seqno, &action_hex)
}

#[tauri::command]
pub(crate) fn list_mnemonic_addresses(
    mnemonic: String,
    passphrase: Option<String>,
    count: Option<u32>,
) -> Result<Vec<signing::MnemonicAddressEntry>, String> {
    signing::list_mnemonic_addresses(
        &mnemonic,
        passphrase.as_deref().unwrap_or(""),
        count.unwrap_or(20),  // ← COUNT UNBOUNDED
    )
}
```

The `count` parameter in `list_mnemonic_addresses` defaults to 20 and is unbounded. A malicious frontend can request 1 billion addresses, causing the Tauri app to hang or OOM.

**Risk:**
- **Denial of Service:** Malicious frontend can lock up the signer's desktop app.
- **Resource exhaustion:** Each address derivation involves BIP32 hashing; requesting a huge count will peg CPU.

**Smallest fix:**
```rust
let count = std::cmp::min(count.unwrap_or(20), 1000);  // ← CAP AT 1000
```

---

### 🟡 MEDIUM — D10: No idempotency on broadcast commands; duplicate broadcasts possible

**Severity:** MEDIUM  
**Location:** `desktop-app/src-tauri/src/application/proposals.rs:109–228` (`broadcast_commit_then_reveal`)

**Evidence:**
The function accepts an `action_id` and broadcasts the commit/reveal Txs to Bitcoin. If the frontend (maliciously or due to a bug) calls `proposals_broadcast` twice with the same `action_id`:
1. First call: broadcasts commit, waits for confirmation, broadcasts reveal.
2. Second call: tries to broadcast commit again—but the operator's Bitcoin wallet now sees the commit UTXO as already spent (if reveal was built), leading to a double-spend attempt.

**Attack narrative:**
1. **Replay attack:** A malicious frontend calls `proposals_broadcast` multiple times in rapid succession.
2. **Double spend:** The Bitcoin RPC attempts to build two separate transactions for the same commit UTXO, resulting in conflicting Txs.
3. **Fee drain:** Each failed attempt still incurs fees; a malicious frontend can drain the operator's wallet.

**Risk:**
- **No deduplication:** The app does not track which `action_id`s have already been broadcast.
- **Economic attack:** Repeated broadcasts drain fees.

**Smallest fix:**
- Maintain a local cache of recently broadcast `action_id`s:
  ```rust
  static BROADCAST_CACHE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
  
  pub async fn broadcast_commit_then_reveal(...) -> Result<(String, String), BroadcastError> {
      let mut cache = BROADCAST_CACHE.get_or_init(|| Mutex::new(HashSet::new())).lock().unwrap();
      if cache.contains(&action_id.to_string()) {
          return Err(BroadcastError::Setup("proposal already broadcast".to_string()));
      }
      cache.insert(action_id.to_string());
      // ... proceed ...
  }
  ```

---

### 🟡 MEDIUM — D11: No capability-based access control; all commands equally exposed

**Severity:** MEDIUM  
**Location:** `desktop-app/src-tauri/src/main.rs:9–36` (invoke_handler)

**Evidence:**
```rust
.invoke_handler(tauri::generate_handler![
    commands::asm_state::get_multisig_config,
    commands::action_builder::build_admin_multisig_update_hex,
    commands::authentication::auth_start_challenge,
    // ... 20+ more commands all exposed equally ...
    commands::signing::sign_with_mnemonic_path,
])
```

All commands are registered with equal priority. There is no capability model: no way to restrict certain commands to certain flows, no way to require "user just authenticated before calling `sign_with_mnemonic_path`," etc.

**Attack narrative:**
1. **Out-of-order execution:** Malicious frontend calls `proposals_broadcast` before the signer has completed the authentication flow.
2. **Stale state:** The app relies on frontend to enforce flow ordering (start auth → complete auth → create proposal → approve → broadcast). A malicious frontend skips steps.
3. **Privilege escalation:** Frontend calls high-risk commands (broadcast, sign with operator key) without completing required prerequisites.

**Risk:**
- **No OCAP model:** Commands are gated only by whether the Tauri process is running, not by application state or user intent.

**Smallest fix:**
- Implement a command capability guard:
  ```rust
  #[tauri::command]
  pub async fn proposals_broadcast(input: BroadcastInput) -> Result<BroadcastResultDto, String> {
      let session = get_session()?  // ← GATE: must have valid session
          .ok_or("not authenticated")?;
      
      if session.role != AuthRole::StrataAdministrator {
          return Err("insufficient role".to_string());
      }
      
      // ... proceed ...
  }
  ```

---

## Attack narratives (3–6)

### Narrative A: Malware-in-the-middle (React / Tauri bridge)

**Setup:** Signer installs desktop app. A supply-chain compromise injects malicious code into the React build (or a dependency update delivers a trojan).

**Attack:**
1. Malware intercepts the signer's authentication flow and saves the bearer token (D2 + D3).
2. When the signer initiates a proposal broadcast, malware modifies the `BroadcastInput.action_id` to point to a different proposal (one that escalates privileges or drains funds).
3. The Tauri backend accepts the tampered input and broadcasts the wrong proposal (D5 + D4).
4. Attacker repeats the broadcast to drain operator fees (D10).

**Outcome:** Authority corruption, financial loss, irreversible onchain damage.

---

### Narrative B: Desktop compromise + key exfiltration

**Setup:** Signer's machine is infected with local malware (credential stealer, keylogger, etc.).

**Attack:**
1. Malware observes the signer entering a mnemonic phrase to derive addresses (D1).
2. Malware exfiltrates the mnemonic and waits for the signer to close the app.
3. Using the mnemonic, malware computes the signer's private key (BIP39 + BIP32 are deterministic).
4. Malware uses the key to sign a forged proposal and submits it to the backend as if from the legitimate signer.
5. If the backend does not verify the proposal's content against the signer's intent (D5), it accepts the forged proposal.

**Outcome:** Unauthorized proposals, phishing attacks on other signers, loss of trust.

---

### Narrative C: Untrusted backend (or MITM)

**Setup:** Signer enters an incorrect backend URL (or network is compromised).

**Attack:**
1. Attacker intercepts all traffic or sets up a fake backend.
2. When signer requests to broadcast proposal A, attacker's backend returns a crafted `Proposal` struct with different `action_hex` (D5).
3. Signer's frontend does not verify that the returned proposal matches what was requested; it displays the attacker's version.
4. If the signer approves based on the frontend's display, the Tauri app broadcasts the attacker's proposal (because `broadcast_commit_then_reveal` uses `proposal.action_hex` from the backend response, not what was sent).
5. Attacker has used the signer's operator key to enact an unauthorized state change.

**Outcome:** Authority compromise, loss of operator key trust.

---

### Narrative D: Role misconfiguration + key mismatch

**Setup:** Signer's on-chain role changes from StrataAdministrator to StrataSequencerManager, but the desktop app still has the old role hardcoded in the derivation path (D8).

**Attack:**
1. Signer tries to sign a proposal using their mnemonic.
2. Desktop app derives keys from `m/86'/0'/73'/0/*` (admin path), but the signer's on-chain role is now #74 (sequencer).
3. Signer's signature does not match the backend's expectation; it is rejected.
4. Signer is confused and retries, eventually giving up or trying to work around the error (e.g., using a different tool, exposing the key to a third party).
5. Attacker observes the retries and captures the key or exploits the workaround.

**Outcome:** Signer frustration, key exposure risk, operational failure.

---

### Narrative E: DoS via unbounded derivation

**Setup:** Malicious frontend triggers excessive key derivation.

**Attack:**
1. Malicious code calls `list_mnemonic_addresses(mnemonic, passphrase, count=1_000_000_000)` (D9).
2. Desktop app attempts to derive 1 billion keys from the mnemonic.
3. The Tauri process maxes out CPU and memory, becoming unresponsive.
4. Signer cannot interact with the app; Tauri must be force-killed.
5. If the app was in the middle of a critical operation (e.g., holding a partial signature), the state is lost.

**Outcome:** Operational outage, signer unable to respond to time-sensitive proposals.

---

## Evidence index (paths)

| Finding | File | Line | Type |
|---------|------|------|------|
| D1: Secret key IPC | `commands/signing.rs` | 22–27 | IPC command |
| D1: Operator key IPC | `commands/proposals.rs` | 74–88 | Struct definition |
| D2: Token storage | `application/orchestrator_auth.rs` | 1–20 | Global state |
| D2: Token clone hazard | `application/orchestrator_auth.rs` | 63 | Memory safety |
| D3: CSP disabled | `tauri.conf.json` | 21–23 | Config |
| D4: Operator key accepted | `commands/proposals.rs` | 281–289 | IPC command |
| D4: No confirmation UX | `commands/proposals.rs` | 281–316 | Logic |
| D5: No signature verification | `infrastructure/orchestrator_client.rs` | 103–113 | HTTP client |
| D5: No proposal validation | `application/proposals.rs` | 60–101 | App logic |
| D6: Default network regtest | `commands/proposals.rs` | 158 | Parse function |
| D7: Bearer token plaintext | `infrastructure/orchestrator_client.rs` | 30–41 | HTTP header |
| D7: Base URL from frontend | `commands/proposals.rs` | 190 | IPC input |
| D8: Hardcoded derivation path | `infrastructure/signing.rs` | 118 | Const string |
| D9: Unbounded count | `infrastructure/signing.rs` | 114 | Default parameter |
| D10: No deduplication | `application/proposals.rs` | 109–228 | Broadcast function |
| D11: No capability model | `main.rs` | 9–36 | Handler registration |

---

## Smallest fixes vs largest bets

### Smallest fixes (can be done in < 1 day per issue)

1. **D3: Enable CSP** — Add strict CSP header in `tauri.conf.json`.
2. **D6: Remove default network** — Require explicit network parameter; fail if omitted.
3. **D7: Enforce HTTPS** — Validate `base_url` scheme in `build_client()`.
4. **D8: Fetch role at auth** — Pass role from `start_challenge` to mnemonic derivation; compute path dynamically.
5. **D9: Cap derivation count** — `let count = std::cmp::min(count.unwrap_or(20), 1000);`
6. **D11: Add capability guards** — Check `get_session()` and role in high-risk commands (broadcast, sign with operator key).

### Largest bets (architectural shifts, 1–2 weeks each)

1. **D1: Split signing architecture** — Move all crypto operations to a separate privileged process; frontend only sends high-level requests ("derive address #5", "sign proposal X"). Keys never leave the crypto process. Requires:
   - New daemon/service for signing.
   - IPC protocol redesign (opaque request/response, no plaintext keys).
   - OS keychain integration for key storage.
   - User confirmation flow with hardware wallet fallback.

2. **D2: Secure token storage** — Implement hardware-backed keychain (OS-level credential storage for bearer token). Requires:
   - Integration with system keychain (Windows Credential Manager, macOS Keychain, Linux GNOME Keyring).
   - Token zeroization on drop (zeroize crate).
   - Per-command token binding (short-lived derived tokens).

3. **D4 + D1: Hardware wallet only for operator keys** — Deprecate mnemonic-based operator signing; require Trezor/Ledger for all operator operations. Requires:
   - Hardened hardware wallet integration (already partially present in `hw_wallet/trezor.rs`).
   - UX redesign: user must approve each broadcast on the physical device.
   - Fallback strategy for cold key recovery.

4. **D5: Client-side proposal verification** — Implement a merkle root commitment model where signer signs the action's hash; backend can only return that exact proposal. Requires:
   - Merkle tree for all proposals; signer caches and verifies against tree root.
   - Backend must include proof of inclusion in response.
   - Substantial protocol change (involves orchestrator backend too).

---

## What would change my mind

### Missing evidence that would shift the risk assessment

1. **Hardware wallet exclusively used for all signing** — If I find evidence that operator keys are stored only on Trezor/Ledger (not in mnemonics or hot keys), D1 and D4 severity downgrade to Medium (still a risk, but not exploitable via the app itself).

2. **Memory zeroization already implemented** — If `OrchestratorAuthSession` uses `zeroize` or a custom wrapper that zeros on drop, D2 severity downgrade to Medium (still in-memory, but at least not left in plaintext garbage).

3. **Production CSP config file** — If there's a separate prod config with CSP enabled, D3 is conditional (only a blocker for debug builds). Look for env-specific Tauri configs.

4. **Backend validates proposal integrity** — If the orchestrator backend enforces that a proposal can only be broadcast if the returned `action_hex` matches the originally submitted one, and includes a signature of the proposal for the client to verify, D5 is mitigated (shifts burden to backend, which is risky but reduces client-side trust assumptions).

5. **Per-command TOTP or MFA** — If there's a second factor (time-based OTP or hardware key challenge) required before high-risk commands like `proposals_broadcast`, D4 is mitigated.

6. **No production use of default network** — If docs/deployment guide explicitly mandate network selection and warn about defaults, D6 is reduced to Low.

### Experiments to validate findings

1. **Intercept IPC to capture secrets** — Build a test app that runs alongside Tauri and attempts to sniff the IPC socket for plaintext keys. Verify that `secret_key_hex` is sent unencrypted. (Validates D1.)

2. **Memory dump bearer token** — Using a debugger or `/proc/[pid]/mem`, attempt to read the `OrchestratorAuthState` from a running app and extract the bearer token. (Validates D2.)

3. **XSS inject malicious command** — Inject a `<script>` tag into the frontend dev environment (or modify the bundled JS in a test build) and verify it can call arbitrary `invoke()` commands without CSP restrictions. (Validates D3.)

4. **MITM proposal broadcast** — Set up a proxy between the Tauri app and the orchestrator backend. Intercept a `proposals_broadcast` call, modify the returned `Proposal` struct, and verify the app broadcasts the attacker's version without warning. (Validates D5.)

5. **Network misconfiguration test** — Call `proposals_broadcast` without specifying a network parameter and verify it defaults to regtest. (Validates D6.)

6. **DoS via unbounded derivation** — Call `list_mnemonic_addresses(mnemonic, "", 1_000_000_000)` and measure app hang time and memory usage. (Validates D9.)

---

## Summary

The Rust Tauri shell exhibits **five critical security flaws** that, individually or combined, could lead to:
- **Signer key compromise** (mnemonics, operator keys exfiltrated in plaintext over IPC or memory; no zeroization).
- **Unauthorized proposal broadcast** (backend response accepted without client-side validation; no user confirmation modal).
- **Authority escalation** (malicious frontend can invoke high-risk commands without capability checks).
- **Supply-chain compromise** (XSS via CSP bypass enables full app hijacking).

The smallest fixes (CSP, network validation, HTTPS enforcement, capability guards) can be deployed quickly and will prevent opportunistic attacks. However, **without a fundamental shift in architecture** (split signing, hardware wallet integration, secure token storage), the app remains vulnerable to sophisticated adversaries with access to the developer's supply chain or the signer's desktop.

**Recommendation:** Treat findings D1–D5 as blocking production deployment. Implement smallest fixes in the next sprint. Plan for largest bets (split signing, hardware wallet) in the next release cycle. Engage a third-party security audit before any public beta or mainnet deployment.
