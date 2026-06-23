#!/usr/bin/env bash
# Verifies that every version field touched by bump-version.sh matches an
# expected version (typically the release tag, vX.Y.Z -> X.Y.Z).
# Run from anywhere: ./scripts/check-version.sh <expected-version>
#
# Used as a release gate (.github/workflows/release.yml) so a tag push whose
# manifests/lockfiles were not bumped — or were bumped without regenerating the
# lockfiles — fails the pipeline before any artifact is built. Mirror this list
# with bump-version.sh whenever the set of version-carrying files changes.
set -euo pipefail

EXPECTED="${1:?Usage: $0 <expected-version>  (e.g. 0.2.0)}"

# Validate semver-ish format (same rule as bump-version.sh).
if ! [[ "$EXPECTED" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9._-]+)?$ ]]; then
    echo "Error: '$EXPECTED' does not look like a valid version (expected X.Y.Z or X.Y.Z-label)"
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

mismatches=0

# Compares an extracted value against $EXPECTED, accumulating (not aborting on)
# each discrepancy so a single run reports every file that is out of sync.
check_field() {
    local label="$1" found="$2"
    if [[ "$found" != "$EXPECTED" ]]; then
        echo "  ✗ $label: expected '$EXPECTED', found '${found:-<empty>}'"
        mismatches=$((mismatches + 1))
    else
        echo "  ✓ $label ($found)"
    fi
}

# --- Cargo.toml [package] version: first `version = "..."` line ---
cargo_pkg_version() {
    grep -m1 '^version = ' "$1" | sed -E 's/.*["'\''"]([^"'\'']*)["'\''].*/\1/'
}

# --- JSON top-level "version" field: first occurrence ---
json_version() {
    grep -m1 '"version"' "$1" | sed -E 's/.*"version": *"([^"]*)".*/\1/'
}

# --- Cargo.lock: version that follows a given crate's `name = "..."` block ---
cargo_lock_version() {
    local crate="$1"
    grep -A1 "^name = \"$crate\"\$" Cargo.lock | grep '^version = ' | head -1 \
        | sed -E 's/.*"([^"]*)".*/\1/'
}

# --- package-lock.json: version of the `desktop-app` workspace package ---
# Parsed with node for robustness (the release runner always has Node).
package_lock_desktop_version() {
    node -p "require('./package-lock.json').packages['desktop-app'].version"
}

echo "Verifying all version fields match $EXPECTED …"

check_field "orchestrator-be/Cargo.toml"            "$(cargo_pkg_version orchestrator-be/Cargo.toml)"
check_field "desktop-app/src-tauri/Cargo.toml"      "$(cargo_pkg_version desktop-app/src-tauri/Cargo.toml)"
check_field "desktop-app/package.json"              "$(json_version desktop-app/package.json)"
check_field "desktop-app/src-tauri/tauri.conf.json" "$(json_version desktop-app/src-tauri/tauri.conf.json)"
check_field "Cargo.lock (orchestrator-be)"          "$(cargo_lock_version orchestrator-be)"
check_field "Cargo.lock (desktop-app)"              "$(cargo_lock_version desktop-app)"
check_field "package-lock.json (desktop-app)"       "$(package_lock_desktop_version)"

echo ""
if [[ "$mismatches" -gt 0 ]]; then
    echo "Error: $mismatches version field(s) do not match '$EXPECTED'."
    echo "Run ./scripts/bump-version.sh $EXPECTED and regenerate the lockfiles, then retry."
    exit 1
fi

echo "✓ All version fields match $EXPECTED."
