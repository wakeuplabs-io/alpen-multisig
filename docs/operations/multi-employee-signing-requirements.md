# Multi-Employee Release Signing

> **External document — For Alpen Labs**

## Overview

The release signing infrastructure supports cryptographic signatures from multiple Alpen Labs employees, providing independent verification that a release has been reviewed and approved by several members of the team. This document describes the process for employees to participate in the release signing ceremony.

## Release Artifact Integrity

Each release consists of platform-specific installer packages (`.deb`, `.rpm`, `.AppImage`, `.dmg`, `.msi`, `.exe`) accompanied by a SHA-256 checksum manifest (`SHA256SUMS`) and a reproducibility digest (`REPRODUCIBLE-DIGESTS.txt`).

When an employee signs the `SHA256SUMS` file using their personal PGP key, they are cryptographically attesting that the checksums match the artifacts they have reviewed and approved for release. Users can verify these signatures to confirm that the binaries they download originate from Alpen Labs and have been reviewed by the named signers.

## Signing Process

### Prerequisites

Each participating employee requires:

- A personal PGP key pair (private key kept secure, public key shared with the team)
- Access to the Alpen Multisig GitHub repository (to commit the public key)
- GitHub Secrets access configured by the development team

### Step 1 — Generate PGP Key Pair

Generate a new PGP key using a tool such as GnuPG:

```bash
gpg --full-generate-key
```

Recommended settings:

- Key type: RSA and RSA
- Key size: 4096 bits
- Expiration: 3 years (or as appropriate)
- Real name: Your full name
- Email: Your Alpen Labs email address

Save the private key securely and never share it. The private key is used to sign releases and must remain under your sole control.

### Step 2 — Export Public Key

Export your public key in ASCII armor format:

```bash
gpg --armor --export your@email.com > alpen-employee-YOURNAME.asc
```

Commit this file to the `release-keys/` directory in the repository. The public key allows anyone to verify signatures made with your corresponding private key.

### Step 3 — Configure GitHub Secrets

Contact the development team to configure your signing credentials in GitHub Actions. You will need to provide:

- Your PGP private key (the raw ASCII armor content)
- The passphrase for your private key (if applicable)
- Your chosen signer identifier (typically your name or handle)

These are stored as GitHub Secrets and are only accessible to the release workflow.

### Step 4 — Signing a Release

When a release is prepared, the release workflow will automatically attempt to sign the `SHA256SUMS` manifest using all configured private keys. The resulting signature files are named `SHA256SUMS.<signer_id>.asc` and are attached to the GitHub Release.

For releases where an employee's key has not yet been configured, the release may proceed with a subset of signatures. The infrastructure is designed to accommodate any number of signers.

## Verification

Users downloading a release can verify the signatures by:

1. Downloading the release artifacts and the `SHA256SUMS` manifest
2. Importing the public keys of the signers they wish to verify
3. Running `gpg --verify SHA256SUMS.<signer_id>.asc SHA256SUMS` for each signer
4. Confirming that the checksums in `SHA256SUMS` match the artifacts they downloaded

A release is considered approved by Alpen Labs when it carries signatures from the required number of authorized signers as defined by Alpen Labs policy.

## Key Management Recommendations

- **Private key security**: Store your private key on a hardware token (YubiKey or similar) when possible. Never commit private keys to version control.
- **Key expiration**: Set an expiration date on your key and plan for renewal. Ensure continuity by maintaining a backup of your key material.
- **Revocation**: If a key is compromised or an employee leaves, the key should be revoked and a new one generated. Contact the development team to remove the compromised key from the release workflow.
- **Multiple devices**: If you work from multiple computers, securely transfer your private key to each device. Avoid transmitting the key over unencrypted channels.

## Ongoing Participation

Employee participation in the release signing ceremony is voluntary and based on Alpen Labs internal policy. When a release requires signatures, the development team will coordinate with participating employees to ensure all required signers have their keys configured before the release is published.

## Questions

For questions about the release signing process, contact the development team at WakeUp Labs.