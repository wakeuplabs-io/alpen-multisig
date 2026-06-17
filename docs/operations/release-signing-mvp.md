# Release signing — internal pointer

**Client-facing guide (canonical):** [`docs/external/release-signing.md`](../external/release-signing.md)

Use the external document for the signing approach, multi-signer model, and verification overview.

## Internal-only references

| Topic | Where |
|-------|--------|
| D3 / D7 deliverable status | [`executable-delivery-plan.md`](./executable-delivery-plan.md) — D3 (signed release), D7 (multi-employee ceremony) |
| Key generation and GitHub secrets | [`release-keys/README.md`](../../release-keys/README.md) |
| Release workflow automation | `.github/workflows/release.yml` (`release` job; graceful degradation when `PGP_PRIVATE_KEY` is unset) |
| Multi-employee process (internal) | [`multi-employee-signing-requirements.md`](./multi-employee-signing-requirements.md) |
| Platform code signing (deferred) | [`platform-code-signing-requirements.md`](./platform-code-signing-requirements.md) — Apple Developer ID / Authenticode |
