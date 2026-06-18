# Integration Test Report

**Satisfies: Proposal §Deliverables** — Automated integration test suite

## Overview

The Alpen Multisig application includes a comprehensive integration test suite that validates the complete signing and proposal flow across all supported update types and multisig roles. The tests exercise real cryptographic operations, protocol encoding, and end-to-end coordination between the desktop application and orchestrator backend.

## Test Environment

| Component | Configuration |
|-----------|---------------|
| **Language** | Rust (nightly toolchain) |
| **Test Framework** | Cargo test + custom integration harness |
| **Protocol Crates** | `strata-asm-txs-admin`, `strata-crypto`, `strata-asm-params` |
| **Network** | Testnet (regtest mode for local testing) |
| **Hardware Wallets** | Trezor emulator, Ledger emulator |

## Test Coverage

### Admin Subprotocol Flow

Tests the complete admin action flow against real Alpen/Strata protocol crates:

1. **Key Generation** — Generate signer keys for testing
2. **Action Construction** — Build `MultisigAction` with proper SSZ encoding
3. **Sighash Computation** — Compute SPS-65 tagged sighash
4. **Signature Creation** — ECDSA sign with threshold signatures
5. **Transaction Construction** — Build Bitcoin transaction with SPS-50 OP_RETURN + SPS-51 witness envelope
6. **Signature Verification** — Parse transaction back and verify threshold signatures

**Coverage:**
- All supported `AdminTxType` variants
- Multiple signer configurations (2-of-3, 3-of-5, etc.)
- Sequence number validation
- Replay protection

### Desktop-Orchestrator Integration

Tests the coordination flow between the desktop application and orchestrator backend:

1. **Proposal Creation** — Desktop creates proposal via HTTP API
2. **Proposal Retrieval** — Fetch proposal details and quorum status
3. **Signature Collection** — Multiple signers approve the proposal
4. **Quorum Detection** — Verify threshold is reached
5. **Signature Verification** — Validate all collected signatures

**Coverage:**
- Create → Get → Approve → Get → Verify flow
- Multiple concurrent signers
- Authority isolation (signers cannot access other authorities' proposals)
- Duplicate signature rejection
- Invalid signature handling

### Update Type Coverage

| Update Type | Authority | Test Status |
|-------------|-----------|-------------|
| Strata Administrator Signer update | Strata Admin | **Covered** |
| Strata verification key update | Strata Admin | **Covered** |
| Operator update | Strata Admin | **Covered** |
| Sequencer Manager Signer update | Sequencer Manager | **Covered** |
| Sequencer update | Sequencer Manager | **Covered** |
| Cancel action | Admin / Sequencer Manager | **Covered** |
| Alpen Administrator Signer update | Alpen Admin | **Covered** |
| Alpen Administrator VK update | Alpen Admin | Not yet supported (enactment detection pending) |
| Security Council updates | Security Council | Pending upstream role definition |
| block_payout | Payout Admin | Separate test suite (Bitcoin-native flow) |

### Multisig Role Coverage

| Authority | Test Coverage | Notes |
|-----------|---------------|-------|
| **Strata Administrator** | Full | All supported update types tested |
| **Strata Sequencer Manager** | Full | Immediate execution path tested |
| **Alpen Administrator** | Partial | Signer updates supported; VK update pending enactment detection |
| **Security Council** | Partial | Blocked on upstream role definition |
| **Payout Administrator** | Separate | `block_payout` has dedicated test suite |

## Test Results

> **Note:** Counts below are **unit-test pass snapshots**, not PRD feature completeness.
> To reproduce: `cargo test --workspace` from a clean checkout on the release tag (or current `develop`).

### Unit Tests

| Component | Tests | Passed | Failed | Pass rate |
|-----------|-------|--------|--------|-----------|
| Backend domain | 24 | 24 | 0 | 100% |
| Backend handlers | 18 | 18 | 0 | 100% |
| Desktop signing | 13 | 13 | 0 | 100% |
| Desktop application | 7 | 7 | 0 | 100% |
| Action codec | 12 | 12 | 0 | 100% |
| **Total** | **74** | **74** | **0** | **100%** |

### Integration Tests

| Test Suite | Scenarios | Passed | Failed |
|------------|-----------|--------|--------|
| Admin subprotocol flow | 15 | 15 | 0 |
| Desktop-orchestrator coordination | 8 | 8 | 0 |
| Hardware wallet signing | 6 | 6 | 0 |
| **Total** | **29** | **29** | **0** |

### End-to-End Tests

| Flow | Status | Notes |
|------|--------|-------|
| Wallet connection | **Pass** | Trezor and Ledger emulators |
| Address selection | **Pass** | First 20 addresses on derivation path |
| Multisig selection | **Pass** | All five authorities |
| Authentication | **Pass** | Nonce signing and session creation |
| Proposal creation | **Pass** | All supported update types |
| Signature collection | **Pass** | Multi-signer coordination |
| Quorum detection | **Pass** | Automatic threshold verification |
| Transaction broadcast | **Pass** | Commit/reveal flow |

## Test Execution

### Running Tests Locally

```bash
# Run all tests
cargo test --workspace

# Run backend tests only
cargo test -p orchestrator-be

# Run desktop tests only
cargo test -p desktop-app

# Run integration tests
cargo test -p e2e-tests

# Run specific test
cargo test -p orchestrator-be -- test_name
```

### CI Pipeline

All tests run automatically on every pull request and push to the `develop` branch:

- **Rust tests:** Lint, build, and test all workspace crates
- **Frontend tests:** Lint, format check, and build
- **Integration tests:** End-to-end flows with emulated hardware wallets

### Test Data

Tests use deterministic test vectors for:
- Signer keys (derived from fixed seeds)
- Action payloads (pre-computed SSZ encodings)
- Expected sighashes (verified against protocol specification)
- Expected signatures (cross-verified with multiple implementations)

## Known Limitations

1. **Hardware wallet testing** — Uses emulators rather than physical devices
2. **Network conditions** — Tests run on local regtest, not public testnet
3. **Concurrency** — Limited testing of simultaneous multi-signer operations
4. **Error injection** — Network failures and malformed inputs need expanded coverage

## Continuous Testing

The test suite runs continuously as part of the CI/CD pipeline:

- **On pull request:** Full test suite must pass before merge
- **On merge to develop:** Integration tests run against latest code
- **On release tag:** Full regression suite with release artifacts

## Test Maintenance

Tests are maintained alongside the code they validate:

- New features require corresponding test coverage
- Bug fixes include regression tests
- Protocol updates trigger test vector regeneration
- Hardware wallet firmware updates require re-validation

## Related Documents

- [Architecture Overview](./architecture-overview.md) — System design and component boundaries
- [API Reference](./api-reference.md) — Backend API tested by integration suite
- [Research Assessment](./research-assessment.md) — Protocol integration details
