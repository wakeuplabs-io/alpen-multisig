# Release signing keys

This directory holds the **public** OpenPGP keys of the Alpen Labs employees
authorized to sign releases of the desktop application. Users import these keys
to verify that a downloaded binary was published by the project.

## Trust model

Each release ships a `SHA256SUMS` manifest covering every platform artifact, plus
one detached signature **per signer**: `SHA256SUMS.<signer>.asc`. Verification is a
manifest-and-keyring model (the same approach used by Bitcoin Core, Tor, and Debian):

- **Today (D3):** a single named employee signs each release. This proves origin
  and integrity — the binary came from the project and was not tampered with.
- **Target (D7):** multiple employees each attach their own signature, and users
  verify an M-of-N threshold, satisfying PRD NF-3 ("approved by multiple employees").

The mechanism does not change between D3 and D7 — D7 only adds more keys to this
directory and more `SHA256SUMS.<signer>.asc` files to the release. Nothing here is
throwaway.

> **Important:** each key here is an individual employee's personal release key, not
> a shared "project key." A shared key would not provide the separation of authority
> that NF-3 requires and does not scale to multi-party signing.

## Adding a signer

1. The employee generates a key (keep the private key offline / in a hardware token):

   ```bash
   gpg --full-generate-key            # Ed25519 / RSA-4096 recommended
   gpg --armor --export <KEY_ID> > release-keys/<signer>.asc
   ```

2. Commit `release-keys/<signer>.asc` via PR. The PR review is the human gate that
   admits the key into the trust set.

3. Configure the signing secrets so CI can sign with this key (one signer per the
   secrets below; D7 will generalize to a signing matrix):

   - Repository secret `PGP_PRIVATE_KEY` — ASCII-armored private key
     (`gpg --armor --export-secret-keys <KEY_ID>`).
   - Repository secret `PGP_PASSPHRASE` — the key passphrase (omit if none).
   - Repository variable `PGP_SIGNER_ID` — short signer slug, e.g. `jane`. Produces
     `SHA256SUMS.jane.asc`.

Until `PGP_PRIVATE_KEY` is set, releases still publish `SHA256SUMS` (integrity) but
no signature (authenticity) — see the release workflow's graceful-degradation guard.

## Verifying a release

See [`docs/operations/verifying-releases.md`](../docs/operations/verifying-releases.md).
