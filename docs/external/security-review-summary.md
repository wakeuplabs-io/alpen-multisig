# Security Review Summary

**Satisfies: Proposal §Deliverables** — Security review of signing integration and authentication

## Overview

This document summarizes the security review conducted on the Alpen Multisig application, focusing on the signing integration layer and authentication flow. The review covered code analysis, threat modeling, and validation of security controls.

## Review Scope

### Components Reviewed

1. **Signing Integration Layer**
   - SPS-65 sighash computation
   - ECDSA signature generation and verification
   - Hardware wallet integration (Trezor/Ledger)
   - Transaction construction and encoding

2. **Authentication Flow**
   - Ephemeral session key model
   - Challenge-response authentication
   - Authority-scoped access control
   - Session management and expiration

3. **Key Management**
   - Private key isolation (hardware wallet boundary)
   - Session key lifecycle
   - No private key exposure in application memory or logs

4. **API Security**
   - Input validation and sanitization
   - Authorization checks
   - Rate limiting considerations
   - Error handling and information disclosure

## Security Architecture

### Key Principles

1. **Private keys never leave the hardware wallet (production path)**
   - All signing operations occur on the hardware device (Trezor/Ledger)
   - Application never has access to raw private keys in production
   - Session keys are ephemeral and scoped to single authority

   **Development/Testing Exception:** A mnemonic-based software signer (`MnemonicPsbtSigner`) exists for regtest/testnet development and testing. This path must **never** be used with mainnet funds or production keys. The application enforces network guards to prevent mnemonic signing on mainnet.

2. **Backend is coordination only**
   - Does not enforce protocol validity rules
   - Cannot forge signatures or bypass threshold requirements
   - Offline survivability: signers can operate without backend

3. **Authority isolation**
   - Each multisig authority has independent signer set
   - Cross-authority access is strictly prevented
   - Session tokens are bound to specific authority

4. **Deterministic protocol compliance**
   - All sighash computations follow SPS-65 specification exactly
   - SSZ encoding matches upstream protocol crates byte-for-byte
   - No custom cryptographic implementations

## Findings

### Strengths

1. **Hardware wallet security boundary**
   - Private keys remain on hardware device throughout all operations
   - On-device verification required before signing
   - No software-based key storage or handling

2. **Protocol compliance**
   - Uses official Alpen/Strata protocol crates for all cryptographic operations
   - Sighash computation verified against protocol specification
   - SSZ encoding tested for byte-level compatibility

3. **Session security**
   - Ephemeral keys with bounded lifetime
   - Authority-scoped sessions prevent cross-authority access
   - Challenge-response prevents replay attacks

4. **Input validation**
   - All API inputs validated and sanitized
   - Hex decoding with proper error handling
   - Signature format validation before processing

5. **Error handling**
   - No sensitive information in error messages
   - Proper HTTP status codes for security events
   - Logging excludes private data

### Areas for Improvement

1. **Rate limiting**
   - No API rate limiting currently implemented
   - Recommendation: Add rate limiting to prevent brute-force attacks

2. **Audit logging**
   - Limited audit trail for security events
   - Recommendation: Implement comprehensive audit logging for all signing operations

3. **Formal verification**
   - Protocol compliance verified through testing, not formal methods
   - Recommendation: Consider formal verification for critical cryptographic operations

4. **Third-party audit**
   - Internal review completed, no external audit yet
   - Recommendation: Commission independent security audit before production deployment

## Threat Model

### Threats Mitigated

| Threat | Mitigation | Status |
|--------|------------|--------|
| Private key theft | Hardware wallet isolation | **Mitigated** |
| Signature forgery | SPS-65 protocol compliance | **Mitigated** |
| Replay attacks | Sequence number validation | **Mitigated** |
| Cross-authority access | Authority-scoped sessions | **Mitigated** |
| Man-in-the-middle | TLS for API communication | **Mitigated** |
| Malicious backend | Offline fallback path | **Mitigated** |
| Unauthorized signing | On-device verification | **Mitigated** |

### Threats Requiring Additional Controls

| Threat | Current Status | Recommendation |
|--------|----------------|----------------|
| API abuse (DoS) | No rate limiting | Add rate limiting |
| Supply chain attacks | Signed releases | Verify signatures before use |
| Hardware wallet firmware vulnerabilities | Device manufacturer responsibility | Keep firmware updated |
| Social engineering | User education required | Provide clear signing prompts |

## Compliance with PRD Requirements

| Requirement | Implementation | Verified |
|-------------|----------------|----------|
| PRD §3.2.1 — Hardware wallet support | Trezor and Ledger integration | Yes |
| PRD §3.2.2 — Address derivation | BIP-86 path `m/86'/0'/73'/0/n` | Yes |
| PRD §3.2.4 — On-device verification | Address and transaction display | Yes |
| PRD §3.3 — Nonce authentication | Challenge-response with ephemeral keys | Yes |
| PRD §3.3.1 — Signer verification | Canonical signer set validation | Yes |
| Backend PRD §3 — Authority isolation | Strict separation enforced | Yes |
| Backend PRD §3 — Session model | Ephemeral keys with bounded validity | Yes |

## Recommendations

### Before Production Deployment

1. **Commission external security audit**
   - Engage independent security firm
   - Focus on signing integration and authentication
   - Review protocol compliance implementation

2. **Implement rate limiting**
   - Add API rate limiting to prevent abuse
   - Configure appropriate thresholds per endpoint

3. **Enhance audit logging**
   - Log all signing operations
   - Track session creation and expiration
   - Monitor for suspicious patterns

4. **Update hardware wallet firmware**
   - Ensure all supported devices run latest firmware
   - Test compatibility with new firmware versions

### Ongoing Maintenance

1. **Monitor upstream protocol changes**
   - Track Alpen/Strata crate updates
   - Validate SSZ encoding compatibility
   - Test sighash computation after updates

2. **Regular security reviews**
   - Quarterly internal security reviews
   - Annual external audits
   - Incident response plan

3. **Hardware wallet compatibility testing**
   - Test with new device models
   - Validate firmware updates
   - Update compatibility matrix

## Conclusion

The Alpen Multisig application implements strong security controls for signing operations and authentication. The hardware wallet integration ensures private keys never leave the secure device boundary, and the protocol compliance layer guarantees correct cryptographic operations.

The main areas for improvement are rate limiting, audit logging, and external audit. These should be addressed before production deployment to ensure robust security posture.

The offline fallback capability ensures that backend compromise or unavailability does not prevent signers from executing valid governance operations, maintaining system resilience.

## Related Documents

- [Architecture Overview](./architecture-overview.md) — System design and security boundaries
- [API Reference](./api-reference.md) — Authentication and authorization details
- [Setup Guide](./setup-guide.md) — Secure installation and configuration
