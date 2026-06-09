# Verifying a release

Every release publishes a `SHA256SUMS` manifest and at least one detached signature
`SHA256SUMS.<signer>.asc`. Verifying gives you two guarantees:

1. **Integrity** — the file you downloaded is bit-for-bit what the project published
   (via the SHA-256 checksum).
2. **Authenticity** — the manifest was signed by an authorized Alpen Labs employee
   (via the OpenPGP signature).

> Verify the **signature on the manifest first**, then check your download against the
> manifest. Checking a checksum without verifying the signature proves nothing — an
> attacker who swaps the binary can swap the checksum too.

## 1. Import the signing keys

The authorized public keys live in [`release-keys/`](../../release-keys/) in this
repository. Import them once:

```bash
gpg --import release-keys/*.asc
```

## 2. Download the release files

From the GitHub Release, download:

- the artifact for your platform (`.deb`, `.rpm`, `.AppImage`, or `.dmg`),
- `SHA256SUMS`,
- `SHA256SUMS.<signer>.asc` (one or more).

Put them in the same directory.

## 3. Verify the signature on the manifest

```bash
gpg --verify SHA256SUMS.<signer>.asc SHA256SUMS
```

Expect `Good signature from "<signer name>"`. A `WARNING: This key is not certified
with a trusted signature` note is normal unless you have locally signed the key — what
matters is that the key fingerprint matches the one published in `release-keys/`.

When multiple signatures are present (D7), verify each one and confirm the required
number of distinct, authorized signers.

## 4. Verify your download against the manifest

```bash
sha256sum --ignore-missing -c SHA256SUMS
```

Expect `OK` next to the file you downloaded. `--ignore-missing` lets you check only the
artifact for your platform without downloading the others.

On macOS, use `shasum -a 256 --ignore-missing -c SHA256SUMS`.

If either step fails, **do not run the binary** — re-download or report it.

## 5. (Optional) Reproduce the build yourself

The signed manifest also covers `REPRODUCIBLE-DIGESTS.txt`, the SHA-256 of the release binary and
frontend bundle. To independently rebuild from source and confirm those digests match bit-for-bit,
see [`reproducible-builds.md`](./reproducible-builds.md).
