# Runbook: resetting state after an ASM pin bump

**Applies to:** any bump of the `alpenlabs/asm` pin that changes the admin wire format.
**First use:** the `e0461f8` → `v0.1-alpha.11` bump for Security Council
([ADR-007](../architecture/adrs/007-asm-pin-for-security-council.md)).

## Why this is mandatory, not hygiene

Admin actions are SSZ unions. `UpdateAction` gained `StrataSecurityCouncilMultisig` **inserted at
selector 3**, which shifts `OperatorSet` and every later variant by one. Two consequences follow,
and neither produces an error:

- An `action_hex` persisted before the bump decodes to a **different action** after it. An
  operator-set update becomes a sequencer update, and so on down the list. Nothing rejects it —
  the bytes are structurally valid, they just mean something else now.
- `ActionId = hash(MultisigAction, SeqNo)` is computed over the decoded action, so pre-bump and
  post-bump IDs are not comparable. Deduplication and lookup silently stop matching.

There is also a schema break the compiler cannot see: `AdministrationInitConfig` and
`ConfirmationDepths` derive `Deserialize` with no `#[serde(default)]`, so any params file missing
the four new fields fails to deserialize at startup.

Skipping any step below leaves the system in a state where the wrong thing happens quietly.

## Procedure

Run these in order. Steps 1–3 are independent of each other; step 4 verifies all of them.

### 1. Orchestrator database

Pre-bump proposals are unreadable. Either reset the database or mark every pre-bump proposal
terminal — do not leave them queryable.

```bash
# Local dev (in-memory repo): nothing to do; restart the process.

# Postgres:
psql "$DATABASE_URL" -c 'TRUNCATE proposals, proposal_signatures CASCADE;'
```

For a deployment with history worth keeping, mark instead of truncating, and record the bump
boundary so the rows are never re-decoded:

```sql
UPDATE proposals SET proposal_status = 'expired'
WHERE proposal_status IN ('pending', 'approved');
```

### 2. ASM runner binary and its database

The runner must be rebuilt from the **same** commit as the workspace pin, and its database
discarded — it holds anchor states produced by the old STF.

```bash
git submodule update --init --recursive          # asm/ now at the new pin
cargo build --release --bin strata-asm-runner --manifest-path asm/Cargo.toml

rm -rf /tmp/asm-runner-db                        # [database].path in asm-config.toml
```

Under Docker Compose the equivalent is a rebuild without cache plus a volume drop:

```bash
cd staging
docker compose down -v
docker compose build --no-cache asm
```

### 3. Regtest datadir

Genesis carries the admin params, so an existing chain still has the **old** authority set — with
no Security Council in it. The chain has to be recreated.

```bash
rm -rf ~/.bitcoin/asm-runner-regtest              # local
docker compose down -v                            # staging: drops the bitcoin volume too
```

Confirm `staging/asm-params.template.json` carries the four new fields before restarting:
`strata_security_council`, and the `strata_security_council_multisig_update`, `defcon3` and
`safe_harbour_address_update` confirmation depths, plus `safe_harbour_address` under `Bridge`.

> **The staging `safe_harbour_address` is a throwaway.** Its payload is the secp256k1 generator
> point, whose private key is `1`. That is fine for regtest and must never reach any other
> environment. The production value is an open question with Alpen — see
> [`specs/security-council.md`](../specs/security-council.md) §9.

### 4. Verify

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The e2e suites that boot a real regtest ASM are the ones that matter here — they skip themselves
when `bitcoind` is not in `PATH`, so check they actually ran rather than trusting a green summary.
Then confirm the runner is serving the new authority set:

```bash
curl -s localhost:8080 -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"strata_asm_getStatus","params":[]}' | jq .
```

## What does not need resetting

- Desktop app state and the admin wallet: no admin-action bytes are persisted there.
- Electrum / Bitcoin Core beyond the regtest datadir.
- Signer mnemonics and hardware wallet configuration.
