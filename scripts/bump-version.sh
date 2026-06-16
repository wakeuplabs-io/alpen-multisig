#!/usr/bin/env bash
# Updates all version fields across Rust and frontend manifests in one shot.
# Run from the repo root: ./scripts/bump-version.sh <new-version>
set -euo pipefail

VERSION="${1:?Usage: $0 <version>  (e.g. 0.2.0)}"

# Validate semver-ish format (digits + dots + optional pre-release label)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9._-]+)?$ ]]; then
    echo "Error: '$VERSION' does not look like a valid version (expected X.Y.Z or X.Y.Z-label)"
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

bump_cargo() {
    local file="$1"
    # Replace only the first `version = "..."` line (the [package] field)
    # — avoids touching dependency version constraints further down the file.
    sed -i "0,/^version = /s|^version = ['\"][^'\"]*['\"]|version = \"$VERSION\"|" "$file"
    echo "  updated $file"
}

bump_json_field() {
    local file="$1"
    # Replace top-level "version" JSON field (the first occurrence is sufficient
    # for both package.json and tauri.conf.json).
    sed -i "s|\"version\": \"[^\"]*\"|\"version\": \"$VERSION\"|" "$file"
    echo "  updated $file"
}

echo "Bumping to $VERSION …"
bump_cargo orchestrator-be/Cargo.toml
bump_cargo desktop-app/src-tauri/Cargo.toml
bump_json_field desktop-app/package.json
bump_json_field desktop-app/src-tauri/tauri.conf.json

echo ""
echo "Done. Next steps:"
echo "  1. Review: git diff"
echo "  2. cargo generate-lockfile        (sync Cargo.lock)"
echo "  3. Commit, PR, merge to develop"
echo "  4. git tag v$VERSION && git push origin v$VERSION"
