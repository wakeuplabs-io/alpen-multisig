#!/usr/bin/env bash
#
# verify-reproducible-build.sh — independently rebuild the desktop app and
# compare its Tier 1 digests (release binary + frontend bundle) against a
# published, signed reference.
#
# Reproducibility scope (D4 / NF-2): this verifies the *code that runs* — the
# `desktop-app` release binary and the Vite `dist/` frontend — not the installer
# wrappers (.deb/.rpm/AppImage/.dmg), which are only partially reproducible, and
# not the signed macOS .dmg, which cannot be bit-for-bit by construction.
# See docs/operations/reproducible-builds.md for the full rationale.
#
# Usage:
#   scripts/verify-reproducible-build.sh path/to/REPRODUCIBLE-DIGESTS.txt
#
# Prerequisites: the pinned toolchain (rust-toolchain.toml), the Node major in
# .nvmrc, and the Tauri system dependencies (see docs/operations/desktop-build-linux.md).
# Run from a clean checkout of the exact release tag.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

REFERENCE="${1:-}"
if [[ -z "$REFERENCE" || ! -f "$REFERENCE" ]]; then
	echo "usage: $0 path/to/REPRODUCIBLE-DIGESTS.txt" >&2
	echo "  (download REPRODUCIBLE-DIGESTS.txt from the GitHub release first)" >&2
	exit 2
fi

# The release workflow hashes with `sha256sum` (Linux) / `shasum -a 256` (macOS),
# which share the "<hash>  <name>" output format. Resolve the matching tool once
# as a command array so `xargs` can exec it directly (it cannot call a function).
if command -v sha256sum >/dev/null 2>&1; then
	SHA_CMD=(sha256sum)
else
	SHA_CMD=(shasum -a 256)
fi

# Bare hex digest of a single file (matches `... | cut -d' ' -f1` in CI).
sha256() { "${SHA_CMD[@]}" "$@" | cut -d' ' -f1; }

dist_digest() {
	# MUST mirror release.yml exactly: hash every file (path included), in a
	# stable order, then hash that listing. Changing either step here or in the
	# workflow breaks verification.
	( cd desktop-app/dist && find . -type f | LC_ALL=C sort \
		| xargs "${SHA_CMD[@]}" | "${SHA_CMD[@]}" | cut -d' ' -f1 )
}

# Reproduce the same fixed build timestamp the release workflow uses.
export SOURCE_DATE_EPOCH="$(git log -1 --pretty=%ct)"
echo "==> SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH (release commit author date)"
echo "==> Building Tauri bundle (this compiles the release binary)…"
( cd desktop-app && npm ci && npm run tauri build )

BIN_DIGEST="$(sha256 target/release/desktop-app)"
DIST_DIGEST="$(dist_digest)"

echo
echo "==> Locally rebuilt digests:"
echo "    binary(desktop-app)  $BIN_DIGEST"
echo "    frontend(dist)       $DIST_DIGEST"
echo

# The reference lists per-platform sections; a match on either OS section's
# binary line plus the frontend line is a successful reproduction for this host.
status=0
if grep -q "$BIN_DIGEST" "$REFERENCE"; then
	echo "OK   binary digest matches the published reference"
else
	echo "FAIL binary digest NOT found in $REFERENCE"
	echo "     run 'diffoscope' against the published binary to explain the diff"
	status=1
fi
if grep -q "$DIST_DIGEST" "$REFERENCE"; then
	echo "OK   frontend digest matches the published reference"
else
	echo "FAIL frontend digest NOT found in $REFERENCE"
	status=1
fi

exit "$status"
